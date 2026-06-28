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
    model_path: PathBuf,
    server: JoinHandle<Result<(), std::io::Error>>,
}

impl LiveServer {
    pub async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let addr = listener.local_addr()?;
        let app = ferrite_server::router(ServerState::with_engine(MODEL_ID.to_owned(), engine));
        let server = tokio::spawn(async move { axum::serve(listener, app).await });

        Ok(Self {
            addr,
            model_path,
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
        let _ = std::fs::remove_file(&self.model_path);
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
