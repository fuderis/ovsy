use crate::prelude::*;
use lm_studio_api::{
    Chat, Content, Context, Format, Message, Messages, Model, Role, Schema, SystemInfo,
};

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
pub async fn handle_query<S, S2>(prompt: S, query: &str, model: S2, context: u32) -> Result<String>
where
    S: Into<String>,
    S2: Into<String>,
{
    let cfg = &Settings::get().lmstudio;

    /*
    // check lmstudio port:
    let addr = fmt!("127.0.0.1:{}", cfg.port);
    if TcpStream::connect(&addr).await.is_err() {
        warn!("Running LM Studio server..");

        // create run command:
        let mut cmd = Command::new(&cfg.exec);
        // cmd.args(["server", "start", "--port", &cfg.port.to_string()]);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        // cmd.env("DISPLAY", ":0");
        // cmd.env("XDG_RUNTIME_DIR", "/run/user/$(id -u)");
        cmd.kill_on_drop(false);

        // spawn process child:
        if let Err(e) = cmd.spawn() {
            err!("Failed to start LM Studio: {e}");
        }
    }
    */

    // init chat:
    let mut chat = Chat::new(
        Model::Other(model.into()),
        Context::new(SystemPrompt::new(), context),
        cfg.port,
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
        format: Some(Format::json(
            "handlers",
            vec![Schema::array("handlers-list", "handler calls list")],
            false,
        )),
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
            Err(e) => err!("{e}"),
        }
    }

    Ok(response)
}
