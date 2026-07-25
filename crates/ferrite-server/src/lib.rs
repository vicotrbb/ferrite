//! OpenAI-compatible local HTTP serving and operational evaluation tools.
//!
//! The crate owns request validation, bounded admission, streaming lifecycle,
//! cache coordination, continuous batching, and the helper clients used by the
//! real-model gates. Model parsing and execution remain in the lower-level
//! `ferrite-model` and `ferrite-inference` crates.
//!
//! This package does not publish a stable Rust library. Its public visibility
//! supports the package binaries and black-box integration tests. Applications
//! should integrate through the HTTP contract documented in `docs/openai-api.md`.
#![deny(missing_docs)]
#![deny(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::return_self_not_must_use
)]

#[doc(hidden)]
/// Process configuration for the Ferrite HTTP server.
pub mod config;
#[doc(hidden)]
mod diagnostic_hash;
#[doc(hidden)]
/// Request token limits and normalization.
pub mod limits;
#[doc(hidden)]
/// Long-context, cancellation, queue, and lifecycle validation tools.
pub mod long_chat_gate;
#[doc(hidden)]
/// OpenAI-compatible schemas, routes, prompt rendering, and streaming.
pub mod openai;
#[doc(hidden)]
/// Inference engine integration and continuous-batch scheduling.
pub mod runtime;
#[doc(hidden)]
/// Shared server state, authentication, admission, and cache configuration.
pub mod state;
#[doc(hidden)]
/// HTTP throughput client, streaming parsers, and resource summaries.
pub mod throughput_client;

use axum::Router;

/// Builds the complete HTTP router for a configured server state.
pub fn router(state: state::ServerState) -> Router {
    openai::routes::router(state)
}
