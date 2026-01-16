use root::prelude::*;
use reqwest::Client;
use clap::Parser;

/// The launch arguments
#[derive(Parser)]
struct LaunchArgs {
    #[arg(short, long)]
    query: Option<String>,
}

#[derive(Serialize)]
struct Request {
    query: String,
}

#[tokio::main]
async fn main() -> Result<()> {   
    let args = LaunchArgs::parse();
    let client = Client::new();
    let request = Request {
        query: args.query.unwrap_or(str!("play geoxor")),
    };

    let response = client
        .post("http://localhost:7878/query")
        .json(&request)
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    
    Ok(())
}
