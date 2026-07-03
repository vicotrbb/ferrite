use ferrite_fixtures::scalar_llama_f32_gguf_fixture;
use ferrite_inference::scalar::{PromptEvaluationControl, ScalarLlamaModel};
use ferrite_model::gguf::parse_gguf;

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
