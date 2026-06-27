use super::{
    math::{dot, ensure_len, softmax},
    InferenceError, ScalarLlamaConfig,
};

pub(super) fn causal_attention(
    config: &ScalarLlamaConfig,
    query: &[f32],
    keys_by_position: &[Vec<f32>],
    values_by_position: &[Vec<f32>],
) -> Result<Vec<f32>, InferenceError> {
    let expected_query = config.attention_head_count * config.head_dim;
    let expected_kv = config.attention_head_count_kv * config.head_dim;
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

    let heads_per_kv = config
        .attention_head_count
        .checked_div(config.attention_head_count_kv)
        .ok_or_else(|| InferenceError::new("invalid zero kv head count"))?;

    let mut output = vec![0.0; expected_query];
    for query_head in 0..config.attention_head_count {
        let kv_head = query_head / heads_per_kv;
        let query_start = query_head * config.head_dim;
        let kv_start = kv_head * config.head_dim;
        let query_slice = &query[query_start..query_start + config.head_dim];
        let mut scores = Vec::with_capacity(keys_by_position.len());

        for key in keys_by_position {
            ensure_len("cached key", key, expected_kv)?;
            let key_slice = &key[kv_start..kv_start + config.head_dim];
            scores.push(dot(query_slice, key_slice)? / (config.head_dim as f32).sqrt());
        }

        let weights = softmax(&scores)?;
        for (position, value) in values_by_position.iter().enumerate() {
            ensure_len("cached value", value, expected_kv)?;
            let value_slice = &value[kv_start..kv_start + config.head_dim];
            for dimension in 0..config.head_dim {
                output[query_start + dimension] += weights[position] * value_slice[dimension];
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_for_ratio(heads_per_kv: usize) -> ScalarLlamaConfig {
        let kv_heads = 2;
        let head_dim = 2;
        ScalarLlamaConfig {
            vocab_size: 1,
            hidden_size: heads_per_kv * kv_heads * head_dim,
            intermediate_size: 1,
            attention_head_count: heads_per_kv * kv_heads,
            attention_head_count_kv: kv_heads,
            head_dim,
            rope_dimension_count: 0,
            rope_freq_base: 10_000.0,
            rms_norm_epsilon: 0.0,
        }
    }

    #[test]
    fn gqa_broadcasts_kv_heads_for_tier1_ratios() -> Result<(), InferenceError> {
        for heads_per_kv in [1, 3, 4, 6, 7] {
            let config = config_for_ratio(heads_per_kv);
            let query = vec![1.0; config.hidden_size];
            let keys = vec![vec![0.0; config.attention_head_count_kv * config.head_dim]];
            let values = vec![vec![10.0, 11.0, 20.0, 21.0]];

            let output = causal_attention(&config, &query, &keys, &values)?;

            for query_head in 0..config.attention_head_count {
                let kv_head = query_head / heads_per_kv;
                let output_start = query_head * config.head_dim;
                let kv_start = kv_head * config.head_dim;
                assert_eq!(
                    &output[output_start..output_start + config.head_dim],
                    &values[0][kv_start..kv_start + config.head_dim],
                    "heads_per_kv={heads_per_kv}, query_head={query_head}"
                );
            }
        }
        Ok(())
    }
}
