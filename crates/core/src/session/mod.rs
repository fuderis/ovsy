use crate::prelude::*;
use anylm::Message;
use cistern::{Cistern, Kv};
use ovsy_share::SessionID;

/// The session table key
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Key {
    Metadata,
    Message(usize),
}

/// The session metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub session_id: SessionID,
    pub message_count: u64,
    pub compressed_until: usize,
}

impl Metadata {
    /// Creates a new session metadata by session id
    pub fn new(session_id: SessionID) -> Self {
        Self {
            session_id,
            message_count: 0,
            compressed_until: 0,
        }
    }
}

/// The user session manager
#[derive(Clone)]
pub struct Session {
    pub id: SessionID,
    pub db: Arc<Cistern<Kv>>,
}

impl Session {
    /// Creates a new user session instance
    pub async fn new(id: SessionID) -> Result<Self> {
        let dir = app_data().join(str!("db/{}/sessions/{id}", id.user_id));
        let db = arc!(Cistern::connect(dir).await?);

        Ok(Self { id, db })
    }

    /// Reads the session metadata
    pub async fn read_metadata(&self) -> Result<Option<Metadata>> {
        let table_name = Self::table_name(&self.id);
        let table = self.db.open_table(&table_name).await?;

        table.read(Key::Metadata).await
    }

    /// Reads all the session messages
    pub async fn read_messages(&self) -> Result<Vec<Message>> {
        let table_name = Self::table_name(&self.id);
        let table = self.db.open_table(&table_name).await?;

        // read metadata or create a new one:
        let meta = match table.read(Key::Metadata).await? {
            Some(meta) => meta,
            None => {
                let new_meta = Metadata::new(self.id);
                table.write(Key::Metadata, new_meta.clone()).await?;
                table.flush().await?;
                new_meta
            }
        };

        // read messages by indexes:
        let start_idx = meta.compressed_until;
        let end_idx = meta.message_count as usize;

        let mut messages = Vec::with_capacity(end_idx.saturating_sub(start_idx));
        for i in start_idx..end_idx {
            let msg_key = Key::Message(i);
            if let Some(msg) = table.read::<_, Message>(msg_key).await? {
                messages.push(msg);
            }
        }

        Ok(messages)
    }

    /// Writes a new message to the session
    pub async fn write_message(&self, message: Message) -> Result<()> {
        let table_name = Self::table_name(&self.id);
        let table = self.db.open_table(&table_name).await?;

        // read metadata:
        let mut meta: Metadata = table
            .read(Key::Metadata)
            .await?
            .unwrap_or(Metadata::new(self.id));

        // gen key for new message:
        let msg_key = Key::Message(meta.message_count as usize);
        table.write(msg_key, message).await?;

        // update messages count:
        meta.message_count += 1;
        table.write(Key::Metadata, meta).await?;

        // force flush buffer to disk:
        table.flush().await?;

        Ok(())
    }

    /// Writes a new messages to the session
    pub async fn write_messages(&self, messages: Vec<Message>) -> Result<()> {
        let table_name = Self::table_name(&self.id);
        let table = self.db.open_table(&table_name).await?;

        // read metadata:
        let mut meta: Metadata = table
            .read(Key::Metadata)
            .await?
            .unwrap_or(Metadata::new(self.id));

        for message in messages {
            // gen key for new message:
            let msg_key = Key::Message(meta.message_count as usize);
            table.write(msg_key, message).await?;
            meta.message_count += 1;
        }

        // update messages count:
        table.write(Key::Metadata, meta).await?;

        // force flush buffer to disk:
        table.flush().await?;

        Ok(())
    }

    /// Inserts a message after the compressed originals and shifts the preserve messages
    pub async fn insert_and_shift(
        &self,
        compressed_msg: Message,
        preserve_msgs: Vec<Message>,
        compress_count: usize,
    ) -> Result<()> {
        let table_name = Self::table_name(&self.id);
        let table = self.db.open_table(&table_name).await?;

        let current_meta = table
            .read::<_, Metadata>(Key::Metadata)
            .await?
            .unwrap_or(Metadata::new(self.id));

        let insert_idx = current_meta.compressed_until + compress_count;
        let mut current_idx = insert_idx;

        table
            .write(Key::Message(current_idx), compressed_msg)
            .await?;
        current_idx += 1;

        for msg in preserve_msgs {
            table.write(Key::Message(current_idx), msg).await?;
            current_idx += 1;
        }

        let new_message_count = std::cmp::max(current_meta.message_count, current_idx as u64);
        let new_meta = Metadata {
            session_id: self.id,
            message_count: new_message_count,
            compressed_until: insert_idx,
        };

        table.write(Key::Metadata, new_meta).await?;
        table.flush().await?;

        Ok(())
    }

    /// Completely clears the session message history
    pub async fn clear(&self) -> Result<()> {
        let table_name = Self::table_name(&self.id);
        let table = self.db.open_table(&table_name).await?;

        // remove all messages:
        if let Some(meta) = table.read::<_, Metadata>(Key::Metadata).await? {
            let start_idx = meta.compressed_until;
            let end_idx = meta.message_count as usize;

            for i in start_idx..end_idx {
                table.remove(Key::Message(i)).await?;
            }
        }

        // write new metadata:
        let fresh_meta = Metadata::new(self.id);
        table.write(Key::Metadata, fresh_meta).await?;
        table.flush().await?;

        Ok(())
    }

    /// Creates the table name by session id
    fn table_name(session_id: &SessionID) -> String {
        str!("{session_id}")
    }
}
