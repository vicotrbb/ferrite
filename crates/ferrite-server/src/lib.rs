//! OpenAI-compatible local HTTP serving and operational evaluation tools.
//!
//! The crate owns request validation, bounded admission, streaming lifecycle,
//! cache coordination, continuous batching, and the helper clients used by the
//! real-model gates. Model parsing and execution remain in the lower-level
//! `ferrite-model` and `ferrite-inference` crates.

/// Process configuration for the Ferrite HTTP server.
pub mod config;
mod diagnostic_hash;
/// Request token limits and normalization.
pub mod limits;
/// Long-context, cancellation, queue, and lifecycle validation tools.
pub mod long_chat_gate;
/// OpenAI-compatible schemas, routes, prompt rendering, and streaming.
pub mod openai;
/// Inference engine integration and continuous-batch scheduling.
pub mod runtime;
/// Shared server state, authentication, admission, and cache configuration.
pub mod state;
/// HTTP throughput client, streaming parsers, and resource summaries.
pub mod throughput_client;

use axum::Router;

/// Builds the complete HTTP router for a configured server state.
pub fn router(state: state::ServerState) -> Router {
    openai::routes::router(state)
}
