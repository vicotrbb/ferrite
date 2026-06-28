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
            token_texts.push(self.decode_token(token_id)?);
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
