pub mod config;
pub mod openai;
pub mod runtime;
pub mod state;

use axum::Router;

pub fn router(state: state::ServerState) -> Router {
    openai::routes::router(state)
}
