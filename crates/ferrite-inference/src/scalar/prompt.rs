use super::InferenceError;
use ferrite_model::tokenizer::GgufTokenizer;

pub(super) fn encode_text_prompt(
    tokenizer: &GgufTokenizer,
    prompt: &str,
) -> Result<Vec<usize>, InferenceError> {
    tokenizer
        .encode(prompt)
        .map_err(|error| InferenceError::new(error.to_string()))
}
