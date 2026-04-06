use crate::prelude::*;
use anylm::{Completions, Embeddings, Proxy};
use std::env::var;
use tokio::fs;

/// Reads prompt file
pub async fn read_prompt(name: &str) -> Result<String> {
    let dir = app_data().join("prompts");
    let mut file = dir.join(str!("{name}.md"));

    // if not exists - use default prompt:
    if !file.exists() {
        #[cfg(debug_assertions)]
        {
            file = path!("$/../../default/prompts/{name}.md");
        }
        #[cfg(not(debug_assertions))]
        {
            file = path!("$/default/prompts/{name}.md");
        }
    };

    Ok(fs::read_to_string(file).await?)
}

/// Creates AI-completions request
pub async fn completions() -> Result<Completions> {
    let cfg = Settings::get().completions.clone();
    let mut request = Completions::new(
        // choose AI service
        cfg.api_kind,
        // read API key
        if let Some(v) = cfg.env_var.as_ref() {
            var(v).unwrap_or_default()
        } else {
            str!()
        },
        // choose model
        cfg.model,
    )
    .max_tokens(cfg.max_tokens)
    .temperature(cfg.temperature);

    // set default server host:
    if let Some(host) = cfg.server.as_ref() {
        request.set_server(host);
    }
    // set proxy options:
    if let Some(proxy) = cfg.proxy.as_ref() {
        request.set_proxy(Proxy::all(proxy)?);
    }

    Ok(request)
}

/// Creates ai-embeddings request
pub async fn embeddings() -> Result<Embeddings> {
    let cfg = Settings::get().embeddings.clone();
    let mut request = Embeddings::new(
        // choose AI service
        cfg.api_kind,
        // read API key
        if let Some(v) = cfg.env_var.as_ref() {
            var(v).unwrap_or_default()
        } else {
            str!()
        },
        // choose model
        cfg.model,
    );

    // set default server host:
    if let Some(host) = cfg.server.as_ref() {
        request.set_server(host);
    }
    // set proxy options:
    if let Some(proxy) = cfg.proxy.as_ref() {
        request.set_proxy(Proxy::all(proxy)?);
    }

    Ok(request)
}

/// Converts text to embeddings vector
pub async fn to_embeddings(s: impl Into<String>) -> Result<Option<Vec<f32>>> {
    let response = embeddings().await?.input(s).send().await?;
    let data = response.data;

    // read first vector (because we only sent 1 line):
    let v = if let Some(first) = data.into_iter().next() {
        Some(first.embedding)
    } else {
        None
    };

    Ok(v)
}
