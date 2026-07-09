use ferrite_fixtures::{scalar_llama_q5_0_gguf_fixture, scalar_llama_q8_0_gguf_fixture};
use ferrite_inference::scalar::{accept_token_ids_batch, ScalarLlamaModel};
use ferrite_model::gguf::parse_gguf;
use std::error::Error;

fn model_from(bytes: &[u8]) -> Result<ScalarLlamaModel, Box<dyn Error>> {
    let gguf = parse_gguf(bytes)?;
    Ok(ScalarLlamaModel::from_gguf_scalar(&gguf, bytes)?)
}

#[test]
fn batched_step_matches_sequential_sessions_bit_for_bit() -> Result<(), Box<dyn Error>> {
    for fixture in [
        scalar_llama_q5_0_gguf_fixture(),
        scalar_llama_q8_0_gguf_fixture(),
    ] {
        let model = model_from(&fixture)?;
        let prompts: [&[usize]; 3] = [&[0], &[0, 0], &[0, 0, 0]];

        let mut sequential = Vec::new();
        let mut batched = Vec::new();
        let mut sequential_next = Vec::new();
        for prompt in prompts {
            let mut session_a = model.start_session();
            let mut session_b = model.start_session();
            let next_a = session_a.accept_prompt(prompt)?;
            let next_b = session_b.accept_prompt(prompt)?;
            assert_eq!(next_a.token_id, next_b.token_id);
            sequential.push(session_a);
            batched.push(session_b);
            sequential_next.push(next_a.token_id);
        }
        // The tiny fixtures only guarantee a well-conditioned forward pass
        // for token 0, so every step feeds token 0; positions (and thus
        // RoPE and attention) still advance differently per session, and
        // the returned ids must stay bit-identical between paths.
        for step in 0..8 {
            for (session, token_id) in sequential.iter_mut().zip(sequential_next.iter_mut()) {
                *token_id = session.accept_token_id(0)?;
            }
            let feed = vec![0; batched.len()];
            let batched_next = accept_token_ids_batch(&mut batched, &feed)?;
            assert_eq!(
                sequential_next, batched_next,
                "step {step}: batched ids diverged from sequential ids"
            );
        }

        let cached = sequential
            .iter()
            .map(|session| session.cached_token_count())
            .collect::<Vec<_>>();
        let batched_cached = batched
            .iter()
            .map(|session| session.cached_token_count())
            .collect::<Vec<_>>();
        assert_eq!(cached, batched_cached);
    }
    Ok(())
}

#[test]
fn batched_step_rejects_mismatched_inputs() -> Result<(), Box<dyn Error>> {
    let fixture = scalar_llama_q5_0_gguf_fixture();
    let model = model_from(&fixture)?;
    let mut sessions = vec![model.start_session()];
    sessions[0].accept_prompt(&[0])?;

    let error = match accept_token_ids_batch(&mut sessions, &[0, 1]) {
        Ok(_) => return Err("mismatched token id count should fail".into()),
        Err(error) => error,
    };
    assert!(error.to_string().contains("sessions"));

    let error = match accept_token_ids_batch(&mut [], &[]) {
        Ok(_) => return Err("empty batch should fail".into()),
        Err(error) => error,
    };
    assert!(error.to_string().contains("at least one session"));
    Ok(())
}

#[test]
fn batched_step_rejects_sessions_from_different_models() -> Result<(), Box<dyn Error>> {
    let fixture = scalar_llama_q5_0_gguf_fixture();
    let model_a = model_from(&fixture)?;
    let model_b = model_from(&fixture)?;
    let mut sessions = vec![model_a.start_session(), model_b.start_session()];
    for session in sessions.iter_mut() {
        session.accept_prompt(&[0])?;
    }

    let error = match accept_token_ids_batch(&mut sessions, &[0, 0]) {
        Ok(_) => return Err("cross-model batch should fail".into()),
        Err(error) => error,
    };
    assert!(error.to_string().contains("same model"));
    Ok(())
}
