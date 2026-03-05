use crate::prelude::*;
use anylm::{Completions, Proxy};
use std::env::var;
use tokio::fs;

/// Reads prompt file
pub async fn read_prompt(name: &str) -> Result<String> {
    let dir = app_data().join("prompts");
    let file = dir.join(fmt!("{name}.md"));

    if !file.exists() {
        fs::create_dir_all(&dir).await?;
        fs::write(
            &file,
            fs::read(path!("$/../../default/prompts/{name}.md")).await?,
        )
        .await?;
    }

    Ok(fs::read_to_string(file).await?)
}

/// Creates AI completions request
pub async fn completions() -> Result<Completions> {
    let lm = Settings::get().lms.clone();
    let mut request = Completions::new(lm.api_kind, var(&lm.env_var).unwrap_or_default(), lm.model)
        .max_tokens(lm.max_tokens)
        .temperature(lm.temperature);

    if !lm.server.trim().is_empty() {
        request.set_server(lm.server);
    }
    if !lm.proxy.trim().is_empty() {
        request.set_proxy(Proxy::all(&lm.proxy)?);
    }

    Ok(request)
}
