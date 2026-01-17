use root::prelude::*;
use reqwest::Client;

#[derive(Serialize)]
struct Request {
    author: String,
}

#[tokio::main]
async fn main() -> Result<()> {   
    let client = Client::new();
    let request = Request {
        author: "disturbed".to_string(),
    };

    let response = client
        .post("http://localhost:3030/play")
        .json(&request)
        .send()
        .await?;

    assert_eq!(response.status(), 200);
    
    Ok(())
}
