use super::{Matrix, ScalarLlamaLayerWeights, ScalarLlamaWeights};

const F32_BYTES: u128 = std::mem::size_of::<f32>() as u128;

pub(super) fn weights_bytes(weights: &ScalarLlamaWeights) -> u128 {
    matrix_bytes(&weights.token_embedding)
        + vector_bytes(&weights.output_norm)
        + weights.output.untied_matrix().map_or(0, matrix_bytes)
        + weights.layers.iter().map(layer_bytes).sum::<u128>()
}

pub(super) fn kv_cache_bytes(keys: &[Vec<Vec<f32>>], values: &[Vec<Vec<f32>>]) -> u128 {
    nested_vector_bytes(keys) + nested_vector_bytes(values)
}

fn layer_bytes(layer: &ScalarLlamaLayerWeights) -> u128 {
    vector_bytes(&layer.attn_norm)
        + matrix_bytes(&layer.q_proj)
        + optional_vector_bytes(&layer.q_bias)
        + matrix_bytes(&layer.k_proj)
        + optional_vector_bytes(&layer.k_bias)
        + matrix_bytes(&layer.v_proj)
        + optional_vector_bytes(&layer.v_bias)
        + matrix_bytes(&layer.o_proj)
        + vector_bytes(&layer.ffn_norm)
        + matrix_bytes(&layer.ffn_gate)
        + matrix_bytes(&layer.ffn_up)
        + matrix_bytes(&layer.ffn_down)
}

fn matrix_bytes(matrix: &Matrix) -> u128 {
    matrix.storage_bytes()
}

fn vector_bytes(values: &[f32]) -> u128 {
    values.len() as u128 * F32_BYTES
}

fn optional_vector_bytes(values: &Option<Vec<f32>>) -> u128 {
    values.as_deref().map_or(0, vector_bytes)
}

fn nested_vector_bytes(values: &[Vec<Vec<f32>>]) -> u128 {
    values
        .iter()
        .flat_map(|layer| layer.iter())
        .map(|position| vector_bytes(position))
        .sum()
}
