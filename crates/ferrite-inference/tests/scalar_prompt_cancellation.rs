#[allow(
    dead_code,
    reason = "fixture fields document cancellation observations"
)]
mod support;

use ferrite_fixtures::scalar_llama_f32_gguf_fixture;
use ferrite_inference::scalar::{PromptEvaluationControl, ScalarLlamaModel};
use ferrite_model::gguf::parse_gguf;
use support::models::documented_argmax_model;

#[test]
fn accept_prompt_with_control_stops_before_next_prompt_token(
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = scalar_llama_f32_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;
    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let mut session = model.start_session();
    let mut observed_tokens = Vec::new();

    let error = match session.accept_prompt_with_control(&[0, 1, 0], |index, token_id| {
        observed_tokens.push((index, token_id));
        if index == 1 {
            Ok(PromptEvaluationControl::Cancel)
        } else {
            Ok(PromptEvaluationControl::Continue)
        }
    }) {
        Ok(_) => return Err("prompt evaluation should stop when cancellation is requested".into()),
        Err(error) => error,
    };

    assert_eq!(error.to_string(), "prompt evaluation cancelled");
    assert_eq!(observed_tokens, [(0, 0), (1, 1)]);
    assert_eq!(session.cached_token_count(), 1);
    Ok(())
}

#[test]
fn accept_prompt_with_cancellation_stops_during_prompt_token_evaluation(
) -> Result<(), Box<dyn std::error::Error>> {
    let model = documented_argmax_model()?;
    let mut session = model.start_session();
    let mut polls = 0;

    let error = match session.accept_prompt_with_cancellation(&[0], || {
        polls += 1;
        if polls == 2 {
            Ok(PromptEvaluationControl::Cancel)
        } else {
            Ok(PromptEvaluationControl::Continue)
        }
    }) {
        Ok(_) => return Err("prompt evaluation should stop during token evaluation".into()),
        Err(error) => error,
    };

    assert_eq!(error.to_string(), "prompt evaluation cancelled");
    assert_eq!(polls, 2);
    assert_eq!(session.cached_token_count(), 0);
    Ok(())
}

#[test]
fn accept_prompt_with_control_and_cancellation_keeps_token_context_and_layer_polling(
) -> Result<(), Box<dyn std::error::Error>> {
    let model = documented_argmax_model()?;
    let mut session = model.start_session();
    let mut observed_tokens = Vec::new();
    let mut cancellation_polls = 0;

    let error = match session.accept_prompt_with_control_and_cancellation(
        &[0],
        |index, token_id| {
            observed_tokens.push((index, token_id));
            Ok(PromptEvaluationControl::Continue)
        },
        || {
            cancellation_polls += 1;
            if cancellation_polls == 2 {
                Ok(PromptEvaluationControl::Cancel)
            } else {
                Ok(PromptEvaluationControl::Continue)
            }
        },
    ) {
        Ok(_) => return Err("prompt evaluation should stop during layer polling".into()),
        Err(error) => error,
    };

    assert_eq!(error.to_string(), "prompt evaluation cancelled");
    assert_eq!(observed_tokens, [(0, 0)]);
    assert_eq!(cancellation_polls, 2);
    assert_eq!(session.cached_token_count(), 0);
    Ok(())
}
