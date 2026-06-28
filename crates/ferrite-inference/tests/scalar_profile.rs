use ferrite_inference::scalar::{
    Matrix, MatrixStorageKind, RopeLayout, ScalarLlamaConfig, ScalarLlamaLayerWeights,
    ScalarLlamaModel, ScalarLlamaOutputWeights, ScalarLlamaWeights,
};
use std::error::Error;

#[test]
fn token_id_only_profile_records_output_argmax_matrix() -> Result<(), Box<dyn Error>> {
    let model = profile_model()?;
    let mut profiled_session = model.start_session();
    let mut reference_session = model.start_session();

    let profiled = profiled_session.accept_token_id_profiled(0)?;
    let reference_token_id = reference_session.accept_token_id(0)?;

    assert_eq!(profiled.token_id, reference_token_id);
    assert!(profiled.total_elapsed() >= profiled.events[0].elapsed());

    let output_event = profiled
        .events
        .iter()
        .find(|event| event.label() == "output")
        .ok_or("missing output profile event")?;
    assert!(output_event.elapsed().as_nanos() > 0);
    assert_eq!(output_event.storage_kind(), MatrixStorageKind::F32);
    assert_eq!(output_event.rows(), 4);
    assert_eq!(output_event.cols(), 2);
    assert_eq!(output_event.storage_bytes(), 32);
    Ok(())
}

fn profile_model() -> Result<ScalarLlamaModel, Box<dyn Error>> {
    let zero = Matrix::from_row_major(2, 2, vec![0.0; 4])?;
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
                ffn_gate: zero.clone(),
                ffn_up: identity,
                ffn_down: zero,
            }],
        },
    )?)
}
