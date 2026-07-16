use ferrite_fixtures::{
    scalar_llama_f32_gguf_fixture, scalar_llama_f32_gguf_fixture_with_context_length,
};
use ferrite_inference::scalar::{ScalarLlamaModel, accept_token_contexts_batch};
use ferrite_model::gguf::parse_gguf;
use std::error::Error;

#[test]
fn gguf_context_limit_rejects_the_next_sequential_position() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_f32_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;
    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    assert_eq!(model.context_length(), Some(128));
    let mut session = model.start_session();

    for _ in 0..128 {
        session.accept_token_context_only(1)?;
    }
    let error = match session.accept_token_context_only(1) {
        Ok(()) => return Err("token beyond the GGUF context should fail".into()),
        Err(error) => error,
    };

    assert_eq!(session.cached_token_count(), 128);
    assert_eq!(
        error.to_string(),
        "model context length 128 is exhausted at token position 128"
    );
    Ok(())
}

#[test]
fn gguf_context_limit_rejects_a_batch_before_mutating_any_session() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_f32_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;
    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let mut sessions = [model.start_session(), model.start_session()];
    for _ in 0..128 {
        accept_token_contexts_batch(&mut sessions, &[1, 1])?;
    }

    let error = match accept_token_contexts_batch(&mut sessions, &[1, 1]) {
        Ok(()) => return Err("batch beyond the GGUF context should fail".into()),
        Err(error) => error,
    };

    assert_eq!(
        sessions.map(|session| session.cached_token_count()),
        [128, 128]
    );
    assert_eq!(
        error.to_string(),
        "model context length 128 is exhausted at token position 128"
    );
    Ok(())
}

#[test]
fn executes_two_and_eight_k_token_context_boundaries() -> Result<(), Box<dyn Error>> {
    for context_length in [2_048usize, 8_192] {
        let bytes = scalar_llama_f32_gguf_fixture_with_context_length(context_length as u64);
        let gguf = parse_gguf(&bytes)?;
        let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
        assert_eq!(model.context_length(), Some(context_length));
        let mut session = model.start_session();

        for _ in 0..context_length {
            session.accept_token_context_only(1)?;
        }
        let error = match session.accept_token_context_only(1) {
            Ok(()) => {
                return Err(format!("token beyond {context_length}-token context passed").into());
            }
            Err(error) => error,
        };

        assert_eq!(session.cached_token_count(), context_length);
        assert_eq!(
            error.to_string(),
            format!(
                "model context length {context_length} is exhausted at token position {context_length}"
            )
        );
    }
    Ok(())
}
