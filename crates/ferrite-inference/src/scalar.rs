mod attention;
mod dense16;
mod float;
mod kernel_check;
mod kernels;
mod kv_store;
mod loader;
mod math;
mod matrix;
mod matvec;
mod memory;
#[cfg(target_arch = "aarch64")]
mod neon_util;
mod options;
mod output;
mod profile;
mod prompt;
mod q4_k;
#[cfg(target_arch = "x86_64")]
mod q4_k_avx2;
#[cfg(target_arch = "aarch64")]
mod q4_k_neon;
#[allow(
    dead_code,
    reason = "scalar Q8_K reference path is used by architecture-specific kernels"
)]
mod q4_k_q8_k;
#[cfg(target_arch = "aarch64")]
#[allow(
    dead_code,
    reason = "NEON Q8_K reference path is retained for parity tests"
)]
mod q4_k_q8_k_neon;
#[cfg(target_arch = "aarch64")]
mod q4_k_q8_residual_i8mm;
#[cfg(target_arch = "aarch64")]
mod q4_k_q8_residual_neon;
mod q5_0;
#[cfg(target_arch = "x86_64")]
mod q5_0_avx2;
#[cfg(target_arch = "aarch64")]
mod q5_0_neon;
#[cfg(target_arch = "aarch64")]
mod q5_0_q8_residual_neon;
mod q5_k;
mod q6_k;
#[cfg(target_arch = "x86_64")]
mod q6_k_avx2;
#[cfg(target_arch = "aarch64")]
mod q6_k_neon;
#[allow(
    dead_code,
    reason = "scalar Q8_K reference path is used by architecture-specific kernels"
)]
mod q6_k_q8_k;
#[cfg(target_arch = "aarch64")]
#[allow(
    dead_code,
    reason = "NEON Q8_K reference path is retained for parity tests"
)]
mod q6_k_q8_k_neon;
#[cfg(target_arch = "aarch64")]
mod q6_k_q8_residual_i8mm;
mod q8_0;
#[cfg(target_arch = "x86_64")]
mod q8_0_avx2;
#[cfg(target_arch = "aarch64")]
mod q8_0_neon;
#[cfg(target_arch = "aarch64")]
mod q8_0_q8_residual_i8mm;
#[allow(
    dead_code,
    reason = "Q8_K helpers are architecture-dependent and used by parity tests"
)]
mod q8_k;
#[cfg(test)]
mod q8_k_reference_tests;
#[cfg(target_arch = "aarch64")]
mod q8_residual_activation;
mod quantized;
#[cfg(test)]
mod quantized_tests;
mod rope;
mod session;
mod tensor;
mod validation;

pub use kernels::{CpuKernelCapabilities, KernelProvider};
pub use math::{argmax, rms_norm};
pub use matrix::{Matrix, MatrixStorageKind};
pub use options::{
    KvBackend, Q8KActivationMatvecPolicy, Q8KActivationMatvecRole, ScalarExecutionOptions,
};
pub use output::ScalarLlamaOutputWeights;
pub use profile::{ProfiledNextToken, ProfiledTokenId, ScalarMatVecComparison, ScalarProfileEvent};
use rope::apply_rope_with_layout_in_place;
pub use rope::{RopeLayout, apply_rope};
pub use session::{
    PromptEvaluationControl, PromptEvaluationLocation, ScalarLlamaSession,
    ScalarLlamaSessionSnapshot, accept_token_contexts_batch, accept_token_ids_batch,
};

/// Architecture-neutral name for mutable scalar generation state.
pub type ScalarTransformerSession<'a> = ScalarLlamaSession<'a>;

/// Architecture-neutral name for an owned scalar KV-cache snapshot.
pub type ScalarTransformerSessionSnapshot = ScalarLlamaSessionSnapshot;

/// Architecture-neutral name for tied or untied output projection weights.
pub type ScalarTransformerOutputWeights = ScalarLlamaOutputWeights;

use ferrite_model::gguf::{GgufError, GgufFile};
use ferrite_model::model_file::MappedModelFile;
use ferrite_model::tokenizer::GgufTokenizer;
use math::ensure_len;
use std::fmt;
use validation::{validate_config, validate_weights};

#[derive(Clone, Debug, PartialEq)]
/// Dimensions and numeric policies for a loaded transformer model.
pub struct ScalarLlamaConfig {
    /// Number of token IDs accepted by the model.
    pub vocab_size: usize,
    /// Hidden-state width.
    pub hidden_size: usize,
    /// Feed-forward network width.
    pub intermediate_size: usize,
    /// Number of query attention heads.
    pub attention_head_count: usize,
    /// Number of key and value attention heads.
    pub attention_head_count_kv: usize,
    /// Width of each attention head.
    pub head_dim: usize,
    /// Number of dimensions transformed by rotary position encoding.
    pub rope_dimension_count: usize,
    /// Rotary position encoding frequency base.
    pub rope_freq_base: f32,
    /// Pairing layout used by rotary position encoding.
    pub rope_layout: RopeLayout,
    /// Epsilon added by RMS normalization.
    pub rms_norm_epsilon: f32,
}

/// Architecture-neutral name for the normalized scalar transformer config.
pub type ScalarTransformerConfig = ScalarLlamaConfig;

#[derive(Clone, Debug, PartialEq)]
/// All model weights required by the scalar transformer runtime.
pub struct ScalarLlamaWeights {
    /// Token embedding matrix.
    pub token_embedding: Matrix,
    /// Final RMS normalization weights.
    pub output_norm: Vec<f32>,
    /// Output projection policy and weights.
    pub output: ScalarLlamaOutputWeights,
    /// Transformer block weights in execution order.
    pub layers: Vec<ScalarLlamaLayerWeights>,
}

/// Architecture-neutral name for normalized scalar transformer weights.
pub type ScalarTransformerWeights = ScalarLlamaWeights;

#[derive(Clone, Debug, PartialEq)]
/// Weights and optional projection biases for one transformer block.
pub struct ScalarLlamaLayerWeights {
    /// Attention input RMS normalization weights.
    pub attn_norm: Vec<f32>,
    /// Query projection matrix.
    pub q_proj: Matrix,
    /// Optional query projection bias.
    pub q_bias: Option<Vec<f32>>,
    /// Key projection matrix.
    pub k_proj: Matrix,
    /// Optional key projection bias.
    pub k_bias: Option<Vec<f32>>,
    /// Value projection matrix.
    pub v_proj: Matrix,
    /// Optional value projection bias.
    pub v_bias: Option<Vec<f32>>,
    /// Attention output projection matrix.
    pub o_proj: Matrix,
    /// Feed-forward input RMS normalization weights.
    pub ffn_norm: Vec<f32>,
    /// Gated feed-forward gate projection matrix.
    pub ffn_gate: Matrix,
    /// Gated feed-forward up projection matrix.
    pub ffn_up: Matrix,
    /// Feed-forward down projection matrix.
    pub ffn_down: Matrix,
}

/// Architecture-neutral name for one normalized transformer layer.
pub type ScalarTransformerLayerWeights = ScalarLlamaLayerWeights;

#[derive(Clone, Debug, PartialEq)]
/// An immutable transformer model prepared for CPU inference.
pub struct ScalarLlamaModel {
    config: ScalarLlamaConfig,
    weights: ScalarLlamaWeights,
    context_length: Option<usize>,
}

/// Architecture-neutral name for a CPU transformer model.
///
/// The historical `ScalarLlamaModel` name remains available for source
/// compatibility. GGUF loader adapters normalize Llama, Qwen2, and Phi-3
/// tensor layouts before constructing this shared execution model.
pub type ScalarTransformerModel = ScalarLlamaModel;

impl ScalarLlamaModel {
    /// Validates and constructs a model from typed configuration and weights.
    ///
    /// # Errors
    ///
    /// Returns an error when dimensions, head layout, numeric parameters, or
    /// any matrix and vector shape is inconsistent.
    pub fn new(
        config: ScalarLlamaConfig,
        weights: ScalarLlamaWeights,
    ) -> Result<Self, InferenceError> {
        validate_config(&config)?;
        validate_weights(&config, &weights)?;
        Ok(Self {
            config,
            weights,
            context_length: None,
        })
    }

    /// Evaluates one token as a one-token prompt and returns the next token.
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-range token or an inference shape or
    /// numeric failure.
    pub fn next_token(&self, token_id: usize) -> Result<NextToken, InferenceError> {
        self.next_token_for_prompt(&[token_id])
    }

    /// Evaluates a nonempty token prompt and returns the next token.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty prompt, an out-of-range token, or an
    /// inference shape or numeric failure.
    pub fn next_token_for_prompt(&self, tokens: &[usize]) -> Result<NextToken, InferenceError> {
        self.start_session().accept_prompt(tokens)
    }

    /// Creates an empty generation session with default execution options.
    pub fn start_session(&self) -> ScalarLlamaSession<'_> {
        ScalarLlamaSession::new(self)
    }

    /// Creates an empty generation session with explicit execution options.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested KV backend or execution policy is
    /// unavailable or invalid for this build and model.
    pub fn start_session_with_options(
        &self,
        options: ScalarExecutionOptions,
    ) -> Result<ScalarLlamaSession<'_>, InferenceError> {
        ScalarLlamaSession::new_with_options(self, options)
    }

    /// Returns the GGUF-declared context length when the model was loaded from
    /// an artifact. Models constructed from typed test weights have no limit.
    pub fn context_length(&self) -> Option<usize> {
        self.context_length
    }

    /// Returns the byte count of physical model tensor storage.
    ///
    /// Shared mapped tensors are counted by their tensor ranges, not by the
    /// complete mapped file size.
    pub fn scalar_weight_bytes(&self) -> u128 {
        memory::weights_bytes(&self.weights)
    }

    /// Returns the size of the shared GGUF mapping retained by quantized weights.
    pub fn mapped_model_file_bytes(&self) -> usize {
        memory::mapped_file_bytes(&self.weights)
    }

    /// Tokenizes a text prompt and returns the next token.
    ///
    /// # Errors
    ///
    /// Returns an error when tokenization fails or inference rejects the
    /// resulting prompt.
    pub fn next_token_for_text_prompt(
        &self,
        tokenizer: &GgufTokenizer,
        prompt: &str,
    ) -> Result<NextToken, InferenceError> {
        let tokens = prompt::encode_text_prompt(tokenizer, prompt)?;
        self.next_token_for_prompt(&tokens)
    }

    /// Loads a model through the scalar GGUF loader.
    ///
    /// This compatibility name accepts every tensor encoding supported by the
    /// scalar loader, not only F32 tensors.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata, tensor layout, storage encoding, or
    /// model validation fails.
    pub fn from_gguf_f32(file: &GgufFile, bytes: &[u8]) -> Result<Self, InferenceError> {
        Self::from_gguf_scalar(file, bytes)
    }

    /// Loads a model through the scalar GGUF loader.
    ///
    /// This compatibility name also accepts supported quantized matrices.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata, tensor layout, storage encoding, or
    /// model validation fails.
    pub fn from_gguf_unquantized(file: &GgufFile, bytes: &[u8]) -> Result<Self, InferenceError> {
        Self::from_gguf_scalar(file, bytes)
    }

    /// Loads all supported scalar and quantized weights from GGUF bytes.
    ///
    /// The supplied bytes must be the same complete artifact parsed into
    /// `file`, because tensor ranges index directly into this slice.
    ///
    /// # Errors
    ///
    /// Returns an error when model configuration, tensor presence, shape,
    /// storage encoding, byte ranges, or numeric validation fails.
    pub fn from_gguf_scalar(file: &GgufFile, bytes: &[u8]) -> Result<Self, InferenceError> {
        let (config, weights) = loader::load_scalar(file, bytes)?;
        Self::new(config, weights)?.with_gguf_context_length(file)
    }

    /// Loads supported scalar and quantized weights from a mapped GGUF file.
    ///
    /// Quantized matrices retain shared read-only ranges of `mapped` instead
    /// of copying their bytes into separate heap allocations. Dense tensors
    /// are still decoded into owned F32 storage.
    ///
    /// # Errors
    ///
    /// Returns an error when model configuration, tensor presence, shape,
    /// storage encoding, byte ranges, or numeric validation fails.
    pub fn from_gguf_mapped(
        file: &GgufFile,
        mapped: &MappedModelFile,
    ) -> Result<Self, InferenceError> {
        let (config, weights) = loader::load_scalar_mapped(file, mapped)?;
        Self::new(config, weights)?.with_gguf_context_length(file)
    }

    fn with_gguf_context_length(mut self, file: &GgufFile) -> Result<Self, InferenceError> {
        let model = file.model_config()?.into_transformer();
        self.context_length =
            Some(usize::try_from(model.context_length).map_err(|_error| {
                InferenceError::new("model context length does not fit in usize")
            })?);
        Ok(self)
    }

    fn apply_rope_to_heads_in_place(
        &self,
        values: &mut [f32],
        position: usize,
        head_count: usize,
    ) -> Result<(), InferenceError> {
        if self.config.rope_dimension_count == 0 {
            return Ok(());
        }

        let expected = head_count
            .checked_mul(self.config.head_dim)
            .ok_or_else(|| InferenceError::new("rope head width overflow"))?;
        ensure_len("rope input", values, expected)?;

        for head in 0..head_count {
            let start = head * self.config.head_dim;
            let end = start + self.config.head_dim;
            apply_rope_with_layout_in_place(
                &mut values[start..end],
                position,
                self.config.rope_dimension_count,
                self.config.rope_freq_base,
                self.config.rope_layout,
            )?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
/// The selected next token together with the logits used to select it.
pub struct NextToken {
    /// The highest-logit token ID.
    pub token_id: usize,
    /// Vocabulary logits in token-ID order.
    pub logits: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// An error produced while loading or executing a model.
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
