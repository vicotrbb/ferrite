use super::{math::ensure_len, InferenceError, Matrix, ScalarLlamaConfig, ScalarLlamaWeights};

pub(super) fn validate_config(config: &ScalarLlamaConfig) -> Result<(), InferenceError> {
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
    if !config.rope_freq_base.is_finite() {
        return Err(InferenceError::new("rope frequency base must be finite"));
    }
    if config.rope_freq_base <= 0.0 {
        return Err(InferenceError::new(format!(
            "rope frequency base {} must be positive",
            config.rope_freq_base
        )));
    }

    Ok(())
}

pub(super) fn validate_weights(
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
        ensure_optional_len(
            &format!("{prefix} q_bias"),
            layer.q_bias.as_deref(),
            config.hidden_size,
        )?;
        ensure_matrix_shape(
            &format!("{prefix} k_proj"),
            &layer.k_proj,
            kv_width,
            config.hidden_size,
        )?;
        ensure_optional_len(
            &format!("{prefix} k_bias"),
            layer.k_bias.as_deref(),
            kv_width,
        )?;
        ensure_matrix_shape(
            &format!("{prefix} v_proj"),
            &layer.v_proj,
            kv_width,
            config.hidden_size,
        )?;
        ensure_optional_len(
            &format!("{prefix} v_bias"),
            layer.v_bias.as_deref(),
            kv_width,
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

fn ensure_optional_len(
    name: &str,
    values: Option<&[f32]>,
    expected: usize,
) -> Result<(), InferenceError> {
    match values {
        Some(values) => ensure_len(name, values, expected),
        None => Ok(()),
    }
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
