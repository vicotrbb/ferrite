use ferrite_fixtures::{
    scalar_llama_bf16_gguf_fixture, scalar_llama_f16_gguf_fixture, scalar_llama_f32_gguf_fixture,
    scalar_llama_q4_k_gguf_fixture, scalar_llama_q5_0_gguf_fixture, scalar_llama_q6_k_gguf_fixture,
    scalar_llama_q8_0_gguf_fixture, scalar_llama_tied_output_f32_gguf_fixture,
};
use ferrite_inference::scalar::{
    apply_rope, argmax, rms_norm, Matrix, ScalarLlamaConfig, ScalarLlamaLayerWeights,
    ScalarLlamaModel, ScalarLlamaOutputWeights, ScalarLlamaWeights,
};
use ferrite_model::gguf::parse_gguf;
use ferrite_model::tokenizer::GgufTokenizer;
use std::error::Error;
use std::io;

fn assert_close(actual: f32, expected: f32) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= 0.0001,
        "expected {actual} to be within 0.0001 of {expected}; diff={diff}"
    );
}

#[test]
fn rms_norm_uses_scalar_root_mean_square_reference() -> Result<(), Box<dyn Error>> {
    let output = rms_norm(&[3.0, 4.0], &[1.0, 0.5], 0.0)?;
    let rms = 12.5_f32.sqrt();

    assert_close(output[0], 3.0 / rms);
    assert_close(output[1], 2.0 / rms);
    Ok(())
}

#[test]
fn matrix_vector_multiply_rejects_shape_mismatch() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_row_major(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0])?;

    let error = match matrix.mul_vec(&[1.0, 2.0]) {
        Ok(_) => return Err(io::Error::other("shape mismatch should fail").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("matrix columns 3 do not match vector length 2"));
    Ok(())
}

#[test]
fn single_token_llama_reference_path_produces_documented_argmax() -> Result<(), Box<dyn Error>> {
    let identity = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;
    let config = ScalarLlamaConfig {
        vocab_size: 3,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count: 2,
        rope_freq_base: 10_000.0,
        rms_norm_epsilon: 0.0,
    };
    let model = ScalarLlamaModel::new(
        config,
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
            layers: vec![ScalarLlamaLayerWeights {
                attn_norm: vec![1.0, 1.0],
                q_proj: identity.clone(),
                q_bias: None,
                k_proj: identity.clone(),
                k_bias: None,
                v_proj: identity.clone(),
                v_bias: None,
                o_proj: identity.clone(),
                ffn_norm: vec![1.0, 1.0],
                ffn_gate: identity.clone(),
                ffn_up: identity.clone(),
                ffn_down: identity,
            }],
        },
    )?;

    let next = model.next_token(0)?;

    assert_eq!(next.token_id, 2);
    assert_close(next.logits[0], 0.2);
    assert_close(next.logits[1], 0.2);
    assert_close(next.logits[2], 1.5);
    assert_eq!(argmax(&next.logits)?, 2);
    Ok(())
}

#[test]
fn attention_value_projection_bias_contributes_to_hidden_state() -> Result<(), Box<dyn Error>> {
    let zero = Matrix::from_row_major(2, 2, vec![0.0; 4])?;
    let identity = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;
    let config = ScalarLlamaConfig {
        vocab_size: 3,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count: 0,
        rope_freq_base: 10_000.0,
        rms_norm_epsilon: 0.0,
    };
    let model = ScalarLlamaModel::new(
        config,
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

    let next = model.next_token(0)?;

    assert_eq!(next.token_id, 2);
    assert!(next.logits[2] > next.logits[1]);
    Ok(())
}

#[test]
fn rope_rotates_even_odd_pairs_by_position_and_frequency() -> Result<(), Box<dyn Error>> {
    let rotated = apply_rope(&[1.0, 0.0, 0.0, 1.0], 1, 4, 1.0)?;

    assert_close(rotated[0], 1.0_f32.cos());
    assert_close(rotated[1], 1.0_f32.sin());
    assert_close(rotated[2], -1.0_f32.sin());
    assert_close(rotated[3], 1.0_f32.cos());
    Ok(())
}

#[test]
fn prompt_path_uses_causal_kv_attention_for_latest_token() -> Result<(), Box<dyn Error>> {
    let identity = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;
    let config = ScalarLlamaConfig {
        vocab_size: 4,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count: 0,
        rope_freq_base: 10_000.0,
        rms_norm_epsilon: 0.0,
    };
    let model = ScalarLlamaModel::new(
        config,
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
            layers: vec![ScalarLlamaLayerWeights {
                attn_norm: vec![1.0, 1.0],
                q_proj: identity.clone(),
                q_bias: None,
                k_proj: identity.clone(),
                k_bias: None,
                v_proj: identity.clone(),
                v_bias: None,
                o_proj: identity.clone(),
                ffn_norm: vec![1.0, 1.0],
                ffn_gate: Matrix::from_row_major(2, 2, vec![0.0; 4])?,
                ffn_up: identity.clone(),
                ffn_down: identity,
            }],
        },
    )?;

    let next = model.next_token_for_prompt(&[0, 1])?;

    assert_eq!(next.token_id, 2);
    assert!(next.logits[2] > next.logits[1]);
    Ok(())
}

#[test]
fn scalar_session_reuses_cached_prompt_state_incrementally() -> Result<(), Box<dyn Error>> {
    let identity = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;
    let config = ScalarLlamaConfig {
        vocab_size: 4,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count: 0,
        rope_freq_base: 10_000.0,
        rms_norm_epsilon: 0.0,
    };
    let model = ScalarLlamaModel::new(
        config,
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
            layers: vec![ScalarLlamaLayerWeights {
                attn_norm: vec![1.0, 1.0],
                q_proj: identity.clone(),
                q_bias: None,
                k_proj: identity.clone(),
                k_bias: None,
                v_proj: identity.clone(),
                v_bias: None,
                o_proj: identity.clone(),
                ffn_norm: vec![1.0, 1.0],
                ffn_gate: Matrix::from_row_major(2, 2, vec![0.0; 4])?,
                ffn_up: identity.clone(),
                ffn_down: identity,
            }],
        },
    )?;

    let mut session = model.start_session();
    let prompt_next = session.accept_prompt(&[0, 1])?;
    let full_prompt_next = model.next_token_for_prompt(&[0, 1])?;

    assert_eq!(session.cached_token_count(), 2);
    assert_eq!(prompt_next, full_prompt_next);

    let generated_next = session.accept_token(prompt_next.token_id)?;
    let full_generated_next = model.next_token_for_prompt(&[0, 1, prompt_next.token_id])?;

    assert_eq!(session.cached_token_count(), 3);
    assert_eq!(generated_next, full_generated_next);
    Ok(())
}

#[test]
fn scalar_session_accepts_token_id_without_returning_logits() -> Result<(), Box<dyn Error>> {
    let identity = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;
    let config = ScalarLlamaConfig {
        vocab_size: 4,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count: 0,
        rope_freq_base: 10_000.0,
        rms_norm_epsilon: 0.0,
    };
    let model = ScalarLlamaModel::new(
        config,
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
            layers: vec![ScalarLlamaLayerWeights {
                attn_norm: vec![1.0, 1.0],
                q_proj: identity.clone(),
                q_bias: None,
                k_proj: identity.clone(),
                k_bias: None,
                v_proj: identity.clone(),
                v_bias: None,
                o_proj: identity.clone(),
                ffn_norm: vec![1.0, 1.0],
                ffn_gate: Matrix::from_row_major(2, 2, vec![0.0; 4])?,
                ffn_up: identity.clone(),
                ffn_down: identity,
            }],
        },
    )?;

    let mut logits_session = model.start_session();
    let mut token_id_session = model.start_session();
    let next = logits_session.accept_token(0)?;
    let token_id = token_id_session.accept_token_id(0)?;

    assert_eq!(token_id, next.token_id);
    assert_eq!(
        token_id_session.cached_token_count(),
        logits_session.cached_token_count()
    );
    Ok(())
}

#[test]
fn scalar_session_generates_token_ids_without_returning_logits() -> Result<(), Box<dyn Error>> {
    let identity = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;
    let config = ScalarLlamaConfig {
        vocab_size: 4,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count: 0,
        rope_freq_base: 10_000.0,
        rms_norm_epsilon: 0.0,
    };
    let model = ScalarLlamaModel::new(
        config,
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
            layers: vec![ScalarLlamaLayerWeights {
                attn_norm: vec![1.0, 1.0],
                q_proj: identity.clone(),
                q_bias: None,
                k_proj: identity.clone(),
                k_bias: None,
                v_proj: identity.clone(),
                v_bias: None,
                o_proj: identity.clone(),
                ffn_norm: vec![1.0, 1.0],
                ffn_gate: Matrix::from_row_major(2, 2, vec![0.0; 4])?,
                ffn_up: identity.clone(),
                ffn_down: identity,
            }],
        },
    )?;

    let mut logits_session = model.start_session();
    let mut token_id_session = model.start_session();
    let mut next = logits_session.accept_token(0)?;
    let next_token_id = token_id_session.accept_token_id(0)?;
    let mut expected = Vec::new();
    for _ in 0..3 {
        expected.push(next.token_id);
        next = logits_session.accept_token(next.token_id)?;
    }

    let generated = token_id_session.generate_token_ids(next_token_id, 3)?;

    assert_eq!(generated, expected);
    assert_eq!(
        token_id_session.cached_token_count(),
        logits_session.cached_token_count()
    );
    Ok(())
}

#[test]
fn scalar_model_reports_weight_and_session_kv_cache_bytes() -> Result<(), Box<dyn Error>> {
    let identity = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;
    let config = ScalarLlamaConfig {
        vocab_size: 4,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count: 0,
        rope_freq_base: 10_000.0,
        rms_norm_epsilon: 0.0,
    };
    let model = ScalarLlamaModel::new(
        config,
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
            layers: vec![ScalarLlamaLayerWeights {
                attn_norm: vec![1.0, 1.0],
                q_proj: identity.clone(),
                q_bias: None,
                k_proj: identity.clone(),
                k_bias: None,
                v_proj: identity.clone(),
                v_bias: None,
                o_proj: identity.clone(),
                ffn_norm: vec![1.0, 1.0],
                ffn_gate: Matrix::from_row_major(2, 2, vec![0.0; 4])?,
                ffn_up: identity.clone(),
                ffn_down: identity,
            }],
        },
    )?;

    assert_eq!(model.scalar_weight_bytes(), 200);

    let mut session = model.start_session();
    assert_eq!(session.kv_cache_bytes(), 0);

    let next = session.accept_prompt(&[0, 1])?;
    assert_eq!(session.kv_cache_bytes(), 32);

    session.accept_token(next.token_id)?;
    assert_eq!(session.kv_cache_bytes(), 48);
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_f32_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_f32_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(next.token_id, 2);
    assert_close(next.logits[0], 0.2);
    assert_close(next.logits[1], 0.2);
    assert_close(next.logits[2], 1.5);
    Ok(())
}

#[test]
fn text_prompt_path_encodes_with_gguf_tokenizer_before_forward() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_f32_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let tokenizer = GgufTokenizer::from_gguf(&gguf)?;
    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let next = model.next_token_for_text_prompt(&tokenizer, "hello")?;
    let expected = model.next_token_for_prompt(&[1])?;

    assert_eq!(next.token_id, expected.token_id);
    assert_eq!(next.logits.len(), expected.logits.len());
    for (actual, expected) in next.logits.iter().zip(expected.logits.iter()) {
        assert_close(*actual, *expected);
    }
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_f16_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_f16_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(next.token_id, 2);
    assert!((next.logits[2] - 1.5).abs() < 0.01);
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_bf16_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_bf16_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(next.token_id, 2);
    assert!((next.logits[2] - 1.5).abs() < 0.01);
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_q8_0_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_q8_0_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(model.scalar_weight_bytes(), 8_136);
    assert_eq!(next.token_id, 1);
    assert!(next.logits[1] > next.logits[0]);
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_q5_0_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_q5_0_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(model.scalar_weight_bytes(), 5_400);
    assert_eq!(next.token_id, 1);
    assert!(next.logits[1] > next.logits[0]);
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_q4_k_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_q4_k_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(model.scalar_weight_bytes(), 17_184);
    assert_eq!(next.token_id, 1);
    assert!(next.logits[1] > next.logits[0]);
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_q6_k_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_q6_k_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(model.scalar_weight_bytes(), 24_708);
    assert_eq!(next.token_id, 1);
    assert!(next.logits[1] > next.logits[0]);
    Ok(())
}

#[test]
fn falls_back_to_token_embeddings_for_tied_output_weight() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_tied_output_f32_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(model.scalar_weight_bytes(), 160);
    assert_eq!(next.logits.len(), 3);
    assert!(next.logits[0] > next.logits[1]);
    assert!(next.logits[0] > next.logits[2]);
    Ok(())
}
