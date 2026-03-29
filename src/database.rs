use crate::prelude::*;

use arrow_array::{Float32Array, RecordBatch, StringArray, UInt64Array};
use arrow_schema::{DataType, Field, Schema};
use atoman::futures::StreamExt;
use lancedb::{
    Connection,
    database::CreateTableMode,
    query::{ExecutableQuery, QueryBase},
};

static DATABASE: State<Database> = State::new();

/// A wrapper over the record data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record<T> {
    pub id: u64,
    pub data: T,
}

/// A wrapper over LanceDB for working with RAG and embeddings
#[derive(Default, Clone)]
pub struct Database {
    connection: Option<Arc<Connection>>,
}

impl Database {
    /// A helper method for obtaining the current connection
    pub async fn db() -> Result<Arc<Connection>> {
        DATABASE
            .get()
            .await
            .connection
            .clone()
            .ok_or_else(|| Error::DatabaseConnect.into())
    }

    /// Connects to the RAG database table
    pub async fn connect(path: impl AsRef<Path>) -> Result<()> {
        let path = app_data().join(path.as_ref());
        let uri = path.to_str().expect("Invalid path for DataBase");

        // create a directory if it doesn't exist:
        if !path.exists() {
            tokio::fs::create_dir_all(&path).await?;
        }

        let conn = lancedb::connect(uri).execute().await?;

        DATABASE
            .set(Self {
                connection: Some(Arc::new(conn)),
            })
            .await;
        Ok(())
    }

    /// Searching and reading similar data
    /// Returns a list of tuples (record ID, deserialized data)
    pub async fn read<T>(
        table_name: &str,
        vector: Vec<f32>,
        limit: usize,
        coef: f32,
    ) -> Result<Option<Vec<Record<T>>>>
    where
        T: serde::de::DeserializeOwned,
    {
        let db = Self::db().await?;
        let table_names = db.table_names().execute().await?;

        // if the table does not exist yet, return None:
        if !table_names.contains(&table_name.to_string()) {
            return Ok(None);
        }

        let table = db.open_table(table_name).execute().await?;
        let max_distance = (1.0f32 - coef).max(0.0f32);

        // vector search using LanceDB tools:
        let mut stream = table
            .query()
            .nearest_to(vector.as_slice())?
            .limit(limit)
            .execute()
            .await?;

        let mut results = Vec::new();

        while let Some(batch_result) = stream.next().await {
            let batch = batch_result?;

            // safely remove columns by name:
            let id_col = batch
                .column_by_name("id")
                .ok_or_else(|| fmt!("Column 'id' not found"))?
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| fmt!("Failed to downcast 'id' to UInt64Array"))?;

            let data_col = batch
                .column_by_name("data")
                .ok_or_else(|| fmt!("Column 'data' not found"))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| fmt!("Failed to downcast 'data' to StringArray"))?;

            let distance_col = batch
                .column_by_name("_distance")
                .map(|col| col.as_any().downcast_ref::<Float32Array>())
                .flatten();

            for i in 0..batch.num_rows() {
                // filtering by distance (similarity coefficient):
                if let Some(dist_arr) = distance_col {
                    if dist_arr.value(i) > max_distance {
                        continue;
                    }
                }

                let id = id_col.value(i);
                let json_str = data_col.value(i);
                let data: T = json::from_str(json_str)?;

                results.push(Record { id, data });
            }
        }

        if results.is_empty() {
            return Ok(None);
        }

        Ok(Some(results))
    }

    /// Writing any serializable data to a table
    pub async fn write<T>(table_name: &str, vector: Vec<f32>, data: T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let db = Self::db().await?;
        let vector_len = vector.len();

        // generating an unique id (Timestamp + Rand in the tail):
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // leave 42 bits for time (mask 0x3FFFFFFFFFF):
        let time_part = now_ms & 0x3FFFFFFFFFF;
        // generate 22 random bits (mask 0x3FFFFF):
        let rand_part = (rand::random::<u32>() & 0x3FFFFF) as u64;
        // shift the time to the left and merge it with the random:
        let id: u64 = (time_part << 22) | rand_part;

        // convert data to string:
        let json_string = json::to_string(&data)?;

        // preparing Arrow arrays:
        let id_array = Arc::new(UInt64Array::from(vec![id]));

        let float_array = Arc::new(Float32Array::from(vector));
        let item_field = Arc::new(Field::new("item", DataType::Float32, true));
        let vector_array = Arc::new(arrow_array::FixedSizeListArray::try_new(
            item_field,
            vector_len as i32,
            float_array as Arc<dyn arrow_array::Array>,
            None,
        )?);

        let data_array = Arc::new(StringArray::from(vec![json_string]));

        // we describe the scheme of the table:
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::UInt64, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    vector_len as i32,
                ),
                false,
            ),
            Field::new("data", DataType::Utf8, false),
        ]));

        // building RecordBatch:
        let batch = RecordBatch::try_new(
            schema,
            vec![
                id_array as Arc<dyn arrow_array::Array>,
                vector_array as Arc<dyn arrow_array::Array>,
                data_array as Arc<dyn arrow_array::Array>,
            ],
        )?;

        let batches = vec![batch];
        let table_names = db.table_names().execute().await?;

        // Если таблица есть — дополняем, если нет — создаем с нуля
        if table_names.contains(&table_name.to_string()) {
            let table = db.open_table(table_name).execute().await?;
            table.add(batches).execute().await?;
        } else {
            db.create_table(table_name, batches)
                .mode(CreateTableMode::Overwrite)
                .execute()
                .await?;
        }

        Ok(())
    }

    /// Deleting a point entry by its ID
    pub async fn delete(table_name: &str, id: u64) -> Result<()> {
        let db = Self::db().await?;
        let table_names = db.table_names().execute().await?;

        if !table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        let table = db.open_table(table_name).execute().await?;
        let predicate = fmt!("id = {}", id);

        table.delete(&predicate).await?;

        Ok(())
    }
}
