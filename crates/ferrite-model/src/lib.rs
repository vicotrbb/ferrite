//! GGUF parsing, model configuration, and tokenizer support for Ferrite.
//!
//! The crate validates GGUF v3 structure before exposing tensor byte ranges,
//! converts supported architecture metadata into typed configuration, and
//! provides atomic and BPE tokenization from GGUF tokenizer metadata.
#![deny(missing_docs)]
#![deny(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::return_self_not_must_use
)]

mod gguf_config;

/// Validated GGUF v3 metadata and tensor layout parsing.
pub mod gguf;
/// Tokenization backed by metadata embedded in a parsed GGUF file.
pub mod tokenizer;
