mod bpe;
mod spm;

use crate::gguf::{GgufError, GgufFile, MetadataValue, MetadataValueType};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
/// A tokenizer constructed from a GGUF tokenizer metadata table.
///
/// Tokenizers with merge metadata use byte-pair encoding. Llama tokenizers
/// with scored vocabulary metadata use SentencePiece-compatible tokenization.
/// Minimal tokenizers without either form use deterministic longest-prefix
/// atomic token matching.
pub struct GgufTokenizer {
    model: TokenizerModel,
    tokens: Vec<String>,
    token_types: Vec<TokenType>,
    merges: Vec<String>,
    bpe_metadata: Option<bpe::BpeMetadata>,
    spm_metadata: Option<spm::SpmMetadata>,
    special_tokens: Vec<(usize, String)>,
    rstrip_special_tokens: Vec<bool>,
    bos_token_id: Option<usize>,
    eos_token_id: Option<usize>,
    end_of_generation_token_ids: Vec<usize>,
    add_bos_token: bool,
    add_eos_token: bool,
    add_space_prefix: bool,
}

impl GgufTokenizer {
    /// Builds a tokenizer from validated GGUF metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when required tokenizer metadata is missing, has the
    /// wrong type, contains inconsistent lengths, or has invalid BPE merges.
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
        let scores = metadata_f32_array(file, "tokenizer.ggml.scores")?;
        let spm_metadata = if merges.is_empty() && matches!(model, TokenizerModel::Llama) {
            scores
                .as_deref()
                .map(|scores| spm::SpmMetadata::new(&tokens, &token_types, scores))
                .transpose()?
        } else {
            None
        };
        let has_spm_metadata = spm_metadata.is_some();

        let special_tokens = tokens
            .iter()
            .enumerate()
            .filter(|(id, token)| {
                !token.is_empty()
                    && matches!(
                        token_types[*id],
                        TokenType::Control | TokenType::UserDefined
                    )
            })
            .map(|(id, token)| (id, token.clone()))
            .collect::<Vec<_>>();
        let architecture = match file.metadata.get("general.architecture") {
            Some(MetadataValue::String(architecture)) => Some(architecture.as_str()),
            _ => None,
        };
        let is_phi3 = architecture == Some("phi3");
        let rstrip_special_tokens = tokens
            .iter()
            .enumerate()
            .map(|(id, token)| {
                is_phi3
                    && matches!(token_types[id], TokenType::Control | TokenType::UserDefined)
                    && !matches!(token.as_str(), "<unk>" | "<s>" | "<|endoftext|>")
            })
            .collect();
        let bos_token_id =
            metadata_optional_token_id(file, "tokenizer.ggml.bos_token_id", tokens.len())?;
        let eos_token_id =
            metadata_optional_token_id(file, "tokenizer.ggml.eos_token_id", tokens.len())?;
        let eot_token_id =
            metadata_optional_token_id(file, "tokenizer.ggml.eot_token_id", tokens.len())?;
        let eom_token_id =
            metadata_optional_token_id(file, "tokenizer.ggml.eom_token_id", tokens.len())?;
        let mut end_of_generation_token_ids = [eos_token_id, eot_token_id, eom_token_id]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        end_of_generation_token_ids.extend(
            tokens
                .iter()
                .zip(&token_types)
                .enumerate()
                .filter(|(_, (token, token_type))| {
                    matches!(token_type, TokenType::Control | TokenType::UserDefined)
                        && is_model_native_end_of_generation_token(architecture, token)
                })
                .map(|(token_id, _)| token_id),
        );
        end_of_generation_token_ids.sort_unstable();
        end_of_generation_token_ids.dedup();

        Ok(Self {
            model,
            tokens,
            token_types,
            merges,
            bpe_metadata,
            spm_metadata,
            special_tokens,
            rstrip_special_tokens,
            bos_token_id,
            eos_token_id,
            end_of_generation_token_ids,
            add_bos_token: metadata_optional_bool(file, "tokenizer.ggml.add_bos_token")?
                .unwrap_or(has_spm_metadata),
            add_eos_token: metadata_optional_bool(file, "tokenizer.ggml.add_eos_token")?
                .unwrap_or(false),
            add_space_prefix: metadata_optional_bool(file, "tokenizer.ggml.add_space_prefix")?
                .unwrap_or(has_spm_metadata),
        })
    }

    /// Returns the tokenizer model identifier.
    pub fn model(&self) -> TokenizerModel {
        self.model.clone()
    }

    /// Returns the vocabulary size.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Returns `true` when the vocabulary contains no tokens.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Returns the token text for `id`, or `None` when the ID is out of range.
    pub fn token(&self, id: usize) -> Option<&str> {
        self.tokens.get(id).map(String::as_str)
    }

    /// Returns the exact decoded bytes contributed by one token.
    ///
    /// Unlike [`Self::decode`], this method permits a token that contains only
    /// part of a UTF-8 sequence. Bounded output grammars use it to validate
    /// candidate tokens without treating tokenizer-internal byte mappings as
    /// literal text.
    ///
    /// # Errors
    ///
    /// Returns an error when `id` is outside the vocabulary or a BPE token
    /// contains a character outside the configured byte alphabet.
    pub fn token_bytes(&self, id: usize) -> Result<Vec<u8>, TokenizerError> {
        if !self.merges.is_empty() {
            return bpe::decode_bytes(&[id], &self.tokens);
        }
        if self.spm_metadata.is_some() {
            return spm::decode_token_bytes(id, &self.tokens, &self.token_types);
        }
        self.tokens
            .get(id)
            .map(|token| token.as_bytes().to_vec())
            .ok_or_else(|| TokenizerError::new(format!("token id {id} is out of bounds")))
    }

    /// Returns the GGUF token classification for `id`.
    pub fn token_type(&self, id: usize) -> Option<TokenType> {
        self.token_types.get(id).copied()
    }

    /// Returns the configured end-of-sequence token ID, when present.
    pub fn eos_token_id(&self) -> Option<usize> {
        self.eos_token_id
    }

    /// Returns every token that terminates model generation.
    ///
    /// In addition to the GGUF EOS, EOT, and EOM metadata fields, Ferrite
    /// recognizes the bounded turn terminators used by its supported chat
    /// template families. Some model artifacts, including Phi-3, declare a
    /// document-level EOS token while using a different token to end an
    /// assistant turn.
    pub fn end_of_generation_token_ids(&self) -> &[usize] {
        &self.end_of_generation_token_ids
    }

    /// Returns whether `token_id` terminates generation for this tokenizer.
    pub fn is_end_of_generation_token(&self, token_id: usize) -> bool {
        self.end_of_generation_token_ids
            .binary_search(&token_id)
            .is_ok()
    }

    /// Returns the configured beginning-of-sequence token ID, when present.
    pub fn bos_token_id(&self) -> Option<usize> {
        self.bos_token_id
    }

    /// Returns whether configured encoding prepends the beginning token.
    pub fn adds_bos_token(&self) -> bool {
        self.add_bos_token
    }

    /// Returns whether configured encoding appends the end token.
    pub fn adds_eos_token(&self) -> bool {
        self.add_eos_token
    }

    /// Decodes token IDs to UTF-8 text.
    ///
    /// # Errors
    ///
    /// Returns an error when an ID is out of range or BPE byte tokens do not
    /// form valid UTF-8.
    pub fn decode(&self, ids: &[usize]) -> Result<String, TokenizerError> {
        if !self.merges.is_empty() {
            return bpe::decode(ids, &self.tokens);
        }

        if self.spm_metadata.is_some() {
            let mut bytes = Vec::new();
            for id in ids {
                bytes.extend(spm::decode_token_bytes(
                    *id,
                    &self.tokens,
                    &self.token_types,
                )?);
            }
            return String::from_utf8(bytes).map_err(|error| {
                let message = format!("SentencePiece decoded invalid UTF-8: {error}");
                if error.utf8_error().error_len().is_none() {
                    TokenizerError::incomplete_utf8(message)
                } else {
                    TokenizerError::new(message)
                }
            });
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

    /// Decodes token IDs when they form complete UTF-8.
    ///
    /// Returns `Ok(None)` when another token may complete a partial UTF-8 byte
    /// sequence.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid token IDs or decoding failures other than
    /// an incomplete UTF-8 suffix.
    pub fn decode_if_complete(&self, ids: &[usize]) -> Result<Option<String>, TokenizerError> {
        match self.decode(ids) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.is_incomplete_utf8() => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Encodes text with longest-prefix atomic token matching.
    ///
    /// # Errors
    ///
    /// Returns an error when no vocabulary token matches the remaining input.
    pub fn encode_atomic(&self, input: &str) -> Result<Vec<usize>, TokenizerError> {
        self.encode_atomic_with_cancellation(input, || TokenizationControl::Continue)
    }

    /// Encodes atomically while polling a caller-provided cancellation hook.
    ///
    /// The hook is called before work begins and before every token match.
    ///
    /// # Errors
    ///
    /// Returns an error when cancellation is requested or no vocabulary token
    /// matches the remaining input.
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

    /// Encodes text with the tokenizer's configured algorithm.
    ///
    /// GGUF tokenizers with merge metadata use BPE. Scored llama vocabularies
    /// use SentencePiece-compatible tokenization, while minimal tokenizers
    /// without either form use atomic longest-prefix matching.
    ///
    /// # Errors
    ///
    /// Returns an error when the input cannot be represented by the vocabulary
    /// or the tokenizer metadata is invalid.
    pub fn encode(&self, input: &str) -> Result<Vec<usize>, TokenizerError> {
        self.encode_with_cancellation(input, || TokenizationControl::Continue)
    }

    /// Encodes with the configured algorithm and cancellation polling.
    ///
    /// # Errors
    ///
    /// Returns an error when cancellation is requested, the input cannot be
    /// represented, or required tokenizer metadata is unavailable.
    pub fn encode_with_cancellation(
        &self,
        input: &str,
        mut on_cancellation_poll: impl FnMut() -> TokenizationControl,
    ) -> Result<Vec<usize>, TokenizerError> {
        let mut token_ids = if !self.merges.is_empty() {
            self.encode_bpe_with_cancellation(input, &mut on_cancellation_poll)?
        } else if self.spm_metadata.is_some() {
            self.encode_spm_with_cancellation(input, &mut on_cancellation_poll)?
        } else {
            self.encode_atomic_with_cancellation(input, &mut on_cancellation_poll)?
        };
        if self.add_bos_token {
            let bos_token_id = self.bos_token_id.ok_or_else(|| {
                TokenizerError::new(
                    "tokenizer.ggml.add_bos_token is true but bos_token_id is missing",
                )
            })?;
            if token_ids.first() != Some(&bos_token_id) {
                token_ids.insert(0, bos_token_id);
            }
        }
        if self.add_eos_token {
            let eos_token_id = self.eos_token_id.ok_or_else(|| {
                TokenizerError::new(
                    "tokenizer.ggml.add_eos_token is true but eos_token_id is missing",
                )
            })?;
            if token_ids.last() != Some(&eos_token_id) {
                token_ids.push(eos_token_id);
            }
        }
        Ok(token_ids)
    }

    fn encode_spm_with_cancellation(
        &self,
        input: &str,
        mut on_cancellation_poll: impl FnMut() -> TokenizationControl,
    ) -> Result<Vec<usize>, TokenizerError> {
        let metadata = self.spm_metadata.as_ref().ok_or_else(|| {
            TokenizerError::new("SentencePiece tokenizer has no scored vocabulary metadata")
        })?;
        if on_cancellation_poll() == TokenizationControl::Cancel {
            return Err(TokenizerError::cancelled());
        }

        let mut output = Vec::new();
        let mut cursor = 0usize;
        let mut previous_was_special = true;
        while cursor < input.len() {
            if on_cancellation_poll() == TokenizationControl::Cancel {
                return Err(TokenizerError::cancelled());
            }
            let suffix = &input[cursor..];
            let special = self.next_special_token(suffix);
            let ordinary_len = special.map_or(suffix.len(), |(offset, _, _)| offset);
            if ordinary_len > 0 {
                let ordinary = &suffix[..ordinary_len];
                let prefixed;
                let ordinary = if self.add_space_prefix && previous_was_special {
                    prefixed = format!(" {ordinary}");
                    prefixed.as_str()
                } else {
                    ordinary
                };
                output.extend(spm::encode_ordinary_with_cancellation(
                    ordinary,
                    metadata,
                    &mut on_cancellation_poll,
                )?);
                cursor += ordinary_len;
                previous_was_special = false;
                continue;
            }
            let Some((_, token_id, token)) = special else {
                break;
            };
            output.push(token_id);
            cursor += token.len();
            if self.rstrip_special_tokens[token_id] {
                while cursor < input.len() {
                    let Some(character) = input[cursor..].chars().next() else {
                        break;
                    };
                    if !character.is_ascii_whitespace() {
                        break;
                    }
                    cursor += character.len_utf8();
                }
            }
            previous_was_special = true;
        }
        Ok(output)
    }

    /// Encodes text using byte-pair encoding metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when merge metadata is absent or the input cannot be
    /// represented by the BPE vocabulary.
    pub fn encode_bpe(&self, input: &str) -> Result<Vec<usize>, TokenizerError> {
        self.encode_bpe_with_cancellation(input, || TokenizationControl::Continue)
    }

    /// Encodes with BPE while polling a caller-provided cancellation hook.
    ///
    /// # Errors
    ///
    /// Returns an error when cancellation is requested, merge metadata is
    /// absent, or the input cannot be represented by the BPE vocabulary.
    pub fn encode_bpe_with_cancellation(
        &self,
        input: &str,
        mut on_cancellation_poll: impl FnMut() -> TokenizationControl,
    ) -> Result<Vec<usize>, TokenizerError> {
        let Some(metadata) = &self.bpe_metadata else {
            return Err(TokenizerError::new("BPE tokenizer has no merge metadata"));
        };
        let mut output = Vec::new();
        let mut cursor = 0;
        while cursor < input.len() {
            if on_cancellation_poll() == TokenizationControl::Cancel {
                return Err(TokenizerError::cancelled());
            }
            let suffix = &input[cursor..];
            let special = self.next_special_token(suffix);
            let ordinary_len = special.map_or(suffix.len(), |(offset, _, _)| offset);
            if ordinary_len > 0 {
                output.extend(bpe::encode_with_cancellation(
                    &suffix[..ordinary_len],
                    metadata,
                    &mut on_cancellation_poll,
                )?);
                cursor += ordinary_len;
                continue;
            }
            let Some((_, token_id, token)) = special else {
                break;
            };
            output.push(token_id);
            cursor += token.len();
        }
        Ok(output)
    }

    fn next_special_token<'a>(&'a self, input: &str) -> Option<(usize, usize, &'a str)> {
        self.special_tokens
            .iter()
            .filter_map(|(token_id, token)| {
                input
                    .find(token)
                    .map(|offset| (offset, *token_id, token.as_str()))
            })
            .min_by(|left, right| {
                left.0
                    .cmp(&right.0)
                    .then_with(|| right.2.len().cmp(&left.2.len()))
                    .then_with(|| left.1.cmp(&right.1))
            })
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
/// The tokenizer model name declared by GGUF metadata.
pub enum TokenizerModel {
    /// The `llama` tokenizer model.
    Llama,
    /// Any tokenizer model identifier that Ferrite does not classify specially.
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A token classification from `tokenizer.ggml.token_type` metadata.
pub enum TokenType {
    /// A normal vocabulary token.
    Normal,
    /// The tokenizer's unknown-token marker.
    Unknown,
    /// A control token.
    Control,
    /// A user-defined token.
    UserDefined,
    /// A vocabulary entry reserved as unused.
    Unused,
    /// A token representing a raw byte.
    Byte,
    /// An unrecognized GGUF token-type code.
    Other(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The result requested by a tokenization cancellation hook.
pub enum TokenizationControl {
    /// Continue tokenization.
    Continue,
    /// Stop tokenization and return an error.
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

fn is_model_native_end_of_generation_token(architecture: Option<&str>, token: &str) -> bool {
    match architecture {
        Some("llama") => matches!(token, "</s>" | "<|eot_id|>"),
        Some("qwen2") => token == "<|im_end|>",
        Some("phi3") => token == "<|end|>",
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// An error produced while building, encoding with, or decoding a tokenizer.
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

    /// Returns `true` when decoding ended with a potentially completable UTF-8
    /// byte sequence.
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

fn metadata_f32_array(file: &GgufFile, key: &str) -> Result<Option<Vec<f32>>, TokenizerError> {
    match file.metadata.get(key) {
        Some(MetadataValue::Array {
            value_type: MetadataValueType::Float32,
            values,
        }) => values
            .iter()
            .map(|value| match value {
                MetadataValue::Float32(score) if score.is_finite() => Ok(*score),
                MetadataValue::Float32(score) => Err(TokenizerError::new(format!(
                    "{key} contains non-finite score {score}"
                ))),
                other => Err(TokenizerError::new(format!(
                    "{key} contains non-float32 array value {other:?}"
                ))),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(other) => Err(TokenizerError::new(format!(
            "{key} must be a float32 array, found {other:?}"
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
        .map_err(|_error| TokenizerError::new(format!("{key} value {value} does not fit usize")))
}

fn metadata_optional_token_id(
    file: &GgufFile,
    key: &str,
    vocabulary_size: usize,
) -> Result<Option<usize>, TokenizerError> {
    let token_id = metadata_optional_usize(file, key)?;
    if let Some(token_id) = token_id.filter(|token_id| *token_id >= vocabulary_size) {
        return Err(TokenizerError::new(format!(
            "{key} value {token_id} is outside vocabulary size {vocabulary_size}"
        )));
    }
    Ok(token_id)
}

fn metadata_optional_bool(file: &GgufFile, key: &str) -> Result<Option<bool>, TokenizerError> {
    match file.metadata.get(key) {
        Some(MetadataValue::Bool(value)) => Ok(Some(*value)),
        Some(other) => Err(TokenizerError::new(format!(
            "{key} must be a bool, found {other:?}"
        ))),
        None => Ok(None),
    }
}
