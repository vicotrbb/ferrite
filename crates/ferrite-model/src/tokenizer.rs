mod bpe;

use crate::gguf::{GgufError, GgufFile, MetadataValue, MetadataValueType};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GgufTokenizer {
    model: TokenizerModel,
    tokens: Vec<String>,
    token_types: Vec<TokenType>,
    merges: Vec<String>,
    bpe_metadata: Option<bpe::BpeMetadata>,
    eos_token_id: Option<usize>,
}

impl GgufTokenizer {
    pub fn from_gguf(file: &GgufFile) -> Result<Self, TokenizerError> {
        let model = match metadata_string(file, "tokenizer.ggml.model")? {
            "llama" => TokenizerModel::Llama,
            other => TokenizerModel::Other(other.to_owned()),
        };
        let tokens = metadata_string_array(file, "tokenizer.ggml.tokens")?;
        let token_types = match metadata_i32_array(file, "tokenizer.ggml.token_type")? {
            Some(values) => {
                if values.len() != tokens.len() {
                    return Err(TokenizerError::new(format!(
                        "token_type length {} does not match tokens length {}",
                        values.len(),
                        tokens.len()
                    )));
                }
                values.into_iter().map(TokenType::from_gguf).collect()
            }
            None => vec![TokenType::Normal; tokens.len()],
        };

        let merges = metadata_optional_string_array(file, "tokenizer.ggml.merges")?;
        let bpe_metadata = if merges.is_empty() {
            None
        } else {
            Some(bpe::BpeMetadata::new(&tokens, &merges)?)
        };

        Ok(Self {
            model,
            tokens,
            token_types,
            merges,
            bpe_metadata,
            eos_token_id: metadata_optional_usize(file, "tokenizer.ggml.eos_token_id")?,
        })
    }

    pub fn model(&self) -> TokenizerModel {
        self.model.clone()
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn token(&self, id: usize) -> Option<&str> {
        self.tokens.get(id).map(String::as_str)
    }

    pub fn token_type(&self, id: usize) -> Option<TokenType> {
        self.token_types.get(id).copied()
    }

    pub fn eos_token_id(&self) -> Option<usize> {
        self.eos_token_id
    }

    pub fn decode(&self, ids: &[usize]) -> Result<String, TokenizerError> {
        if !self.merges.is_empty() {
            return bpe::decode(ids, &self.tokens);
        }

        let mut output = String::new();
        for id in ids {
            let token = self
                .tokens
                .get(*id)
                .ok_or_else(|| TokenizerError::new(format!("token id {id} is out of bounds")))?;
            output.push_str(token);
        }
        Ok(output)
    }

    pub fn decode_if_complete(&self, ids: &[usize]) -> Result<Option<String>, TokenizerError> {
        match self.decode(ids) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.is_incomplete_utf8() => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn encode_atomic(&self, input: &str) -> Result<Vec<usize>, TokenizerError> {
        self.encode_atomic_with_cancellation(input, || TokenizationControl::Continue)
    }

    pub fn encode_atomic_with_cancellation(
        &self,
        input: &str,
        mut on_cancellation_poll: impl FnMut() -> TokenizationControl,
    ) -> Result<Vec<usize>, TokenizerError> {
        let mut output = Vec::new();
        let mut cursor = 0usize;
        if on_cancellation_poll() == TokenizationControl::Cancel {
            return Err(TokenizerError::cancelled());
        }
        while cursor < input.len() {
            if on_cancellation_poll() == TokenizationControl::Cancel {
                return Err(TokenizerError::cancelled());
            }
            let suffix = &input[cursor..];
            let Some((id, token)) = self.longest_token_prefix(suffix) else {
                return Err(TokenizerError::new(format!(
                    "no atomic token matches input at byte offset {cursor}"
                )));
            };
            output.push(id);
            cursor += token.len();
        }
        Ok(output)
    }

    pub fn encode(&self, input: &str) -> Result<Vec<usize>, TokenizerError> {
        self.encode_with_cancellation(input, || TokenizationControl::Continue)
    }

    pub fn encode_with_cancellation(
        &self,
        input: &str,
        on_cancellation_poll: impl FnMut() -> TokenizationControl,
    ) -> Result<Vec<usize>, TokenizerError> {
        if self.merges.is_empty() {
            self.encode_atomic_with_cancellation(input, on_cancellation_poll)
        } else {
            self.encode_bpe_with_cancellation(input, on_cancellation_poll)
        }
    }

    pub fn encode_bpe(&self, input: &str) -> Result<Vec<usize>, TokenizerError> {
        self.encode_bpe_with_cancellation(input, || TokenizationControl::Continue)
    }

    pub fn encode_bpe_with_cancellation(
        &self,
        input: &str,
        on_cancellation_poll: impl FnMut() -> TokenizationControl,
    ) -> Result<Vec<usize>, TokenizerError> {
        let Some(metadata) = &self.bpe_metadata else {
            return Err(TokenizerError::new("BPE tokenizer has no merge metadata"));
        };
        bpe::encode_with_cancellation(input, metadata, on_cancellation_poll)
    }

    fn longest_token_prefix(&self, input: &str) -> Option<(usize, &str)> {
        self.tokens
            .iter()
            .enumerate()
            .filter_map(|(id, token)| {
                if token.is_empty() || !input.starts_with(token) {
                    None
                } else {
                    Some((id, token.as_str()))
                }
            })
            .max_by_key(|(_, token)| token.len())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenizerModel {
    Llama,
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    Normal,
    Unknown,
    Control,
    UserDefined,
    Unused,
    Byte,
    Other(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenizationControl {
    Continue,
    Cancel,
}

impl TokenType {
    fn from_gguf(value: i32) -> Self {
        match value {
            1 => Self::Normal,
            2 => Self::Unknown,
            3 => Self::Control,
            4 => Self::UserDefined,
            5 => Self::Unused,
            6 => Self::Byte,
            other => Self::Other(other),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenizerError {
    message: String,
    kind: TokenizerErrorKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenizerErrorKind {
    Other,
    IncompleteUtf8,
}

impl TokenizerError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: TokenizerErrorKind::Other,
        }
    }

    pub(crate) fn cancelled() -> Self {
        Self::new("tokenization cancelled")
    }

    pub(crate) fn incomplete_utf8(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: TokenizerErrorKind::IncompleteUtf8,
        }
    }

    pub fn is_incomplete_utf8(&self) -> bool {
        self.kind == TokenizerErrorKind::IncompleteUtf8
    }
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TokenizerError {}

impl From<GgufError> for TokenizerError {
    fn from(error: GgufError) -> Self {
        Self::new(error.to_string())
    }
}

fn metadata_string<'a>(file: &'a GgufFile, key: &str) -> Result<&'a str, TokenizerError> {
    match file.metadata.get(key) {
        Some(MetadataValue::String(value)) => Ok(value),
        Some(other) => Err(TokenizerError::new(format!(
            "{key} must be a string, found {other:?}"
        ))),
        None => Err(TokenizerError::new(format!("missing metadata {key}"))),
    }
}

fn metadata_string_array(file: &GgufFile, key: &str) -> Result<Vec<String>, TokenizerError> {
    match file.metadata.get(key) {
        Some(MetadataValue::Array {
            value_type: MetadataValueType::String,
            values,
        }) => values
            .iter()
            .map(|value| match value {
                MetadataValue::String(token) => Ok(token.clone()),
                other => Err(TokenizerError::new(format!(
                    "{key} contains non-string array value {other:?}"
                ))),
            })
            .collect(),
        Some(other) => Err(TokenizerError::new(format!(
            "{key} must be a string array, found {other:?}"
        ))),
        None => Err(TokenizerError::new(format!("missing metadata {key}"))),
    }
}

fn metadata_optional_string_array(
    file: &GgufFile,
    key: &str,
) -> Result<Vec<String>, TokenizerError> {
    match file.metadata.get(key) {
        Some(_) => metadata_string_array(file, key),
        None => Ok(Vec::new()),
    }
}

fn metadata_i32_array(file: &GgufFile, key: &str) -> Result<Option<Vec<i32>>, TokenizerError> {
    match file.metadata.get(key) {
        Some(MetadataValue::Array {
            value_type: MetadataValueType::Int32,
            values,
        }) => values
            .iter()
            .map(|value| match value {
                MetadataValue::Int32(token_type) => Ok(*token_type),
                other => Err(TokenizerError::new(format!(
                    "{key} contains non-int32 array value {other:?}"
                ))),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(other) => Err(TokenizerError::new(format!(
            "{key} must be an int32 array, found {other:?}"
        ))),
        None => Ok(None),
    }
}

fn metadata_optional_usize(file: &GgufFile, key: &str) -> Result<Option<usize>, TokenizerError> {
    let Some(value) = file.metadata.get(key) else {
        return Ok(None);
    };
    let value = match value {
        MetadataValue::UInt32(value) => u64::from(*value),
        MetadataValue::UInt64(value) => *value,
        other => {
            return Err(TokenizerError::new(format!(
                "{key} must be a uint32 or uint64, found {other:?}"
            )));
        }
    };
    usize::try_from(value)
        .map(Some)
        .map_err(|_| TokenizerError::new(format!("{key} value {value} does not fit usize")))
}
