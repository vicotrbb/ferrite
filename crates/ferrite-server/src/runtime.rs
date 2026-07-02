mod cache_options;

pub use cache_options::GenerationCacheOptions;

use ferrite_inference::scalar::ScalarLlamaModel;
use ferrite_model::{gguf::parse_gguf, tokenizer::GgufTokenizer};
use std::{error::Error, fmt, fs, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationControl {
    Continue,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationFinishReason {
    Stop,
    Length,
}

#[derive(Debug)]
pub struct InferenceEngine {
    model: ScalarLlamaModel,
    tokenizer: GgufTokenizer,
}

impl InferenceEngine {
    pub fn load(path: &Path) -> Result<Self, RuntimeError> {
        let bytes = fs::read(path)
            .map_err(|error| RuntimeError::new(format!("failed to read model: {error}")))?;
        let gguf = parse_gguf(&bytes)
            .map_err(|error| RuntimeError::new(format!("failed to parse GGUF: {error}")))?;
        let tokenizer = GgufTokenizer::from_gguf(&gguf)
            .map_err(|error| RuntimeError::new(format!("failed to load tokenizer: {error}")))?;
        let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)
            .map_err(|error| RuntimeError::new(format!("failed to load scalar model: {error}")))?;
        Ok(Self { model, tokenizer })
    }

    pub fn generate(&self, prompt: &str, max_tokens: usize) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_token_callback(prompt, max_tokens, |_| Ok(GenerationControl::Continue))
    }

    pub fn generate_with_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        cache_options: GenerationCacheOptions,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_token_callback_and_cache_options(
            prompt,
            max_tokens,
            cache_options,
            |_| Ok(GenerationControl::Continue),
        )
    }

    pub fn generate_with_token_callback(
        &self,
        prompt: &str,
        max_tokens: usize,
        mut on_token: impl FnMut(&str) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_token_callback_and_cache_options(
            prompt,
            max_tokens,
            GenerationCacheOptions::default(),
            &mut on_token,
        )
    }

    pub fn generate_with_token_callback_and_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        cache_options: GenerationCacheOptions,
        mut on_token: impl FnMut(&str) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        let _cache_options = cache_options;
        let prompt_token_ids = self
            .tokenizer
            .encode(prompt)
            .map_err(|error| RuntimeError::new(format!("failed to tokenize prompt: {error}")))?;
        if prompt_token_ids.is_empty() {
            return Err(RuntimeError::new("prompt must contain at least one token"));
        }

        let mut session = self.model.start_session();
        let next = session
            .accept_prompt(&prompt_token_ids)
            .map_err(|error| RuntimeError::new(format!("failed to evaluate prompt: {error}")))?;
        let mut token_id = next.token_id;
        let mut generated_token_ids = Vec::with_capacity(max_tokens);
        let mut token_texts = Vec::with_capacity(max_tokens);
        let mut token_text_buffer = TokenTextBuffer::new();
        let mut finish_reason = GenerationFinishReason::Length;
        let mut stopped_on_eos = false;

        for _ in 0..max_tokens {
            generated_token_ids.push(token_id);
            if Some(token_id) == self.tokenizer.eos_token_id() {
                finish_reason = GenerationFinishReason::Stop;
                stopped_on_eos = true;
                break;
            }
            let control = token_text_buffer.emit_ready_text(
                &generated_token_ids,
                |ids| self.decode_token_text(ids),
                |token_text| {
                    let control = on_token(token_text)?;
                    token_texts.push(token_text.to_owned());
                    Ok(control)
                },
            )?;
            if control == GenerationControl::Stop {
                finish_reason = GenerationFinishReason::Stop;
                break;
            }
            token_id = session.accept_token_id(token_id).map_err(|error| {
                RuntimeError::new(format!("failed to generate next token: {error}"))
            })?;
        }

        let visible_token_ids = if stopped_on_eos {
            &generated_token_ids[..generated_token_ids.len().saturating_sub(1)]
        } else {
            &generated_token_ids
        };
        let text = if visible_token_ids.is_empty() {
            String::new()
        } else {
            self.tokenizer.decode(visible_token_ids).map_err(|error| {
                RuntimeError::new(format!("failed to decode completion: {error}"))
            })?
        };
        Ok(GeneratedText::with_finish_reason(
            text,
            prompt_token_ids.len(),
            generated_token_ids.len(),
            token_texts,
            finish_reason,
        ))
    }

    fn decode_token_text(&self, token_ids: &[usize]) -> Result<Option<String>, RuntimeError> {
        self.tokenizer
            .decode_if_complete(token_ids)
            .map_err(|error| RuntimeError::new(format!("failed to decode token text: {error}")))
    }
}

#[derive(Debug, Default)]
struct TokenTextBuffer {
    emitted_token_count: usize,
}

impl TokenTextBuffer {
    fn new() -> Self {
        Self::default()
    }

    fn emit_ready_text(
        &mut self,
        generated_token_ids: &[usize],
        decode: impl FnOnce(&[usize]) -> Result<Option<String>, RuntimeError>,
        mut on_text: impl FnMut(&str) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GenerationControl, RuntimeError> {
        if self.emitted_token_count >= generated_token_ids.len() {
            return Ok(GenerationControl::Continue);
        }

        let pending_token_ids = &generated_token_ids[self.emitted_token_count..];
        let Some(text) = decode(pending_token_ids)? else {
            return Ok(GenerationControl::Continue);
        };

        self.emitted_token_count = generated_token_ids.len();
        on_text(&text)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedText {
    text: String,
    prompt_tokens: usize,
    cached_prompt_tokens: usize,
    completion_tokens: usize,
    token_texts: Vec<String>,
    finish_reason: GenerationFinishReason,
}

impl GeneratedText {
    pub fn new(
        text: String,
        prompt_tokens: usize,
        completion_tokens: usize,
        token_texts: Vec<String>,
    ) -> Self {
        Self::with_finish_reason(
            text,
            prompt_tokens,
            completion_tokens,
            token_texts,
            GenerationFinishReason::Stop,
        )
    }

    pub fn with_finish_reason(
        text: String,
        prompt_tokens: usize,
        completion_tokens: usize,
        token_texts: Vec<String>,
        finish_reason: GenerationFinishReason,
    ) -> Self {
        Self {
            text,
            prompt_tokens,
            cached_prompt_tokens: 0,
            completion_tokens,
            token_texts,
            finish_reason,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn prompt_tokens(&self) -> usize {
        self.prompt_tokens
    }

    pub fn cached_prompt_tokens(&self) -> usize {
        self.cached_prompt_tokens
    }

    pub fn with_cached_prompt_tokens(
        mut self,
        cached_prompt_tokens: usize,
    ) -> Result<Self, RuntimeError> {
        if cached_prompt_tokens > self.prompt_tokens {
            return Err(RuntimeError::new(format!(
                "cached prompt tokens {cached_prompt_tokens} exceed prompt tokens {}",
                self.prompt_tokens
            )));
        }
        self.cached_prompt_tokens = cached_prompt_tokens;
        Ok(self)
    }

    pub fn completion_tokens(&self) -> usize {
        self.completion_tokens
    }

    pub fn token_texts(&self) -> &[String] {
        &self.token_texts
    }

    pub fn finish_reason(&self) -> GenerationFinishReason {
        self.finish_reason
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
}

impl RuntimeError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn generate_with_token_callback_reports_each_token_piece(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;

        let mut pieces = Vec::new();
        let generated = engine.generate_with_token_callback("hello", 1, |piece| {
            pieces.push(piece.to_owned());
            Ok(GenerationControl::Continue)
        })?;

        assert_eq!(pieces, ["winner"]);
        assert_eq!(generated.text(), "winner");
        assert_eq!(generated.token_texts(), pieces);
        Ok(())
    }

    #[test]
    fn generated_text_records_cached_prompt_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let generated = GeneratedText::new("winner".to_owned(), 4, 1, vec!["winner".to_owned()])
            .with_cached_prompt_tokens(3)?;

        assert_eq!(generated.prompt_tokens(), 4);
        assert_eq!(generated.cached_prompt_tokens(), 3);
        Ok(())
    }

    #[test]
    fn generated_text_rejects_cached_prompt_tokens_above_prompt_tokens() {
        let error = GeneratedText::new("winner".to_owned(), 2, 1, vec!["winner".to_owned()])
            .with_cached_prompt_tokens(3)
            .expect_err("cached prompt tokens above prompt token count should fail");

        assert!(error
            .to_string()
            .contains("cached prompt tokens 3 exceed prompt tokens 2"));
    }

    #[test]
    fn token_text_buffer_waits_for_decodable_utf8_sequence(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = TokenTextBuffer::new();
        let mut generated_token_ids = vec![13];
        let mut pieces = Vec::new();

        let control =
            buffer.emit_ready_text(&generated_token_ids, decode_partial_bpe, |piece| {
                pieces.push(piece.to_owned());
                Ok(GenerationControl::Continue)
            })?;

        assert_eq!(control, GenerationControl::Continue);
        assert!(pieces.is_empty());

        generated_token_ids.push(14);
        let control =
            buffer.emit_ready_text(&generated_token_ids, decode_partial_bpe, |piece| {
                pieces.push(piece.to_owned());
                Ok(GenerationControl::Continue)
            })?;

        assert_eq!(control, GenerationControl::Continue);
        assert_eq!(pieces, ["é"]);
        Ok(())
    }

    fn decode_partial_bpe(ids: &[usize]) -> Result<Option<String>, RuntimeError> {
        match ids {
            [13] => Ok(None),
            [13, 14] => Ok(Some("é".to_owned())),
            other => Err(RuntimeError::new(format!(
                "unexpected token ids: {other:?}"
            ))),
        }
    }

    fn write_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "ferrite-runtime-fixture-{}-{}.gguf",
            std::process::id(),
            FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, ferrite_fixtures::scalar_llama_f32_gguf_fixture())?;
        Ok(path)
    }

    fn remove_fixture_model(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}
