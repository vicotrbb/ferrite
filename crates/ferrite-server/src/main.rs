use ferrite_server::{config::ServerConfig, state::ServerState};
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
    let app = ferrite_server::router(ServerState::new(config.model_id().to_owned()));
    axum::serve(listener, app).await?;
    Ok(())
}
