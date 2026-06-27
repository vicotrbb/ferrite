use super::{
    InferenceError, Matrix, ScalarLlamaConfig, ScalarLlamaLayerWeights, ScalarLlamaWeights,
};
use ferrite_model::gguf::{GgmlType, GgufFile, TensorInfo};

pub(super) fn load_unquantized(
    file: &GgufFile,
    bytes: &[u8],
) -> Result<(ScalarLlamaConfig, ScalarLlamaWeights), InferenceError> {
    let llama = file.llama_config()?;
    let hidden_size = usize_from_u64(llama.embedding_length, "llama.embedding_length")?;
    let intermediate_size = usize_from_u64(llama.feed_forward_length, "llama.feed_forward_length")?;
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

    Ok((
        config,
        ScalarLlamaWeights {
            token_embedding: f32_matrix(file, bytes, "token_embd.weight", vocab_size, hidden_size)?,
            output_norm: f32_vector(file, bytes, "output_norm.weight", hidden_size)?,
            output: f32_matrix(file, bytes, "output.weight", vocab_size, hidden_size)?,
            layers,
        },
    ))
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
