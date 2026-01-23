use crate::{LMKind, lms, prelude::*};
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
type ToolCall = (String, HashMap<String, JsonValue>);

/// Handles user query
async fn handle_query(query: String) -> Result<()> {
    let cfg = Settings::read()?;
    info!("⏳ Handle query '{:.100}'..", query.replace("\n", "\\n"));

    // read prompt:
    let mut prompt_dir = path!("$/prompt");
    if !prompt_dir.exists() {
        prompt_dir = path!("$/../../prompt");
    }
    let prompt = tfs::read_to_string(prompt_dir.join("handle-query.md")).await?;
    let prompt = prompt.replace("{DOCS}", &Tools::docs().await.join("\n\n"));

    // handle query by LLM:
    let query = fmt!("\n## User query (handle it):\n{query}");
    let json = match &cfg.lms.slm {
        LMKind::LMStudio => {
            let small = Settings::get().lmstudio.small.clone();
            lms::lmstudio::handle_query(prompt, &query, small).await?
        }
    };

    // trim code block:
    let re = re!(r#"^\s*```(?:\S+\b)?|\n```\s*$"#);
    let json = re.replace_all(&json, "").trim().to_string();
    let calls: Vec<ToolCall> =
        json::from_str(&json).map_err(|e| fmt!("Invalid LM response format: {e}"))?;

    // handle tool calls:
    for (tool_name, tool_data) in calls {
        // parse tool call:
        let mut spl = tool_name.splitn(2, "/");
        let name = spl
            .next()
            .ok_or(Error::InvalidToolNameFormat(tool_name.clone()))?
            .to_owned();
        let action = spl
            .next()
            .ok_or(Error::InvalidToolNameFormat(tool_name.clone()))?
            .to_owned();
        let data = json::to_value(&tool_data)?;

        // do tool call:
        super::tool::handle_tool(name, action, data).await?;
    }

    Ok(())
}
