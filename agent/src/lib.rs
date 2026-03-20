pub use macron;

pub mod error;
pub use error::{Result, StdResult};
pub mod prelude;

pub mod chunk;
pub use chunk::SessionChunk;

use crate::prelude::*;
use axum::{
    Json, Router,
    routing::{get, post},
};
use clap::Parser;
use serde_json::{Value, json};
use std::net::SocketAddr;
use tokio::net::TcpListener;

/// Стандартные аргументы для всех агентов
#[derive(Parser, Debug)]
pub struct AgentArgs {
    #[arg(short, long)]
    pub port: u16,
}

pub struct AgentServer {
    router: Router,
}

impl AgentServer {
    pub fn new() -> Self {
        // Базовый роутер с дефолтными эндпоинтами
        let router = Router::new()
            .route("/", get(|| async { Html("") }))
            .route("/health", post(Self::health_handler));

        Self { router }
    }

    /// Добавляет POST маршрут (стандарт для вызова действий агента)
    pub fn route<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, ()>,
        T: 'static,
    {
        self.router = self.router.route(path, post(handler));
        self
    }

    /// Стандартный обработчик здоровья
    async fn health_handler() -> Json<Value> {
        Json(json!({
            "log_file": Logger::get_path()
        }))
    }

    /// Запуск сервера с автоматическим парсингом порта
    pub async fn run(self) -> Result<()> {
        // Инициализация логгера (если еще не сделана)
        // Logger::init(app_data().join("logs"), 20).ok();

        let args = AgentArgs::parse();
        let addr = SocketAddr::from(([127, 0, 0, 1], args.port));

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            error!("Agent could not bind to port {}: {}", args.port, e);
            e
        })?;

        info!("🚀 Agent is running on http://{}", addr);
        axum::serve(listener, self.router).await?;

        Ok(())
    }
}
