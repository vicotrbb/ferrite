use super::{
    kv_store::KvCacheStore,
    math::{dot, ensure_len, softmax},
    InferenceError, ScalarLlamaConfig,
};

pub(super) fn causal_attention(
    config: &ScalarLlamaConfig,
    query: &[f32],
    store: &mut KvCacheStore,
    layer: usize,
) -> Result<Vec<f32>, InferenceError> {
    let expected_query = config.attention_head_count * config.head_dim;
    let expected_kv = config.attention_head_count_kv * config.head_dim;
    ensure_len("query", query, expected_query)?;

    let position_count = store.layer_len(layer);
    if position_count == 0 {
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

        let mut scores = Vec::with_capacity(position_count);
        for position in 0..position_count {
            let key = store.key(layer, position)?;
            ensure_len("cached key", key, expected_kv)?;
            let key_slice = &key[kv_start..kv_start + config.head_dim];
            scores.push(dot(query_slice, key_slice)? / (config.head_dim as f32).sqrt());
        }

        let weights = softmax(&scores)?;
        for (position, weight) in weights.iter().copied().enumerate() {
            let value = store.value(layer, position)?;
            ensure_len("cached value", value, expected_kv)?;
            let value_slice = &value[kv_start..kv_start + config.head_dim];
            if value_slice.iter().any(|value| !value.is_finite()) {
                return Err(InferenceError::new("cached value must be finite"));
            }
            for dimension in 0..config.head_dim {
                output[query_start + dimension] += weight * value_slice[dimension];
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scalar::kv_store::KvCacheStore;
    use crate::scalar::RopeLayout;

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
            rope_layout: RopeLayout::AdjacentPairs,
            rms_norm_epsilon: 0.0,
        }
    }

    fn single_position_store(
        config: &ScalarLlamaConfig,
        value: Vec<f32>,
    ) -> Result<KvCacheStore, InferenceError> {
        let dim = config.attention_head_count_kv * config.head_dim;
        let mut store = KvCacheStore::new_vec(1, dim);
        store.push(0, vec![0.0; dim], value)?;
        Ok(store)
    }

    #[test]
    fn gqa_broadcasts_kv_heads_for_tier1_ratios() -> Result<(), InferenceError> {
        for heads_per_kv in [1, 3, 4, 6, 7] {
            let config = config_for_ratio(heads_per_kv);
            let query = vec![1.0; config.hidden_size];
            let value = vec![10.0, 11.0, 20.0, 21.0];
            let mut store = single_position_store(&config, value.clone())?;

            let output = causal_attention(&config, &query, &mut store, 0)?;

            for query_head in 0..config.attention_head_count {
                let kv_head = query_head / heads_per_kv;
                let output_start = query_head * config.head_dim;
                let kv_start = kv_head * config.head_dim;
                assert_eq!(
                    &output[output_start..output_start + config.head_dim],
                    &value[kv_start..kv_start + config.head_dim],
                    "heads_per_kv={heads_per_kv}, query_head={query_head}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn attention_rejects_non_finite_cached_values() -> Result<(), InferenceError> {
        let config = config_for_ratio(1);
        let query = vec![1.0; config.hidden_size];

        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let dim = config.attention_head_count_kv * config.head_dim;
            let mut values = vec![0.0; dim];
            values[0] = value;
            let mut store = single_position_store(&config, values)?;

            let error = match causal_attention(&config, &query, &mut store, 0) {
                Ok(_) => return Err(InferenceError::new("non-finite cached value should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("cached value must be finite"));
        }
        Ok(())
    }
}
