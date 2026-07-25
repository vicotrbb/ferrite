//! Ferrite's CPU inference runtime.
//!
//! The crate loads supported GGUF transformer weights, executes deterministic
//! scalar and architecture-optimized kernels, manages generation sessions and
//! KV state, and exposes prefix-cache identity and metadata primitives.
//!
//! # Example
//!
//! ```
//! use ferrite_fixtures::scalar_llama_f32_gguf_fixture;
//! use ferrite_inference::scalar::ScalarTransformerModel;
//! use ferrite_model::gguf::parse_gguf;
//!
//! let bytes = scalar_llama_f32_gguf_fixture();
//! let file = parse_gguf(&bytes)?;
//! let model = ScalarTransformerModel::from_gguf_scalar(&file, &bytes)?;
//! let next = model.next_token(0)?;
//!
//! assert_eq!(next.token_id, 2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![deny(missing_docs)]
#![deny(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::return_self_not_must_use
)]

/// Token-level identity and budgeted metadata for prefix caching.
pub mod prefix_cache;
/// Per-request greedy and probabilistic token sampling.
pub mod sampling;
/// Model loading, tensor kernels, sessions, profiling, and generation output.
pub mod scalar;
/// Process-wide inference worker-pool configuration.
pub mod threading;
