use ferrite_inference::scalar::ScalarLlamaModel;
use ferrite_model::{gguf::parse_gguf, tokenizer::GgufTokenizer};
use std::{error::Error, fmt, fs, path::Path};

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
        self.generate_with_token_callback(prompt, max_tokens, |_| Ok(()))
    }

    pub fn generate_with_token_callback(
        &self,
        prompt: &str,
        max_tokens: usize,
        mut on_token: impl FnMut(&str) -> Result<(), RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
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

        for _ in 0..max_tokens {
            generated_token_ids.push(token_id);
            let token_text = self.decode_token(token_id)?;
            on_token(&token_text)?;
            token_texts.push(token_text);
            if Some(token_id) == self.tokenizer.eos_token_id() {
                break;
            }
            token_id = session.accept_token_id(token_id).map_err(|error| {
                RuntimeError::new(format!("failed to generate next token: {error}"))
            })?;
        }

        let text = self
            .tokenizer
            .decode(&generated_token_ids)
            .map_err(|error| RuntimeError::new(format!("failed to decode completion: {error}")))?;
        Ok(GeneratedText::new(
            text,
            prompt_token_ids.len(),
            generated_token_ids.len(),
            token_texts,
        ))
    }

    fn decode_token(&self, token_id: usize) -> Result<String, RuntimeError> {
        self.tokenizer
            .decode(&[token_id])
            .map_err(|error| RuntimeError::new(format!("failed to decode token: {error}")))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedText {
    text: String,
    prompt_tokens: usize,
    completion_tokens: usize,
    token_texts: Vec<String>,
}

impl GeneratedText {
    pub fn new(
        text: String,
        prompt_tokens: usize,
        completion_tokens: usize,
        token_texts: Vec<String>,
    ) -> Self {
        Self {
            text,
            prompt_tokens,
            completion_tokens,
            token_texts,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn prompt_tokens(&self) -> usize {
        self.prompt_tokens
    }

    pub fn completion_tokens(&self) -> usize {
        self.completion_tokens
    }

    pub fn token_texts(&self) -> &[String] {
        &self.token_texts
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
}

impl RuntimeError {
    fn new(message: impl Into<String>) -> Self {
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
            Ok(())
        })?;

        assert_eq!(pieces, ["winner"]);
        assert_eq!(generated.text(), "winner");
        assert_eq!(generated.token_texts(), pieces);
        Ok(())
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
