use ferrite_inference::scalar::{
    Matrix, RopeLayout, ScalarLlamaConfig, ScalarLlamaLayerWeights, ScalarLlamaModel,
    ScalarLlamaOutputWeights, ScalarLlamaWeights,
};
use std::error::Error;
use std::io;

#[test]
fn session_truncates_kv_cache_to_resume_prior_turn() -> Result<(), Box<dyn Error>> {
    let model = cache_model()?;
    let mut session = model.start_session();

    session.accept_prompt(&[0, 1])?;
    assert_eq!(session.cached_token_count(), 2);
    assert_eq!(session.kv_cache_bytes(), 32);

    session.truncate_cache(1)?;
    assert_eq!(session.cached_token_count(), 1);
    assert_eq!(session.kv_cache_bytes(), 16);

    let resumed = session.accept_token(1)?;
    let recomputed = model.next_token_for_prompt(&[0, 1])?;
    assert_eq!(resumed, recomputed);
    assert_eq!(session.cached_token_count(), 2);
    Ok(())
}

#[test]
fn session_restores_cache_snapshot_to_resume_prefix() -> Result<(), Box<dyn Error>> {
    let model = cache_model()?;
    let mut prefix_session = model.start_session();

    prefix_session.accept_token(0)?;
    let snapshot = prefix_session.cache_snapshot();
    assert_eq!(snapshot.cached_token_count(), 1);
    assert_eq!(snapshot.kv_cache_bytes(), 16);

    let mut resumed_session = model.start_session();
    resumed_session.restore_cache_snapshot(&snapshot)?;
    assert_eq!(resumed_session.cached_token_count(), 1);
    assert_eq!(resumed_session.kv_cache_bytes(), 16);

    let resumed = resumed_session.accept_token(1)?;
    let recomputed = model.next_token_for_prompt(&[0, 1])?;

    assert_eq!(resumed, recomputed);
    assert_eq!(resumed_session.cached_token_count(), 2);
    assert_eq!(resumed_session.kv_cache_bytes(), 32);
    Ok(())
}

#[test]
fn session_rejects_truncation_beyond_cached_tokens() -> Result<(), Box<dyn Error>> {
    let model = cache_model()?;
    let mut session = model.start_session();

    session.accept_token(0)?;
    let error = match session.truncate_cache(2) {
        Ok(_) => return Err(io::Error::other("cache truncation should not extend").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("cannot truncate kv cache from 1 tokens to 2 tokens"));
    Ok(())
}

fn cache_model() -> Result<ScalarLlamaModel, Box<dyn Error>> {
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
        rope_layout: RopeLayout::AdjacentPairs,
        rms_norm_epsilon: 0.0,
    };

    Ok(ScalarLlamaModel::new(
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
    )?)
}
