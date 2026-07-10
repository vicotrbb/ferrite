use super::{
    tensor, InferenceError, Matrix, ScalarLlamaConfig, ScalarLlamaLayerWeights,
    ScalarLlamaOutputWeights, ScalarLlamaWeights,
};
use ferrite_model::gguf::{GgmlType, GgufFile, ModelArchitecture, ModelConfig, TensorInfo};

pub(super) fn load_scalar(
    file: &GgufFile,
    bytes: &[u8],
) -> Result<(ScalarLlamaConfig, ScalarLlamaWeights), InferenceError> {
    let model = match file.model_config()? {
        ModelConfig::Llama(config) | ModelConfig::Qwen2(config) => config,
    };
    let hidden_size = usize_from_u64(model.embedding_length, "model.embedding_length")?;
    let intermediate_size = usize_from_u64(model.feed_forward_length, "model.feed_forward_length")?;
    let attention_head_count =
        usize_from_u64(model.attention_head_count, "model.attention.head_count")?;
    let attention_head_count_kv = usize_from_u64(
        model.attention_head_count_kv,
        "model.attention.head_count_kv",
    )?;
    let head_dim = usize_from_u64(model.key_length, "model.attention.key_length")?;
    let block_count = usize_from_u64(model.block_count, "model.block_count")?;

    if model.key_length != model.value_length {
        return Err(InferenceError::new(format!(
            "scalar GGUF loader requires key length {} to equal value length {}",
            model.key_length, model.value_length
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
            model.rope_dimension_count,
            "model.rope.dimension_count",
        )?,
        rope_freq_base: model.rope_freq_base.unwrap_or(10_000.0),
        rope_layout: match model.architecture {
            ModelArchitecture::Llama => super::RopeLayout::AdjacentPairs,
            ModelArchitecture::Qwen2 => super::RopeLayout::SplitHalf,
        },
        rms_norm_epsilon: model.attention_layer_norm_rms_epsilon.unwrap_or(0.0),
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
            q_bias: optional_f32_vector(
                file,
                bytes,
                &format!("blk.{layer_index}.attn_q.bias"),
                hidden_size,
            )?,
            k_proj: f32_matrix(
                file,
                bytes,
                &format!("blk.{layer_index}.attn_k.weight"),
                attention_head_count_kv * head_dim,
                hidden_size,
            )?,
            k_bias: optional_f32_vector(
                file,
                bytes,
                &format!("blk.{layer_index}.attn_k.bias"),
                attention_head_count_kv * head_dim,
            )?,
            v_proj: f32_matrix(
                file,
                bytes,
                &format!("blk.{layer_index}.attn_v.weight"),
                attention_head_count_kv * head_dim,
                hidden_size,
            )?,
            v_bias: optional_f32_vector(
                file,
                bytes,
                &format!("blk.{layer_index}.attn_v.bias"),
                attention_head_count_kv * head_dim,
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

    let token_embedding = f32_matrix(file, bytes, "token_embd.weight", vocab_size, hidden_size)?;
    let output = output_matrix_or_tied(file, bytes, &token_embedding, vocab_size, hidden_size)?;

    Ok((
        config,
        ScalarLlamaWeights {
            token_embedding,
            output_norm: f32_vector(file, bytes, "output_norm.weight", hidden_size)?,
            output,
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

    match tensor.ty {
        GgmlType::Q4K => {
            Matrix::from_q4_k_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        GgmlType::Q5_0 => {
            Matrix::from_q5_0_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        GgmlType::Q6K => {
            Matrix::from_q6_k_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        GgmlType::Q8_0 => {
            Matrix::from_q8_0_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        _ => Matrix::from_row_major(rows, cols, tensor::f32_values(tensor, bytes)?),
    }
}

fn output_matrix_or_tied(
    file: &GgufFile,
    bytes: &[u8],
    token_embedding: &Matrix,
    rows: usize,
    cols: usize,
) -> Result<ScalarLlamaOutputWeights, InferenceError> {
    if file.tensor("output.weight").is_some() {
        Ok(ScalarLlamaOutputWeights::untied(f32_matrix(
            file,
            bytes,
            "output.weight",
            rows,
            cols,
        )?))
    } else if token_embedding.rows() == rows && token_embedding.cols() == cols {
        Ok(ScalarLlamaOutputWeights::tied_token_embedding())
    } else {
        Err(InferenceError::new(format!(
            "token_embd.weight shape {}x{} cannot be reused as output.weight shape {rows}x{cols}",
            token_embedding.rows(),
            token_embedding.cols()
        )))
    }
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

    tensor::f32_values(tensor, bytes)
}

fn optional_f32_vector(
    file: &GgufFile,
    bytes: &[u8],
    name: &str,
    len: usize,
) -> Result<Option<Vec<f32>>, InferenceError> {
    if file.tensor(name).is_none() {
        return Ok(None);
    }

    f32_vector(file, bytes, name, len).map(Some)
}

fn usize_from_u64(value: u64, name: &str) -> Result<usize, InferenceError> {
    usize::try_from(value)
        .map_err(|_error| InferenceError::new(format!("{name} does not fit in usize")))
}
