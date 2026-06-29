pub mod config;
pub mod limits;
pub mod openai;
pub mod runtime;
pub mod state;
pub mod throughput_client;

use axum::Router;

pub fn router(state: state::ServerState) -> Router {
    openai::routes::router(state)
}
