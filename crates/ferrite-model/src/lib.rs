//! GGUF parsing, model configuration, and tokenizer support for Ferrite.
//!
//! The crate validates GGUF v3 structure before exposing tensor byte ranges,
//! converts supported architecture metadata into typed configuration, and
//! provides atomic and BPE tokenization from GGUF tokenizer metadata.
//!
//! # Example
//!
//! ```
//! use ferrite_fixtures::scalar_llama_f32_gguf_fixture;
//! use ferrite_model::gguf::{ModelArchitecture, parse_gguf};
//!
//! let bytes = scalar_llama_f32_gguf_fixture();
//! let file = parse_gguf(&bytes)?;
//!
//! assert_eq!(file.architecture()?, ModelArchitecture::Llama);
//! # Ok::<(), ferrite_model::gguf::GgufError>(())
//! ```
#![deny(missing_docs)]
#![deny(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::return_self_not_must_use
)]

mod gguf_config;

/// Validated GGUF v3 metadata and tensor layout parsing.
pub mod gguf;
/// Read-only model-file mapping for shared GGUF tensor storage.
pub mod model_file;
/// Tokenization backed by metadata embedded in a parsed GGUF file.
pub mod tokenizer;
