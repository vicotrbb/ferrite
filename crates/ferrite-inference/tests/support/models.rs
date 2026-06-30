use ferrite_inference::scalar::{
    Matrix, RopeLayout, ScalarLlamaConfig, ScalarLlamaLayerWeights, ScalarLlamaModel,
    ScalarLlamaOutputWeights, ScalarLlamaWeights,
};
use std::error::Error;

pub(crate) fn documented_argmax_model() -> Result<ScalarLlamaModel, Box<dyn Error>> {
    let identity = identity_2x2()?;
    let model = ScalarLlamaModel::new(
        scalar_config_with_rope_dimensions(3, 2),
        ScalarLlamaWeights {
            token_embedding: Matrix::from_row_major(
                3,
                2,
                vec![
                    1.0, 1.0, // token 0
                    0.0, 1.0, // token 1
                    2.0, -1.0, // token 2
                ],
            )?,
            output_norm: vec![1.0, 1.0],
            output: ScalarLlamaOutputWeights::untied(Matrix::from_row_major(
                3,
                2,
                vec![
                    0.1, 0.1, // token 0 logit = 0.2 after final norm
                    0.2, 0.0, // token 1 logit = 0.2 after final norm
                    1.0, 0.5, // token 2 logit = 1.5 after final norm
                ],
            )?),
            layers: vec![identity_layer(
                identity.clone(),
                identity.clone(),
                identity,
            )?],
        },
    )?;
    Ok(model)
}

pub(crate) fn value_bias_model() -> Result<ScalarLlamaModel, Box<dyn Error>> {
    let zero = zero_2x2()?;
    let identity = identity_2x2()?;
    let model = ScalarLlamaModel::new(
        scalar_config(3),
        ScalarLlamaWeights {
            token_embedding: Matrix::from_row_major(
                3,
                2,
                vec![
                    1.0, 0.0, // token 0
                    0.0, 0.0, // token 1
                    0.0, 0.0, // token 2
                ],
            )?,
            output_norm: vec![1.0, 1.0],
            output: ScalarLlamaOutputWeights::untied(Matrix::from_row_major(
                3,
                2,
                vec![
                    0.0, 0.0, // token 0
                    1.0, 0.0, // token 1 follows the original hidden state
                    0.0, 1.0, // token 2 follows the value bias contribution
                ],
            )?),
            layers: vec![ScalarLlamaLayerWeights {
                attn_norm: vec![1.0, 1.0],
                q_proj: zero.clone(),
                q_bias: None,
                k_proj: zero.clone(),
                k_bias: None,
                v_proj: zero.clone(),
                v_bias: Some(vec![0.0, 3.0]),
                o_proj: identity.clone(),
                ffn_norm: vec![1.0, 1.0],
                ffn_gate: zero.clone(),
                ffn_up: identity,
                ffn_down: zero,
            }],
        },
    )?;
    Ok(model)
}

pub(crate) fn prompt_causal_attention_model() -> Result<ScalarLlamaModel, Box<dyn Error>> {
    let identity = identity_2x2()?;
    let model = ScalarLlamaModel::new(
        scalar_config(4),
        ScalarLlamaWeights {
            token_embedding: Matrix::from_row_major(
                4,
                2,
                vec![
                    1.0, 0.0, // token 0
                    0.0, 1.0, // token 1
                    0.0, 0.0, // token 2
                    0.0, 0.0, // token 3
                ],
            )?,
            output_norm: vec![1.0, 1.0],
            output: ScalarLlamaOutputWeights::untied(Matrix::from_row_major(
                4,
                2,
                vec![
                    0.0, 0.0, // token 0
                    1.0, 0.0, // token 1 follows attention toward prior token
                    0.0, 1.0, // token 2 follows current token
                    -1.0, -1.0, // token 3
                ],
            )?),
            layers: vec![identity_layer(identity.clone(), zero_2x2()?, identity)?],
        },
    )?;
    Ok(model)
}

pub(crate) fn causal_attention_model() -> Result<ScalarLlamaModel, Box<dyn Error>> {
    let identity = identity_2x2()?;
    let model = ScalarLlamaModel::new(
        scalar_config(4),
        ScalarLlamaWeights {
            token_embedding: Matrix::from_row_major(
                4,
                2,
                vec![
                    1.0, 0.0, // token 0
                    0.0, 1.0, // token 1
                    1.0, 1.0, // token 2
                    0.0, 0.0, // token 3
                ],
            )?,
            output_norm: vec![1.0, 1.0],
            output: ScalarLlamaOutputWeights::untied(Matrix::from_row_major(
                4,
                2,
                vec![
                    0.0, 0.0, // token 0
                    1.0, 0.0, // token 1
                    0.0, 1.0, // token 2
                    -1.0, -1.0, // token 3
                ],
            )?),
            layers: vec![identity_layer(identity.clone(), zero_2x2()?, identity)?],
        },
    )?;
    Ok(model)
}

fn scalar_config(vocab_size: usize) -> ScalarLlamaConfig {
    scalar_config_with_rope_dimensions(vocab_size, 0)
}

fn scalar_config_with_rope_dimensions(
    vocab_size: usize,
    rope_dimension_count: usize,
) -> ScalarLlamaConfig {
    ScalarLlamaConfig {
        vocab_size,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count,
        rope_freq_base: 10_000.0,
        rope_layout: RopeLayout::AdjacentPairs,
        rms_norm_epsilon: 0.0,
    }
}

fn identity_layer(
    attention_projection: Matrix,
    ffn_gate: Matrix,
    ffn_projection: Matrix,
) -> Result<ScalarLlamaLayerWeights, Box<dyn Error>> {
    Ok(ScalarLlamaLayerWeights {
        attn_norm: vec![1.0, 1.0],
        q_proj: attention_projection.clone(),
        q_bias: None,
        k_proj: attention_projection.clone(),
        k_bias: None,
        v_proj: attention_projection.clone(),
        v_bias: None,
        o_proj: attention_projection,
        ffn_norm: vec![1.0, 1.0],
        ffn_gate,
        ffn_up: ffn_projection.clone(),
        ffn_down: ffn_projection,
    })
}

fn identity_2x2() -> Result<Matrix, Box<dyn Error>> {
    Ok(Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?)
}

fn zero_2x2() -> Result<Matrix, Box<dyn Error>> {
    Ok(Matrix::from_row_major(2, 2, vec![0.0; 4])?)
}
