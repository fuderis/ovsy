use crate::prelude::*;
use anylm::{Completions, Embeddings, Proxy};
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

/// Creates ai-completions request
pub async fn completions() -> Result<Completions> {
    let lm = Settings::get().completions.clone();
    let mut request = Completions::new(
        lm.api_kind,
        if let Some(v) = lm.env_var.as_ref() {
            var(v).unwrap_or_default()
        } else {
            str!()
        },
        lm.model,
    )
    .max_tokens(lm.max_tokens)
    .temperature(lm.temperature);

    if let Some(host) = lm.server.as_ref() {
        request.set_server(host);
    }
    if let Some(proxy) = lm.proxy.as_ref() {
        request.set_proxy(Proxy::all(proxy)?);
    }

    Ok(request)
}

/// Creates ai-embeddings request
pub async fn embeddings() -> Result<Embeddings> {
    let lm = Settings::get().embeddings.clone();
    let mut request = Embeddings::new(
        lm.api_kind,
        if let Some(v) = lm.env_var.as_ref() {
            var(v).unwrap_or_default()
        } else {
            str!()
        },
        lm.model,
    );

    if let Some(host) = lm.server.as_ref() {
        request.set_server(host);
    }
    if let Some(proxy) = lm.proxy.as_ref() {
        request.set_proxy(Proxy::all(proxy)?);
    }

    Ok(request)
}
