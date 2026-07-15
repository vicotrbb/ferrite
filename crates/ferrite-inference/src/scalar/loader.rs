use super::{
    tensor, InferenceError, Matrix, ScalarLlamaConfig, ScalarLlamaLayerWeights,
    ScalarLlamaOutputWeights, ScalarLlamaWeights,
};
use ferrite_model::gguf::{
    AttentionProjectionLayout, FeedForwardProjectionLayout, GgmlType, GgufFile, RotaryPairing,
    TensorInfo,
};
use ferrite_model::model_file::MappedModelFile;

#[derive(Clone, Copy)]
enum LoaderSource<'a> {
    Bytes(&'a [u8]),
    Mapped(&'a MappedModelFile),
}

impl<'a> LoaderSource<'a> {
    fn as_bytes(self) -> &'a [u8] {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::Mapped(file) => file.as_bytes(),
        }
    }
}

pub(super) fn load_scalar(
    file: &GgufFile,
    bytes: &[u8],
) -> Result<(ScalarLlamaConfig, ScalarLlamaWeights), InferenceError> {
    load_scalar_from_source(file, LoaderSource::Bytes(bytes))
}

pub(super) fn load_scalar_mapped(
    file: &GgufFile,
    mapped: &MappedModelFile,
) -> Result<(ScalarLlamaConfig, ScalarLlamaWeights), InferenceError> {
    load_scalar_from_source(file, LoaderSource::Mapped(mapped))
}

fn load_scalar_from_source(
    file: &GgufFile,
    source: LoaderSource<'_>,
) -> Result<(ScalarLlamaConfig, ScalarLlamaWeights), InferenceError> {
    let model_config = file.model_config()?;
    let model = model_config.transformer();
    let execution = model.architecture.execution();
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
        rope_layout: match execution.rotary_pairing {
            RotaryPairing::Adjacent => super::RopeLayout::AdjacentPairs,
            RotaryPairing::SplitHalf => super::RopeLayout::SplitHalf,
        },
        rms_norm_epsilon: model.attention_layer_norm_rms_epsilon.unwrap_or(0.0),
    };

    let mut layers = Vec::with_capacity(block_count);
    for layer_index in 0..block_count {
        let (q_proj, q_bias, k_proj, k_bias, v_proj, v_bias) = match execution.attention {
            AttentionProjectionLayout::Separate => (
                f32_matrix(
                    file,
                    source,
                    &format!("blk.{layer_index}.attn_q.weight"),
                    hidden_size,
                    hidden_size,
                )?,
                optional_f32_vector(
                    file,
                    source,
                    &format!("blk.{layer_index}.attn_q.bias"),
                    hidden_size,
                )?,
                f32_matrix(
                    file,
                    source,
                    &format!("blk.{layer_index}.attn_k.weight"),
                    attention_head_count_kv * head_dim,
                    hidden_size,
                )?,
                optional_f32_vector(
                    file,
                    source,
                    &format!("blk.{layer_index}.attn_k.bias"),
                    attention_head_count_kv * head_dim,
                )?,
                f32_matrix(
                    file,
                    source,
                    &format!("blk.{layer_index}.attn_v.weight"),
                    attention_head_count_kv * head_dim,
                    hidden_size,
                )?,
                optional_f32_vector(
                    file,
                    source,
                    &format!("blk.{layer_index}.attn_v.bias"),
                    attention_head_count_kv * head_dim,
                )?,
            ),
            AttentionProjectionLayout::FusedQkv => {
                let kv_width = attention_head_count_kv
                    .checked_mul(head_dim)
                    .ok_or_else(|| {
                        InferenceError::new("model fused QKV key/value width overflow")
                    })?;
                let fused_rows =
                    hidden_size
                        .checked_add(kv_width.checked_mul(2).ok_or_else(|| {
                            InferenceError::new("model fused QKV row count overflow")
                        })?)
                        .ok_or_else(|| InferenceError::new("model fused QKV row count overflow"))?;
                let name = format!("blk.{layer_index}.attn_qkv.weight");
                let fused = f32_matrix(file, source, &name, fused_rows, hidden_size)?;
                let q_proj = fused.row_range(0..hidden_size)?;
                let k_proj = fused.row_range(hidden_size..hidden_size + kv_width)?;
                let v_proj = fused.row_range(hidden_size + kv_width..fused_rows)?;
                let (q_bias, k_bias, v_bias) = match optional_f32_vector(
                    file,
                    source,
                    &format!("blk.{layer_index}.attn_qkv.bias"),
                    fused_rows,
                )? {
                    Some(fused_bias) => (
                        Some(fused_bias[0..hidden_size].to_vec()),
                        Some(fused_bias[hidden_size..hidden_size + kv_width].to_vec()),
                        Some(fused_bias[hidden_size + kv_width..].to_vec()),
                    ),
                    None => (None, None, None),
                };
                (q_proj, q_bias, k_proj, k_bias, v_proj, v_bias)
            }
        };
        let (ffn_gate, ffn_up) = match execution.feed_forward {
            FeedForwardProjectionLayout::Separate => (
                f32_matrix(
                    file,
                    source,
                    &format!("blk.{layer_index}.ffn_gate.weight"),
                    intermediate_size,
                    hidden_size,
                )?,
                f32_matrix(
                    file,
                    source,
                    &format!("blk.{layer_index}.ffn_up.weight"),
                    intermediate_size,
                    hidden_size,
                )?,
            ),
            FeedForwardProjectionLayout::FusedGateUp => {
                let fused_rows = intermediate_size
                    .checked_mul(2)
                    .ok_or_else(|| InferenceError::new("model fused gate/up row count overflow"))?;
                let fused = f32_matrix(
                    file,
                    source,
                    &format!("blk.{layer_index}.ffn_up.weight"),
                    fused_rows,
                    hidden_size,
                )?;
                (
                    fused.row_range(0..intermediate_size)?,
                    fused.row_range(intermediate_size..fused_rows)?,
                )
            }
        };
        layers.push(ScalarLlamaLayerWeights {
            attn_norm: f32_vector(
                file,
                source,
                &format!("blk.{layer_index}.attn_norm.weight"),
                hidden_size,
            )?,
            q_proj,
            q_bias,
            k_proj,
            k_bias,
            v_proj,
            v_bias,
            o_proj: f32_matrix(
                file,
                source,
                &format!("blk.{layer_index}.attn_output.weight"),
                hidden_size,
                hidden_size,
            )?,
            ffn_norm: f32_vector(
                file,
                source,
                &format!("blk.{layer_index}.ffn_norm.weight"),
                hidden_size,
            )?,
            ffn_gate,
            ffn_up,
            ffn_down: f32_matrix(
                file,
                source,
                &format!("blk.{layer_index}.ffn_down.weight"),
                hidden_size,
                intermediate_size,
            )?,
        });
    }

    let token_embedding = f32_matrix(file, source, "token_embd.weight", vocab_size, hidden_size)?;
    let output = output_matrix_or_tied(file, source, &token_embedding, vocab_size, hidden_size)?;

    Ok((
        config,
        ScalarLlamaWeights {
            token_embedding,
            output_norm: f32_vector(file, source, "output_norm.weight", hidden_size)?,
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
    source: LoaderSource<'_>,
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

    match (tensor.ty, source) {
        (GgmlType::F16, LoaderSource::Mapped(mapped)) => {
            Matrix::from_f16_mapped_bytes(rows, cols, mapped.clone(), tensor.data_range.clone())
        }
        (GgmlType::BF16, LoaderSource::Mapped(mapped)) => {
            Matrix::from_bf16_mapped_bytes(rows, cols, mapped.clone(), tensor.data_range.clone())
        }
        (GgmlType::Q4K, LoaderSource::Mapped(mapped)) => {
            Matrix::from_q4_k_mapped_bytes(rows, cols, mapped.clone(), tensor.data_range.clone())
        }
        (GgmlType::Q5_0, LoaderSource::Mapped(mapped)) => {
            Matrix::from_q5_0_mapped_bytes(rows, cols, mapped.clone(), tensor.data_range.clone())
        }
        (GgmlType::Q5K, LoaderSource::Mapped(mapped)) => {
            Matrix::from_q5_k_mapped_bytes(rows, cols, mapped.clone(), tensor.data_range.clone())
        }
        (GgmlType::Q6K, LoaderSource::Mapped(mapped)) => {
            Matrix::from_q6_k_mapped_bytes(rows, cols, mapped.clone(), tensor.data_range.clone())
        }
        (GgmlType::Q8_0, LoaderSource::Mapped(mapped)) => {
            Matrix::from_q8_0_mapped_bytes(rows, cols, mapped.clone(), tensor.data_range.clone())
        }
        (GgmlType::F16, LoaderSource::Bytes(bytes)) => {
            Matrix::from_f16_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        (GgmlType::BF16, LoaderSource::Bytes(bytes)) => {
            Matrix::from_bf16_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        (GgmlType::Q4K, LoaderSource::Bytes(bytes)) => {
            Matrix::from_q4_k_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        (GgmlType::Q5_0, LoaderSource::Bytes(bytes)) => {
            Matrix::from_q5_0_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        (GgmlType::Q5K, LoaderSource::Bytes(bytes)) => {
            Matrix::from_q5_k_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        (GgmlType::Q6K, LoaderSource::Bytes(bytes)) => {
            Matrix::from_q6_k_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        (GgmlType::Q8_0, LoaderSource::Bytes(bytes)) => {
            Matrix::from_q8_0_row_major_bytes(rows, cols, tensor::raw_bytes(tensor, bytes)?)
        }
        _ => Matrix::from_row_major(rows, cols, tensor::f32_values(tensor, source.as_bytes())?),
    }
}

fn output_matrix_or_tied(
    file: &GgufFile,
    source: LoaderSource<'_>,
    token_embedding: &Matrix,
    rows: usize,
    cols: usize,
) -> Result<ScalarLlamaOutputWeights, InferenceError> {
    if file.tensor("output.weight").is_some() {
        Ok(ScalarLlamaOutputWeights::untied(f32_matrix(
            file,
            source,
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
    source: LoaderSource<'_>,
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

    tensor::f32_values(tensor, source.as_bytes())
}

fn optional_f32_vector(
    file: &GgufFile,
    source: LoaderSource<'_>,
    name: &str,
    len: usize,
) -> Result<Option<Vec<f32>>, InferenceError> {
    if file.tensor(name).is_none() {
        return Ok(None);
    }

    f32_vector(file, source, name, len).map(Some)
}

fn usize_from_u64(value: u64, name: &str) -> Result<usize, InferenceError> {
    usize::try_from(value)
        .map_err(|_error| InferenceError::new(format!("{name} does not fit in usize")))
}
