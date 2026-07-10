use super::{
    kv_store::KvCacheStore,
    math::{ensure_len, softmax_in_place},
    InferenceError, ScalarLlamaConfig,
};

pub(super) fn causal_attention(
    config: &ScalarLlamaConfig,
    query: &[f32],
    store: &mut KvCacheStore,
    layer: usize,
) -> Result<Vec<f32>, InferenceError> {
    let expected_query = config.attention_head_count * config.head_dim;
    ensure_len("query", query, expected_query)?;
    if query.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("dot left must be finite"));
    }

    let position_count = store.layer_len(layer);
    if position_count == 0 {
        return Err(InferenceError::new("attention cache must not be empty"));
    }

    let heads_per_kv = config
        .attention_head_count
        .checked_div(config.attention_head_count_kv)
        .ok_or_else(|| InferenceError::new("invalid zero kv head count"))?;

    let mut output = vec![0.0; expected_query];
    let mut scores = Vec::with_capacity(position_count);
    let attention_scale = (config.head_dim as f32).sqrt();
    for query_head in 0..config.attention_head_count {
        let kv_head = query_head / heads_per_kv;
        let query_start = query_head * config.head_dim;
        let kv_start = kv_head * config.head_dim;
        let query_slice = &query[query_start..query_start + config.head_dim];

        scores.clear();
        for position in 0..position_count {
            let key = store.key(layer, position)?;
            let key_slice = &key[kv_start..kv_start + config.head_dim];
            scores.push(attention_dot(query_slice, key_slice)? / attention_scale);
        }

        softmax_in_place(&mut scores)?;
        for (position, weight) in scores.iter().copied().enumerate() {
            let value = store.value(layer, position)?;
            let value_slice = &value[kv_start..kv_start + config.head_dim];
            for dimension in 0..config.head_dim {
                output[query_start + dimension] += weight * value_slice[dimension];
            }
        }
    }

    Ok(output)
}

fn attention_dot(left: &[f32], right: &[f32]) -> Result<f32, InferenceError> {
    debug_assert_eq!(left.len(), right.len());
    let mut sum = 0.0;
    for (left, right) in left.iter().zip(right) {
        sum += *left * *right;
    }
    if !sum.is_finite() {
        return Err(InferenceError::new("dot result must be finite"));
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scalar::kv_store::KvCacheStore;
    use crate::scalar::math::dot;
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
    fn kv_cache_rejects_non_finite_values_before_attention() -> Result<(), InferenceError> {
        let config = config_for_ratio(1);

        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let dim = config.attention_head_count_kv * config.head_dim;
            let mut values = vec![0.0; dim];
            values[0] = value;
            let error = match single_position_store(&config, values) {
                Ok(_) => return Err(InferenceError::new("non-finite cached value should fail")),
                Err(error) => error,
            };

            assert!(error.to_string().contains("cached value must be finite"));
        }
        Ok(())
    }

    #[test]
    fn optimized_attention_is_bit_identical_to_reference() -> Result<(), InferenceError> {
        let config = config_for_ratio(3);
        let dim = config.attention_head_count_kv * config.head_dim;
        let mut optimized_store = KvCacheStore::new_vec(1, dim);
        let mut reference_store = KvCacheStore::new_vec(1, dim);
        for position in 0..7 {
            let key = (0..dim)
                .map(|index| ((position * 17 + index * 11) as f32 - 23.0) / 13.0)
                .collect::<Vec<_>>();
            let value = (0..dim)
                .map(|index| ((position * 7 + index * 19) as f32 - 29.0) / 17.0)
                .collect::<Vec<_>>();
            optimized_store.push(0, key.clone(), value.clone())?;
            reference_store.push(0, key, value)?;
        }
        let query = (0..config.hidden_size)
            .map(|index| ((index * 23 % 41) as f32 - 20.0) / 9.0)
            .collect::<Vec<_>>();

        let actual = causal_attention(&config, &query, &mut optimized_store, 0)?;
        let expected = reference_attention(&config, &query, &mut reference_store, 0)?;

        assert_eq!(actual, expected);
        Ok(())
    }

    fn reference_attention(
        config: &ScalarLlamaConfig,
        query: &[f32],
        store: &mut KvCacheStore,
        layer: usize,
    ) -> Result<Vec<f32>, InferenceError> {
        let position_count = store.layer_len(layer);
        let heads_per_kv = config.attention_head_count / config.attention_head_count_kv;
        let mut output = vec![0.0; config.attention_head_count * config.head_dim];
        for query_head in 0..config.attention_head_count {
            let kv_head = query_head / heads_per_kv;
            let query_start = query_head * config.head_dim;
            let kv_start = kv_head * config.head_dim;
            let query_slice = &query[query_start..query_start + config.head_dim];
            let mut scores = Vec::with_capacity(position_count);
            for position in 0..position_count {
                let key = store.key(layer, position)?;
                scores.push(
                    dot(query_slice, &key[kv_start..kv_start + config.head_dim])?
                        / (config.head_dim as f32).sqrt(),
                );
            }
            let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let exponentials = scores
                .iter()
                .map(|score| (*score - max).exp())
                .collect::<Vec<_>>();
            let sum = exponentials.iter().sum::<f32>();
            let weights = exponentials
                .into_iter()
                .map(|value| value / sum)
                .collect::<Vec<_>>();
            for (position, weight) in weights.into_iter().enumerate() {
                let value = store.value(layer, position)?;
                for dimension in 0..config.head_dim {
                    output[query_start + dimension] += weight * value[kv_start + dimension];
                }
            }
        }
        Ok(output)
    }
}
