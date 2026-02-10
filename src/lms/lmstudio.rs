use crate::prelude::*;
use lm_studio_api::{Chat, Content, Context, Message, Messages, Model, Role, SystemInfo};

/// The system prompt
struct SystemPrompt;

impl SystemInfo for SystemPrompt {
    fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn update(&mut self) -> String {
        let dt = Local::now();

        fmt!(
            r##"
            # Actual system info:
            * datetime: {dt}.
        "##
        )
    }
}

/// Handles query to LLM
pub async fn handle_query<S>(prompt: S, query: &str, options: (String, u32, f32)) -> Result<String>
where
    S: Into<String>,
{
    let (model, context, temperature) = options;
    let port = Settings::get().lmstudio.port;

    // init chat:
    let mut chat = Chat::new(
        Model::Other(model),
        Context::new(SystemPrompt::new(), context),
        port,
    );

    // generating request:
    let request = Messages {
        messages: vec![
            Message {
                role: Role::System,
                content: vec![Content::Text {
                    text: prompt.into(),
                }],
            },
            Message {
                role: Role::User,
                content: vec![Content::Text { text: query.into() }],
            },
        ],
        context: false,
        stream: true,
        /*
        format: Some(Format::json(
            "response",
            vec![Schema::object(
                "tool call",
                "",
                map! {
                    "tool": Schema::string("tool name", None),
                    "data": Schema::object("tool data", "", map! {}),
                    "query": Schema::string("next task query", None),
                },
            )],
            false,
        )),
        */
        temperature,
        ..Default::default()
    };

    // sending request:
    let _ = chat.send(request.into()).await?;

    // reading pre-results:
    let mut response = str!("");
    while let Some(result) = chat.next().await {
        match result {
            Ok(r) => {
                if let Some(text) = r.text() {
                    response.push_str(text);
                }
            }
            Err(e) => error!("{e}"),
        }
    }

    Ok(response)
}
