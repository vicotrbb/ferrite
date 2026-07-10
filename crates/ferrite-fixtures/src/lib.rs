//! In-memory GGUF fixtures used by Ferrite's deterministic tests.
//!
//! Fixtures are assembled from source-controlled values at test time. The
//! crate keeps binary model assets out of Git while exercising the real parser,
//! tokenizer, loader, and inference boundaries.
#![deny(missing_docs)]

mod chat_llama;
mod gguf_writer;
mod scalar_llama;
mod scalar_llama_tensors;

pub use chat_llama::{
    scalar_llama_chat_f32_gguf_fixture, scalar_llama_chat_f32_gguf_fixture_with_eos_token_id,
};
pub use scalar_llama::{
    scalar_llama_bf16_gguf_fixture, scalar_llama_f16_gguf_fixture, scalar_llama_f32_gguf_fixture,
    scalar_llama_f32_gguf_fixture_with_eos_token_id, scalar_llama_q4_k_gguf_fixture,
    scalar_llama_q5_0_gguf_fixture, scalar_llama_q6_k_gguf_fixture, scalar_llama_q8_0_gguf_fixture,
    scalar_llama_tied_output_f32_gguf_fixture,
};
