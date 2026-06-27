mod math;
mod matrix;

pub use math::{apply_rope, argmax, rms_norm};
pub use matrix::Matrix;

use ferrite_model::gguf::{GgmlType, GgufError, GgufFile, TensorInfo};
use math::{add_assign, dot, ensure_len, softmax, swiglu};
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
    pub output: Matrix,
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
        if tokens.is_empty() {
            return Err(InferenceError::new(
                "prompt must contain at least one token",
            ));
        }

        let mut layer_keys = vec![Vec::<Vec<f32>>::new(); self.weights.layers.len()];
        let mut layer_values = vec![Vec::<Vec<f32>>::new(); self.weights.layers.len()];
        let mut last_logits = Vec::new();

        for (position, token_id) in tokens.iter().enumerate() {
            if *token_id >= self.config.vocab_size {
                return Err(InferenceError::new(format!(
                    "token id {token_id} is out of bounds for vocab size {}",
                    self.config.vocab_size
                )));
            }

            let mut hidden = self.weights.token_embedding.row(*token_id)?.to_vec();

            for (layer_index, layer) in self.weights.layers.iter().enumerate() {
                let normed = rms_norm(&hidden, &layer.attn_norm, self.config.rms_norm_epsilon)?;
                let mut query = layer.q_proj.mul_vec(&normed)?;
                let mut key = layer.k_proj.mul_vec(&normed)?;
                let value = layer.v_proj.mul_vec(&normed)?;

                query =
                    self.apply_rope_to_heads(&query, position, self.config.attention_head_count)?;
                key =
                    self.apply_rope_to_heads(&key, position, self.config.attention_head_count_kv)?;

                layer_keys[layer_index].push(key);
                layer_values[layer_index].push(value);

                let attention = self.causal_attention(
                    &query,
                    &layer_keys[layer_index],
                    &layer_values[layer_index],
                )?;
                let attention_output = layer.o_proj.mul_vec(&attention)?;
                add_assign(&mut hidden, &attention_output)?;

                let ffn_normed = rms_norm(&hidden, &layer.ffn_norm, self.config.rms_norm_epsilon)?;
                let gate = layer.ffn_gate.mul_vec(&ffn_normed)?;
                let up = layer.ffn_up.mul_vec(&ffn_normed)?;
                let activated = swiglu(&gate, &up)?;
                let ffn_output = layer.ffn_down.mul_vec(&activated)?;
                add_assign(&mut hidden, &ffn_output)?;
            }

            let normed = rms_norm(
                &hidden,
                &self.weights.output_norm,
                self.config.rms_norm_epsilon,
            )?;
            last_logits = self.weights.output.mul_vec(&normed)?;
        }

        let token_id = argmax(&last_logits)?;
        Ok(NextToken {
            token_id,
            logits: last_logits,
        })
    }

    pub fn from_gguf_f32(file: &GgufFile, bytes: &[u8]) -> Result<Self, InferenceError> {
        Self::from_gguf_unquantized(file, bytes)
    }

    pub fn from_gguf_unquantized(file: &GgufFile, bytes: &[u8]) -> Result<Self, InferenceError> {
        let llama = file.llama_config()?;
        let hidden_size = usize_from_u64(llama.embedding_length, "llama.embedding_length")?;
        let intermediate_size =
            usize_from_u64(llama.feed_forward_length, "llama.feed_forward_length")?;
        let attention_head_count =
            usize_from_u64(llama.attention_head_count, "llama.attention.head_count")?;
        let attention_head_count_kv = usize_from_u64(
            llama.attention_head_count_kv,
            "llama.attention.head_count_kv",
        )?;
        let head_dim = usize_from_u64(llama.key_length, "llama.attention.key_length")?;
        let block_count = usize_from_u64(llama.block_count, "llama.block_count")?;

        if llama.key_length != llama.value_length {
            return Err(InferenceError::new(format!(
                "scalar GGUF loader requires key length {} to equal value length {}",
                llama.key_length, llama.value_length
            )));
        }

        let token_embedding_shape = required_tensor(file, "token_embd.weight")?
            .dimensions
            .clone();
        if token_embedding_shape.len() != 2 {
            return Err(InferenceError::new(format!(
                "token_embd.weight must be a matrix, found dimensions {token_embedding_shape:?}"
            )));
        }
        let vocab_size = usize_from_u64(token_embedding_shape[1], "token_embd.weight vocab")?;

        let config = ScalarLlamaConfig {
            vocab_size,
            hidden_size,
            intermediate_size,
            attention_head_count,
            attention_head_count_kv,
            head_dim,
            rope_dimension_count: usize_from_u64(
                llama.rope_dimension_count,
                "llama.rope.dimension_count",
            )?,
            rope_freq_base: llama.rope_freq_base.unwrap_or(10_000.0),
            rms_norm_epsilon: llama.attention_layer_norm_rms_epsilon.unwrap_or(0.0),
        };

        let mut layers = Vec::with_capacity(block_count);
        for layer_index in 0..block_count {
            layers.push(ScalarLlamaLayerWeights {
                attn_norm: f32_vector(
                    file,
                    bytes,
                    &format!("blk.{layer_index}.attn_norm.weight"),
                    hidden_size,
                )?,
                q_proj: f32_matrix(
                    file,
                    bytes,
                    &format!("blk.{layer_index}.attn_q.weight"),
                    hidden_size,
                    hidden_size,
                )?,
                k_proj: f32_matrix(
                    file,
                    bytes,
                    &format!("blk.{layer_index}.attn_k.weight"),
                    attention_head_count_kv * head_dim,
                    hidden_size,
                )?,
                v_proj: f32_matrix(
                    file,
                    bytes,
                    &format!("blk.{layer_index}.attn_v.weight"),
                    attention_head_count_kv * head_dim,
                    hidden_size,
                )?,
                o_proj: f32_matrix(
                    file,
                    bytes,
                    &format!("blk.{layer_index}.attn_output.weight"),
                    hidden_size,
                    hidden_size,
                )?,
                ffn_norm: f32_vector(
                    file,
                    bytes,
                    &format!("blk.{layer_index}.ffn_norm.weight"),
                    hidden_size,
                )?,
                ffn_gate: f32_matrix(
                    file,
                    bytes,
                    &format!("blk.{layer_index}.ffn_gate.weight"),
                    intermediate_size,
                    hidden_size,
                )?,
                ffn_up: f32_matrix(
                    file,
                    bytes,
                    &format!("blk.{layer_index}.ffn_up.weight"),
                    intermediate_size,
                    hidden_size,
                )?,
                ffn_down: f32_matrix(
                    file,
                    bytes,
                    &format!("blk.{layer_index}.ffn_down.weight"),
                    hidden_size,
                    intermediate_size,
                )?,
            });
        }

        Self::new(
            config,
            ScalarLlamaWeights {
                token_embedding: f32_matrix(
                    file,
                    bytes,
                    "token_embd.weight",
                    vocab_size,
                    hidden_size,
                )?,
                output_norm: f32_vector(file, bytes, "output_norm.weight", hidden_size)?,
                output: f32_matrix(file, bytes, "output.weight", vocab_size, hidden_size)?,
                layers,
            },
        )
    }

    fn causal_attention(
        &self,
        query: &[f32],
        keys_by_position: &[Vec<f32>],
        values_by_position: &[Vec<f32>],
    ) -> Result<Vec<f32>, InferenceError> {
        let expected_query = self.config.attention_head_count * self.config.head_dim;
        let expected_kv = self.config.attention_head_count_kv * self.config.head_dim;
        ensure_len("query", query, expected_query)?;
        if keys_by_position.len() != values_by_position.len() {
            return Err(InferenceError::new(format!(
                "key position count {} does not match value position count {}",
                keys_by_position.len(),
                values_by_position.len()
            )));
        }
        if keys_by_position.is_empty() {
            return Err(InferenceError::new("attention cache must not be empty"));
        }

        let heads_per_kv = self
            .config
            .attention_head_count
            .checked_div(self.config.attention_head_count_kv)
            .ok_or_else(|| InferenceError::new("invalid zero kv head count"))?;

        let mut output = vec![0.0; expected_query];
        for query_head in 0..self.config.attention_head_count {
            let kv_head = query_head / heads_per_kv;
            let query_start = query_head * self.config.head_dim;
            let kv_start = kv_head * self.config.head_dim;
            let query_slice = &query[query_start..query_start + self.config.head_dim];
            let mut scores = Vec::with_capacity(keys_by_position.len());

            for key in keys_by_position {
                ensure_len("cached key", key, expected_kv)?;
                let key_slice = &key[kv_start..kv_start + self.config.head_dim];
                scores.push(dot(query_slice, key_slice)? / (self.config.head_dim as f32).sqrt());
            }

            let weights = softmax(&scores)?;
            for (position, value) in values_by_position.iter().enumerate() {
                ensure_len("cached value", value, expected_kv)?;
                let value_slice = &value[kv_start..kv_start + self.config.head_dim];
                for dimension in 0..self.config.head_dim {
                    output[query_start + dimension] += weights[position] * value_slice[dimension];
                }
            }
        }

        Ok(output)
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
    ensure_matrix_shape(
        "output",
        &weights.output,
        config.vocab_size,
        config.hidden_size,
    )?;
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

fn required_tensor<'a>(file: &'a GgufFile, name: &str) -> Result<&'a TensorInfo, InferenceError> {
    file.tensor(name)
        .ok_or_else(|| InferenceError::new(format!("missing required tensor {name}")))
}

fn f32_matrix(
    file: &GgufFile,
    bytes: &[u8],
    name: &str,
    rows: usize,
    cols: usize,
) -> Result<Matrix, InferenceError> {
    let tensor = required_tensor(file, name)?;
    let expected_dimensions = vec![cols as u64, rows as u64];
    if tensor.dimensions != expected_dimensions {
        return Err(InferenceError::new(format!(
            "{name} dimensions {:?} do not match expected GGUF matrix dimensions {:?}",
            tensor.dimensions, expected_dimensions
        )));
    }

    Matrix::from_row_major(rows, cols, f32_values(tensor, bytes)?)
}

fn f32_vector(
    file: &GgufFile,
    bytes: &[u8],
    name: &str,
    len: usize,
) -> Result<Vec<f32>, InferenceError> {
    let tensor = required_tensor(file, name)?;
    let expected_dimensions = vec![len as u64];
    if tensor.dimensions != expected_dimensions {
        return Err(InferenceError::new(format!(
            "{name} dimensions {:?} do not match expected GGUF vector dimensions {:?}",
            tensor.dimensions, expected_dimensions
        )));
    }

    f32_values(tensor, bytes)
}

fn f32_values(tensor: &TensorInfo, bytes: &[u8]) -> Result<Vec<f32>, InferenceError> {
    let slice = bytes.get(tensor.data_range.clone()).ok_or_else(|| {
        InferenceError::new(format!("tensor {} byte range is invalid", tensor.name))
    })?;

    match tensor.ty {
        GgmlType::F32 => f32_values_from_le_bytes(&tensor.name, slice),
        GgmlType::F16 => f16_values_from_le_bytes(&tensor.name, slice),
        GgmlType::BF16 => bf16_values_from_le_bytes(&tensor.name, slice),
        other => Err(InferenceError::new(format!(
            "tensor {} has type {:?}; expected F32, F16, or BF16",
            tensor.name, other
        ))),
    }
}

fn f32_values_from_le_bytes(name: &str, slice: &[u8]) -> Result<Vec<f32>, InferenceError> {
    if !slice.len().is_multiple_of(4) {
        return Err(InferenceError::new(format!(
            "tensor {name} byte length {} is not divisible by 4",
            slice.len()
        )));
    }

    let mut values = Vec::with_capacity(slice.len() / 4);
    for chunk in slice.chunks_exact(4) {
        values.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(values)
}

fn f16_values_from_le_bytes(name: &str, slice: &[u8]) -> Result<Vec<f32>, InferenceError> {
    if !slice.len().is_multiple_of(2) {
        return Err(InferenceError::new(format!(
            "tensor {name} byte length {} is not divisible by 2",
            slice.len()
        )));
    }

    let mut values = Vec::with_capacity(slice.len() / 2);
    for chunk in slice.chunks_exact(2) {
        values.push(f16_bits_to_f32(u16::from_le_bytes([chunk[0], chunk[1]])));
    }
    Ok(values)
}

fn bf16_values_from_le_bytes(name: &str, slice: &[u8]) -> Result<Vec<f32>, InferenceError> {
    if !slice.len().is_multiple_of(2) {
        return Err(InferenceError::new(format!(
            "tensor {name} byte length {} is not divisible by 2",
            slice.len()
        )));
    }

    let mut values = Vec::with_capacity(slice.len() / 2);
    for chunk in slice.chunks_exact(2) {
        let bits = u32::from(u16::from_le_bytes([chunk[0], chunk[1]])) << 16;
        values.push(f32::from_bits(bits));
    }
    Ok(values)
}

fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = ((bits & 0x8000) as u32) << 16;
    let exponent = ((bits >> 10) & 0x1f) as u32;
    let mantissa = (bits & 0x03ff) as u32;

    let f32_bits = match exponent {
        0 => {
            if mantissa == 0 {
                sign
            } else {
                let mut normalized_mantissa = mantissa;
                let mut exponent_adjust = -14i32;
                while normalized_mantissa & 0x0400 == 0 {
                    normalized_mantissa <<= 1;
                    exponent_adjust -= 1;
                }
                normalized_mantissa &= 0x03ff;
                let exponent_bits = ((exponent_adjust + 127) as u32) << 23;
                sign | exponent_bits | (normalized_mantissa << 13)
            }
        }
        0x1f => sign | 0x7f80_0000 | (mantissa << 13),
        _ => {
            let exponent_bits = (exponent + 112) << 23;
            sign | exponent_bits | (mantissa << 13)
        }
    };

    f32::from_bits(f32_bits)
}

fn usize_from_u64(value: u64, name: &str) -> Result<usize, InferenceError> {
    usize::try_from(value).map_err(|_| InferenceError::new(format!("{name} does not fit in usize")))
}
