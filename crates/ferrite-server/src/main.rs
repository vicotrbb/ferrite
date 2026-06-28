use ferrite_server::{config::ServerConfig, runtime::InferenceEngine, state::ServerState};
use std::error::Error;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::parse(std::env::args_os())?;
    let listener = tokio::net::TcpListener::bind(config.bind_addr()).await?;
    let state = match config.model_path() {
        Some(path) => {
            let engine = InferenceEngine::load(path)?;
            ServerState::with_engine(config.model_id().to_owned(), engine)
        }
        None => ServerState::new(config.model_id().to_owned()),
    };
    let state = match config.api_key() {
        Some(api_key) => state.with_api_key(api_key),
        None => state,
    }
    .with_token_limits(config.token_limits());
    let state = state.with_inference_wait_timeout(config.inference_wait_timeout());
    let app = ferrite_server::router(state);
    axum::serve(listener, app).await?;
    Ok(())
}
