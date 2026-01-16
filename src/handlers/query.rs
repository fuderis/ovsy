use crate::{ prelude::*, llm };
use tokio::fs as tfs;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    query: String,
}

/// Api '/query' handler
pub async fn handle(Json(data): Json<QueryData>) -> Json<JsonValue> {
    match handle_query(data.query).await {
        Ok(_) => Json(json!({ "status": 200 })),
        Err(e) => {
            err!("{e}");
            Json(json!({ "status": 500, "error": fmt!("{e}") }))
        }
    }
}

/// The LLM tool call data
#[derive(Debug, Deserialize)]
struct ToolCall {
    name: String,
    data: HashMap<String, JsonValue>,
}

/// Handles user query
async fn handle_query(query: String) -> Result<()> {
    let cfg = Settings::read()?;
    info!("⏳ Handle query '{:.100}'..", query.replace("\n", "\\n"));

    // read prompt:
    let mut prompt_dir = path!("$/prompt");
    if !prompt_dir.exists() { prompt_dir = path!("$/../../../prompt"); }
    let prompt = tfs::read_to_string(prompt_dir.join("handle-query.md")).await?;
    let prompt = prompt.replace("{DOCS}", &Tools::docs().await.join("\n\n"));

    // handle query by LLM:
    let json = llm::handle_query(prompt, &cfg.llm.model, cfg.llm.context, &fmt!("\n## User query (handle it):\n{query}")).await?;

    // trim code block:
    let re = re!(r#"^\s*```(?:\S+\b)?|\n```\s*$"#);
    let json = re.replace_all(&json, "").trim().to_string();
    let calls: Vec<ToolCall> = json::from_str(&json).map_err(|e| fmt!("Invalid LLM response format: {e}"))?;

    // handle tool calls:
    for call in calls {
        // parse tool:
        let (name, action, data) = {
            let tool_name = &call.name;
            let mut spl = tool_name.splitn(2, "/");
    
            let name = spl.next().ok_or(Error::InvalidToolNameFormat(call.name.clone()))?;
            let action = spl.next().ok_or(Error::InvalidToolNameFormat(call.name.clone()))?;
            let data = json::to_value(&call.data)?;
    
            (name.to_owned(), action.to_owned(), data)
        };
    
        // do tool call:
        if let Err(e) = super::tool::handle_tool(name, action, data).await {
            return Err(e);
        }
    }

    Ok(())
}
