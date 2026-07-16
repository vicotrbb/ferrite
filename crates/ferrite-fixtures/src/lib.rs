//! In-memory GGUF fixtures used by Ferrite's deterministic tests.
//!
//! Fixtures are assembled from source-controlled values at test time. The
//! crate keeps binary model assets out of Git while exercising the real parser,
//! tokenizer, loader, and inference boundaries.
#![deny(missing_docs)]
#![deny(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::return_self_not_must_use
)]

mod chat_llama;
mod chat_templates;
mod gguf_writer;
mod scalar_llama;
mod scalar_llama_tensors;
mod scalar_phi3;

pub use chat_llama::{
    scalar_llama_chat_f32_gguf_fixture, scalar_llama_chat_f32_gguf_fixture_with_context_length,
    scalar_llama_chat_f32_gguf_fixture_with_eos_token_id,
};
pub use chat_templates::{
    LLAMA2_INSTRUCT_CHAT_TEMPLATE, LLAMA3_INSTRUCT_CHAT_TEMPLATE, QWEN2_5_INSTRUCT_CHAT_TEMPLATE,
    SMOLLM2_INSTRUCT_CHAT_TEMPLATE,
};
pub use scalar_llama::{
    scalar_llama_bf16_gguf_fixture, scalar_llama_f16_gguf_fixture, scalar_llama_f32_gguf_fixture,
    scalar_llama_f32_gguf_fixture_with_context_length,
    scalar_llama_f32_gguf_fixture_with_eos_token_id,
    scalar_llama_f32_gguf_fixture_with_eot_token_id, scalar_llama_q4_k_gguf_fixture,
    scalar_llama_q5_0_gguf_fixture, scalar_llama_q6_k_gguf_fixture, scalar_llama_q8_0_gguf_fixture,
    scalar_llama_tied_output_f32_gguf_fixture,
};
pub use scalar_phi3::{PHI3_INSTRUCT_CHAT_TEMPLATE, scalar_phi3_f32_gguf_fixture};
