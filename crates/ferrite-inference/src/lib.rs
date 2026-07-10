//! Ferrite's CPU inference runtime.
//!
//! The crate loads supported GGUF transformer weights, executes deterministic
//! scalar and architecture-optimized kernels, manages generation sessions and
//! KV state, and exposes prefix-cache identity and metadata primitives.
#![deny(missing_docs)]

/// Token-level identity and budgeted metadata for prefix caching.
pub mod prefix_cache;
/// Model loading, tensor kernels, sessions, profiling, and generation output.
pub mod scalar;
/// Process-wide inference worker-pool configuration.
pub mod threading;
