#![allow(dead_code)]

pub mod http;
pub mod openai_client;
pub mod stop_sequences;
pub mod throughput;

use ferrite_server::{runtime::InferenceEngine, state::ServerState};
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{net::TcpListener, task::JoinHandle};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);
pub const MODEL_ID: &str = "fixture-model";

pub struct LiveServer {
    addr: SocketAddr,
    model_path_to_remove: Option<PathBuf>,
    server: JoinHandle<Result<(), std::io::Error>>,
}

impl LiveServer {
    pub async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        Self::start_with_state(|state| state).await
    }

    pub async fn start_with_model_id(model_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        Self::start_with_loaded_model(model_id, &model_path, Some(model_path.clone()), |state| {
            state
        })
        .await
    }

    pub async fn start_configured(
        configure: impl FnOnce(ServerState) -> ServerState,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::start_with_state(configure).await
    }

    pub async fn start_with_api_key(api_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::start_with_state(|state| state.with_api_key(api_key)).await
    }

    pub async fn start_with_existing_model(
        model_id: &str,
        model_path: impl Into<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model_path = model_path.into();
        Self::start_with_loaded_model(model_id, &model_path, None, |state| state).await
    }

    pub async fn start_with_existing_model_configured(
        model_id: &str,
        model_path: impl Into<PathBuf>,
        configure: impl FnOnce(ServerState) -> ServerState,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model_path = model_path.into();
        Self::start_with_loaded_model(model_id, &model_path, None, configure).await
    }

    async fn start_with_state(
        configure: impl FnOnce(ServerState) -> ServerState,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        Self::start_with_loaded_model(MODEL_ID, &model_path, Some(model_path.clone()), configure)
            .await
    }

    async fn start_with_loaded_model(
        model_id: &str,
        model_path: &std::path::Path,
        model_path_to_remove: Option<PathBuf>,
        configure: impl FnOnce(ServerState) -> ServerState,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let engine = InferenceEngine::load(model_path)?;
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let addr = listener.local_addr()?;
        let state = configure(ServerState::with_engine(model_id.to_owned(), engine));
        let app = ferrite_server::router(state);
        let server = tokio::spawn(async move { axum::serve(listener, app).await });

        Ok(Self {
            addr,
            model_path_to_remove,
            server,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Drop for LiveServer {
    fn drop(&mut self) {
        self.server.abort();
        if let Some(model_path) = &self.model_path_to_remove {
            let _ = std::fs::remove_file(model_path);
        }
    }
}

fn write_fixture_model() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ferrite-server-http-fixture-{}-{}.gguf",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(
        &path,
        ferrite_fixtures::scalar_llama_chat_f32_gguf_fixture(),
    )?;
    Ok(path)
}
