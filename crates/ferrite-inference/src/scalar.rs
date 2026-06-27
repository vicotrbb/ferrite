mod attention;
mod kernel_check;
mod loader;
mod math;
mod matrix;
mod memory;
mod output;
mod prompt;
mod quantized;
mod rope;
mod session;
mod tensor;

pub use math::{argmax, rms_norm};
pub use matrix::Matrix;
pub use output::ScalarLlamaOutputWeights;
pub use rope::apply_rope;
pub use session::ScalarLlamaSession;

use ferrite_model::gguf::{GgufError, GgufFile};
use ferrite_model::tokenizer::GgufTokenizer;
use math::ensure_len;
use std::fmt;

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
    pub k_proj: Matrix,
    pub v_proj: Matrix,
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
            output.extend(apply_rope(
                &values[start..end],
                position,
                self.config.rope_dimension_count,
                self.config.rope_freq_base,
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

fn validate_config(config: &ScalarLlamaConfig) -> Result<(), InferenceError> {
    if config.vocab_size == 0
        || config.hidden_size == 0
        || config.intermediate_size == 0
        || config.attention_head_count == 0
        || config.attention_head_count_kv == 0
        || config.head_dim == 0
    {
        return Err(InferenceError::new(
            "scalar llama config dimensions must be non-zero",
        ));
    }

    let attention_width = config
        .attention_head_count
        .checked_mul(config.head_dim)
        .ok_or_else(|| InferenceError::new("attention width overflow"))?;
    if attention_width != config.hidden_size {
        return Err(InferenceError::new(format!(
            "attention heads {} * head dim {} must equal hidden size {}",
            config.attention_head_count, config.head_dim, config.hidden_size
        )));
    }

    if !config
        .attention_head_count
        .is_multiple_of(config.attention_head_count_kv)
    {
        return Err(InferenceError::new(format!(
            "attention head count {} must be divisible by kv head count {}",
            config.attention_head_count, config.attention_head_count_kv
        )));
    }

    if config.rope_dimension_count > config.head_dim {
        return Err(InferenceError::new(format!(
            "rope dimension count {} must not exceed head dim {}",
            config.rope_dimension_count, config.head_dim
        )));
    }
    if !config.rope_dimension_count.is_multiple_of(2) {
        return Err(InferenceError::new(format!(
            "rope dimension count {} must be even",
            config.rope_dimension_count
        )));
    }
    if config.rope_freq_base <= 0.0 {
        return Err(InferenceError::new(format!(
            "rope frequency base {} must be positive",
            config.rope_freq_base
        )));
    }

    Ok(())
}

fn validate_weights(
    config: &ScalarLlamaConfig,
    weights: &ScalarLlamaWeights,
) -> Result<(), InferenceError> {
    ensure_matrix_shape(
        "token_embedding",
        &weights.token_embedding,
        config.vocab_size,
        config.hidden_size,
    )?;
    if let Some(output) = weights.output.untied_matrix() {
        ensure_matrix_shape("output", output, config.vocab_size, config.hidden_size)?;
    }
    ensure_len("output_norm", &weights.output_norm, config.hidden_size)?;

    let kv_width = config
        .attention_head_count_kv
        .checked_mul(config.head_dim)
        .ok_or_else(|| InferenceError::new("kv width overflow"))?;

    for (index, layer) in weights.layers.iter().enumerate() {
        let prefix = format!("layer {index}");
        ensure_len(
            &format!("{prefix} attn_norm"),
            &layer.attn_norm,
            config.hidden_size,
        )?;
        ensure_len(
            &format!("{prefix} ffn_norm"),
            &layer.ffn_norm,
            config.hidden_size,
        )?;
        ensure_matrix_shape(
            &format!("{prefix} q_proj"),
            &layer.q_proj,
            config.hidden_size,
            config.hidden_size,
        )?;
        ensure_matrix_shape(
            &format!("{prefix} k_proj"),
            &layer.k_proj,
            kv_width,
            config.hidden_size,
        )?;
        ensure_matrix_shape(
            &format!("{prefix} v_proj"),
            &layer.v_proj,
            kv_width,
            config.hidden_size,
        )?;
        ensure_matrix_shape(
            &format!("{prefix} o_proj"),
            &layer.o_proj,
            config.hidden_size,
            config.hidden_size,
        )?;
        ensure_matrix_shape(
            &format!("{prefix} ffn_gate"),
            &layer.ffn_gate,
            config.intermediate_size,
            config.hidden_size,
        )?;
        ensure_matrix_shape(
            &format!("{prefix} ffn_up"),
            &layer.ffn_up,
            config.intermediate_size,
            config.hidden_size,
        )?;
        ensure_matrix_shape(
            &format!("{prefix} ffn_down"),
            &layer.ffn_down,
            config.hidden_size,
            config.intermediate_size,
        )?;
    }

    Ok(())
}

fn ensure_matrix_shape(
    name: &str,
    matrix: &Matrix,
    rows: usize,
    cols: usize,
) -> Result<(), InferenceError> {
    if matrix.rows() == rows && matrix.cols() == cols {
        Ok(())
    } else {
        Err(InferenceError::new(format!(
            "{name} shape {}x{} does not match expected {rows}x{cols}",
            matrix.rows(),
            matrix.cols()
        )))
    }
}
