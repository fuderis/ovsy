use crate::prelude::*;
use lm_studio_api::{ SystemInfo, Chat, Model, Context, Messages, Message, Role, Content, Format, Schema };

/// The system prompt
struct SystemPrompt;

impl SystemInfo for SystemPrompt {
    fn new() -> Box<Self> {
        Box::new(Self {})
    }
    
    fn update(&mut self) -> String {
        let dt = Local::now();
        
        fmt!(r##"
            # Actual system info:
            * datetime: {dt}.
        "##)
    }
}

/// Handles query to LLM
pub async fn handle_query<S, S2>(prompt: S, query: &str, model: S2, context: u32, port: u16) -> Result<String>
where
    S: Into<String>,
    S2: Into<String>,
{
    // init chat:
    let mut chat = Chat::new(
        Model::Other(model.into()),
        Context::new(SystemPrompt::new(), context),
        port,
    );

    // generating request:
    let request = Messages {
        messages: vec![
            Message {
                role: Role::System,
                content: vec![
                    Content::Text { text: prompt.into() },
                ]
            },
            Message {
                role: Role::User,
                content: vec![
                    Content::Text { text: query.into() },
                ]
            }
        ],
        context: false,
        stream: true,
        format: Some(Format::json(
            "handlers",
            vec![
                Schema::array(
                    "handlers-list",
                    "handler calls list"
                ),
            ],
            false
        )),
        ..Default::default()
    };
    
    // sending request:
    let _ = chat.send(request.into()).await?;

    // reading pre-results:
    let mut response = str!("");
    while let Some(result) = chat.next().await {
        match result {
            Ok(r) => if let Some(text) = r.text() { response.push_str(text); },
            Err(e) => err!("{e}"),
        }
    }

    Ok(response)
}
