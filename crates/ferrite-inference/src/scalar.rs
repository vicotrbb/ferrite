mod attention;
mod float;
mod kernel_check;
mod loader;
mod math;
mod matrix;
mod matvec;
mod memory;
mod output;
mod profile;
mod prompt;
mod q4_k;
#[cfg(target_arch = "x86_64")]
mod q4_k_avx2;
#[cfg(target_arch = "aarch64")]
mod q4_k_neon;
#[allow(dead_code)]
mod q4_k_q8_k;
mod q5_0;
#[cfg(target_arch = "x86_64")]
mod q5_0_avx2;
#[cfg(target_arch = "aarch64")]
mod q5_0_neon;
mod q6_k;
#[cfg(target_arch = "x86_64")]
mod q6_k_avx2;
#[cfg(target_arch = "aarch64")]
mod q6_k_neon;
#[allow(dead_code)]
mod q6_k_q8_k;
mod q8_0;
#[cfg(target_arch = "x86_64")]
mod q8_0_avx2;
#[cfg(target_arch = "aarch64")]
mod q8_0_neon;
#[allow(dead_code)]
mod q8_k;
mod quantized;
#[cfg(test)]
mod quantized_tests;
mod rope;
mod session;
mod tensor;
mod validation;

pub use math::{argmax, rms_norm};
pub use matrix::{Matrix, MatrixStorageKind};
pub use output::ScalarLlamaOutputWeights;
pub use profile::{ProfiledNextToken, ProfiledTokenId, ScalarProfileEvent};
use rope::apply_rope_with_layout;
pub use rope::{apply_rope, RopeLayout};
pub use session::ScalarLlamaSession;

use ferrite_model::gguf::{GgufError, GgufFile};
use ferrite_model::tokenizer::GgufTokenizer;
use math::ensure_len;
use std::fmt;
use validation::{validate_config, validate_weights};

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarLlamaConfig {
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub attention_head_count: usize,
    pub attention_head_count_kv: usize,
    pub head_dim: usize,
    pub rope_dimension_count: usize,
    pub rope_freq_base: f32,
    pub rope_layout: RopeLayout,
    pub rms_norm_epsilon: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarLlamaWeights {
    pub token_embedding: Matrix,
    pub output_norm: Vec<f32>,
    pub output: ScalarLlamaOutputWeights,
    pub layers: Vec<ScalarLlamaLayerWeights>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarLlamaLayerWeights {
    pub attn_norm: Vec<f32>,
    pub q_proj: Matrix,
    pub q_bias: Option<Vec<f32>>,
    pub k_proj: Matrix,
    pub k_bias: Option<Vec<f32>>,
    pub v_proj: Matrix,
    pub v_bias: Option<Vec<f32>>,
    pub o_proj: Matrix,
    pub ffn_norm: Vec<f32>,
    pub ffn_gate: Matrix,
    pub ffn_up: Matrix,
    pub ffn_down: Matrix,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarLlamaModel {
    config: ScalarLlamaConfig,
    weights: ScalarLlamaWeights,
}

impl ScalarLlamaModel {
    pub fn new(
        config: ScalarLlamaConfig,
        weights: ScalarLlamaWeights,
    ) -> Result<Self, InferenceError> {
        validate_config(&config)?;
        validate_weights(&config, &weights)?;
        Ok(Self { config, weights })
    }

    pub fn next_token(&self, token_id: usize) -> Result<NextToken, InferenceError> {
        self.next_token_for_prompt(&[token_id])
    }

    pub fn next_token_for_prompt(&self, tokens: &[usize]) -> Result<NextToken, InferenceError> {
        self.start_session().accept_prompt(tokens)
    }

    pub fn start_session(&self) -> ScalarLlamaSession<'_> {
        ScalarLlamaSession::new(self)
    }

    pub fn scalar_weight_bytes(&self) -> u128 {
        memory::weights_bytes(&self.weights)
    }

    pub fn next_token_for_text_prompt(
        &self,
        tokenizer: &GgufTokenizer,
        prompt: &str,
    ) -> Result<NextToken, InferenceError> {
        let tokens = prompt::encode_text_prompt(tokenizer, prompt)?;
        self.next_token_for_prompt(&tokens)
    }

    pub fn from_gguf_f32(file: &GgufFile, bytes: &[u8]) -> Result<Self, InferenceError> {
        Self::from_gguf_scalar(file, bytes)
    }

    pub fn from_gguf_unquantized(file: &GgufFile, bytes: &[u8]) -> Result<Self, InferenceError> {
        Self::from_gguf_scalar(file, bytes)
    }

    pub fn from_gguf_scalar(file: &GgufFile, bytes: &[u8]) -> Result<Self, InferenceError> {
        let (config, weights) = loader::load_scalar(file, bytes)?;
        Self::new(config, weights)
    }

    fn apply_rope_to_heads(
        &self,
        values: &[f32],
        position: usize,
        head_count: usize,
    ) -> Result<Vec<f32>, InferenceError> {
        if self.config.rope_dimension_count == 0 {
            return Ok(values.to_vec());
        }

        let expected = head_count
            .checked_mul(self.config.head_dim)
            .ok_or_else(|| InferenceError::new("rope head width overflow"))?;
        ensure_len("rope input", values, expected)?;

        let mut output = Vec::with_capacity(values.len());
        for head in 0..head_count {
            let start = head * self.config.head_dim;
            let end = start + self.config.head_dim;
            output.extend(apply_rope_with_layout(
                &values[start..end],
                position,
                self.config.rope_dimension_count,
                self.config.rope_freq_base,
                self.config.rope_layout,
            )?);
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NextToken {
    pub token_id: usize,
    pub logits: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceError {
    message: String,
}

impl InferenceError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for InferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for InferenceError {}

impl From<GgufError> for InferenceError {
    fn from(error: GgufError) -> Self {
        Self::new(error.to_string())
    }
}
