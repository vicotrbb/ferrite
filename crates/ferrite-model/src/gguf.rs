use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;

mod reader;
mod types;

pub use crate::gguf_config::{
    ArchitectureExecution, AttentionProjectionLayout, FeedForwardProjectionLayout, LlamaConfig,
    ModelArchitecture, ModelConfig, RotaryPairing, TransformerConfig,
};
use reader::Reader;
pub use types::{GgmlType, MetadataValue, MetadataValueType};

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
const GGUF_VERSION: u32 = 3;
const DEFAULT_ALIGNMENT: u64 = 32;
const MAX_METADATA_ENTRIES: u64 = 65_536;
const MAX_TENSOR_COUNT: usize = 65_536;

#[derive(Clone, Debug, PartialEq)]
/// A validated view of one GGUF v3 file.
pub struct GgufFile {
    /// The GGUF format version, currently always `3` after successful parsing.
    pub version: u32,
    /// The byte alignment used by the tensor data section.
    pub alignment: u64,
    /// Metadata values keyed by their GGUF names.
    pub metadata: BTreeMap<String, MetadataValue>,
    /// Tensor descriptors in file order.
    pub tensors: Vec<TensorInfo>,
}

impl GgufFile {
    /// Finds a tensor descriptor by its exact GGUF name.
    pub fn tensor(&self, name: &str) -> Option<&TensorInfo> {
        self.tensors.iter().find(|tensor| tensor.name == name)
    }

    /// Returns the model-provided Jinja chat template when present.
    ///
    /// Ferrite exposes the source string for a bounded renderer and does not
    /// execute arbitrary template code in the model parser.
    ///
    /// # Errors
    ///
    /// Returns an error when `tokenizer.chat_template` is present with a type
    /// other than string.
    pub fn chat_template(&self) -> Result<Option<&str>, GgufError> {
        match self.metadata.get("tokenizer.chat_template") {
            Some(MetadataValue::String(template)) => Ok(Some(template)),
            Some(other) => Err(GgufError::new(format!(
                "tokenizer.chat_template must be a string, found {other:?}"
            ))),
            None => Ok(None),
        }
    }

    /// Builds a validated configuration for the architecture named in metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when the architecture is unsupported, required metadata
    /// is absent or has the wrong type, or transformer dimensions are invalid.
    pub fn model_config(&self) -> Result<ModelConfig, GgufError> {
        let architecture = self.architecture()?;
        let config = self.transformer_config(architecture)?;

        match architecture {
            ModelArchitecture::Llama => Ok(ModelConfig::Llama(config)),
            ModelArchitecture::Qwen2 => Ok(ModelConfig::Qwen2(config)),
            ModelArchitecture::Phi3 => Ok(ModelConfig::Phi3(config)),
        }
    }

    /// Builds a validated Llama configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when the file is not a Llama model or its required
    /// configuration metadata is missing or invalid.
    pub fn llama_config(&self) -> Result<LlamaConfig, GgufError> {
        let architecture = self.architecture()?;
        if architecture != ModelArchitecture::Llama {
            return Err(GgufError::new(format!(
                "expected llama architecture, found {}",
                architecture.metadata_value()
            )));
        }

        self.transformer_config(ModelArchitecture::Llama)
    }

    /// Returns the supported architecture declared by `general.architecture`.
    ///
    /// # Errors
    ///
    /// Returns an error when the metadata is missing, malformed, or names an
    /// architecture that Ferrite does not support.
    pub fn architecture(&self) -> Result<ModelArchitecture, GgufError> {
        match self.metadata.get("general.architecture") {
            Some(MetadataValue::String(architecture)) => {
                ModelArchitecture::from_metadata(architecture).ok_or_else(|| {
                    GgufError::new(format!("unsupported architecture {architecture}"))
                })
            }
            _ => Err(GgufError::new("missing general.architecture metadata")),
        }
    }

    fn transformer_config(
        &self,
        architecture: ModelArchitecture,
    ) -> Result<LlamaConfig, GgufError> {
        let prefix = architecture.metadata_prefix();
        let embedding_length =
            self.required_nonzero_count(&format!("{prefix}.embedding_length"))?;
        let attention_head_count =
            self.required_nonzero_count(&format!("{prefix}.attention.head_count"))?;
        validate_embedding_head_layout(prefix, embedding_length, attention_head_count)?;
        let default_head_dimension = embedding_length / attention_head_count;
        let key_length = self
            .optional_nonzero_count(&format!("{prefix}.attention.key_length"))?
            .unwrap_or(default_head_dimension);
        let value_length = self
            .optional_nonzero_count(&format!("{prefix}.attention.value_length"))?
            .unwrap_or(default_head_dimension);
        let rope_dimension_count =
            match self.optional_nonzero_count(&format!("{prefix}.rope.dimension_count"))? {
                Some(value) => value,
                None if matches!(
                    architecture,
                    ModelArchitecture::Qwen2 | ModelArchitecture::Phi3
                ) =>
                {
                    key_length
                }
                None => {
                    return Err(GgufError::new(format!(
                        "missing required metadata {prefix}.rope.dimension_count"
                    )));
                }
            };
        let attention_head_count_kv = self
            .optional_nonzero_count(&format!("{prefix}.attention.head_count_kv"))?
            .unwrap_or(attention_head_count);
        validate_attention_head_layout(prefix, attention_head_count, attention_head_count_kv)?;
        validate_rope_dimension_layout(prefix, key_length, rope_dimension_count)?;
        let rope_freq_base = self.optional_f32(&format!("{prefix}.rope.freq_base"))?;
        validate_rope_freq_base(prefix, rope_freq_base)?;
        let attention_layer_norm_rms_epsilon =
            self.optional_f32(&format!("{prefix}.attention.layer_norm_rms_epsilon"))?;
        validate_attention_layer_norm_rms_epsilon(prefix, attention_layer_norm_rms_epsilon)?;

        Ok(LlamaConfig {
            architecture,
            context_length: self.required_nonzero_count(&format!("{prefix}.context_length"))?,
            embedding_length,
            block_count: self.required_nonzero_count(&format!("{prefix}.block_count"))?,
            feed_forward_length: self
                .required_nonzero_count(&format!("{prefix}.feed_forward_length"))?,
            attention_head_count,
            attention_head_count_kv,
            key_length,
            value_length,
            attention_layer_norm_rms_epsilon,
            rope_dimension_count,
            rope_freq_base,
        })
    }

    fn required_count(&self, key: &str) -> Result<u64, GgufError> {
        self.optional_count(key)?
            .ok_or_else(|| GgufError::new(format!("missing required metadata {key}")))
    }

    fn required_nonzero_count(&self, key: &str) -> Result<u64, GgufError> {
        let value = self.required_count(key)?;
        if value == 0 {
            return Err(GgufError::new(format!("{key} must be greater than zero")));
        }
        Ok(value)
    }

    fn optional_nonzero_count(&self, key: &str) -> Result<Option<u64>, GgufError> {
        self.optional_count(key)?
            .map(|value| {
                if value == 0 {
                    Err(GgufError::new(format!("{key} must be greater than zero")))
                } else {
                    Ok(value)
                }
            })
            .transpose()
    }

    fn optional_count(&self, key: &str) -> Result<Option<u64>, GgufError> {
        self.metadata
            .get(key)
            .map(MetadataValue::as_count)
            .transpose()
    }

    fn optional_f32(&self, key: &str) -> Result<Option<f32>, GgufError> {
        self.metadata
            .get(key)
            .map(MetadataValue::as_f32)
            .transpose()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// The storage description and validated byte range of one GGUF tensor.
pub struct TensorInfo {
    /// The exact tensor name stored in the GGUF directory.
    pub name: String,
    /// Tensor dimensions in GGUF order.
    pub dimensions: Vec<u64>,
    /// The GGML element or quantization type.
    pub ty: GgmlType,
    /// The tensor's byte offset relative to the tensor data section.
    pub relative_offset: u64,
    /// The absolute half-open byte range within the source file.
    pub data_range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// An error produced while parsing or validating GGUF data.
pub struct GgufError {
    message: String,
}

impl GgufError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for GgufError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GgufError {}

/// Parses and validates a complete GGUF v3 byte slice.
///
/// The parser validates the header, metadata keys and types, tensor names,
/// alignment, dimensions, storage sizes, arithmetic overflow, and every tensor
/// byte range before returning a [`GgufFile`].
///
/// # Errors
///
/// Returns an error for malformed input, unsupported versions or value types,
/// invalid metadata, unsupported tensor sizing, or out-of-bounds tensor data.
pub fn parse_gguf(bytes: &[u8]) -> Result<GgufFile, GgufError> {
    let mut reader = Reader::new(bytes);

    let magic = reader.read_exact(4)?;
    if magic != GGUF_MAGIC {
        return Err(GgufError::new("invalid GGUF magic"));
    }

    let version = reader.read_u32()?;
    if version != GGUF_VERSION {
        return Err(GgufError::new(format!(
            "unsupported GGUF version {version}; expected {GGUF_VERSION}"
        )));
    }

    let tensor_count = usize_from_u64(reader.read_u64()?, "tensor count")?;
    let metadata_count = reader.read_u64()?;
    if tensor_count > MAX_TENSOR_COUNT {
        return Err(GgufError::new(format!(
            "tensor count {tensor_count} exceeds parser limit {MAX_TENSOR_COUNT}"
        )));
    }
    if metadata_count > MAX_METADATA_ENTRIES {
        return Err(GgufError::new(format!(
            "metadata entry count {metadata_count} exceeds parser limit {MAX_METADATA_ENTRIES}"
        )));
    }

    let mut metadata = BTreeMap::new();
    for _ in 0..metadata_count {
        let key = reader.read_string_with_limit(u16::MAX as usize, "metadata key")?;
        validate_metadata_key(&key)?;
        let value_type = MetadataValueType::from_u32(reader.read_u32()?)?;
        let value = reader.read_metadata_value(value_type)?;
        if metadata.insert(key.clone(), value).is_some() {
            return Err(GgufError::new(format!("duplicate metadata key {key}")));
        }
    }

    let alignment = read_alignment(&metadata)?;
    let mut raw_tensors = Vec::new();
    raw_tensors
        .try_reserve_exact(tensor_count)
        .map_err(|error| {
            GgufError::new(format!(
                "failed to reserve tensor directory capacity for {tensor_count} entries: {error}"
            ))
        })?;
    let mut tensor_names = BTreeSet::new();
    for _ in 0..tensor_count {
        let raw_tensor = reader.read_tensor_info(alignment)?;
        if !tensor_names.insert(raw_tensor.name.clone()) {
            return Err(GgufError::new(format!(
                "duplicate tensor name {}",
                raw_tensor.name
            )));
        }
        raw_tensors.push(raw_tensor);
    }

    let data_start = align_offset(
        u64::try_from(reader.position())
            .map_err(|_error| GgufError::new("reader position overflow"))?,
        alignment,
    )?;
    let file_len =
        u64::try_from(bytes.len()).map_err(|_error| GgufError::new("file length overflow"))?;
    if data_start > file_len && !raw_tensors.is_empty() {
        return Err(GgufError::new("tensor data start is past end of file"));
    }

    let mut tensors = Vec::new();
    tensors
        .try_reserve_exact(raw_tensors.len())
        .map_err(|error| {
            GgufError::new(format!(
                "failed to reserve validated tensor capacity for {} entries: {error}",
                raw_tensors.len()
            ))
        })?;
    for raw in raw_tensors {
        let element_count = raw.element_count()?;
        let byte_len = raw.ty.storage_bytes(element_count)?;
        let absolute_start = data_start
            .checked_add(raw.relative_offset)
            .ok_or_else(|| GgufError::new("tensor absolute offset overflow"))?;
        let absolute_end = absolute_start
            .checked_add(byte_len)
            .ok_or_else(|| GgufError::new("tensor end offset overflow"))?;
        if absolute_end > file_len {
            return Err(GgufError::new(format!(
                "tensor {} range exceeds file length",
                raw.name
            )));
        }

        tensors.push(TensorInfo {
            name: raw.name,
            dimensions: raw.dimensions,
            ty: raw.ty,
            relative_offset: raw.relative_offset,
            data_range: usize_from_u64(absolute_start, "tensor start")?
                ..usize_from_u64(absolute_end, "tensor end")?,
        });
    }

    Ok(GgufFile {
        version,
        alignment,
        metadata,
        tensors,
    })
}

fn read_alignment(metadata: &BTreeMap<String, MetadataValue>) -> Result<u64, GgufError> {
    let alignment = match metadata.get("general.alignment") {
        None => DEFAULT_ALIGNMENT,
        Some(MetadataValue::UInt32(value)) => u64::from(*value),
        Some(MetadataValue::UInt64(value)) => *value,
        Some(other) => {
            return Err(GgufError::new(format!(
                "general.alignment must be uint32 or uint64, found {other:?}"
            )));
        }
    };

    if alignment == 0 || alignment % 8 != 0 {
        return Err(GgufError::new(format!(
            "general.alignment {alignment} must be a non-zero multiple of 8"
        )));
    }

    Ok(alignment)
}

fn validate_embedding_head_layout(
    prefix: &str,
    embedding_length: u64,
    attention_head_count: u64,
) -> Result<(), GgufError> {
    if !embedding_length.is_multiple_of(attention_head_count) {
        return Err(GgufError::new(format!(
            "{prefix}.embedding_length {embedding_length} must be divisible by {prefix}.attention.head_count {attention_head_count}"
        )));
    }

    Ok(())
}

fn validate_attention_head_layout(
    prefix: &str,
    attention_head_count: u64,
    attention_head_count_kv: u64,
) -> Result<(), GgufError> {
    if !attention_head_count.is_multiple_of(attention_head_count_kv) {
        return Err(GgufError::new(format!(
            "{prefix}.attention.head_count {attention_head_count} must be divisible by {prefix}.attention.head_count_kv {attention_head_count_kv}"
        )));
    }

    Ok(())
}

fn validate_rope_dimension_layout(
    prefix: &str,
    key_length: u64,
    rope_dimension_count: u64,
) -> Result<(), GgufError> {
    if !rope_dimension_count.is_multiple_of(2) {
        return Err(GgufError::new(format!(
            "{prefix}.rope.dimension_count {rope_dimension_count} must be even"
        )));
    }

    if rope_dimension_count > key_length {
        return Err(GgufError::new(format!(
            "{prefix}.rope.dimension_count {rope_dimension_count} must be less than or equal to {prefix}.attention.key_length {key_length}"
        )));
    }

    Ok(())
}

fn validate_rope_freq_base(prefix: &str, rope_freq_base: Option<f32>) -> Result<(), GgufError> {
    if let Some(value) = rope_freq_base {
        if !value.is_finite() {
            return Err(GgufError::new(format!(
                "{prefix}.rope.freq_base must be finite"
            )));
        }

        if value <= 0.0 {
            return Err(GgufError::new(format!(
                "{prefix}.rope.freq_base {value} must be positive"
            )));
        }
    }

    Ok(())
}

fn validate_attention_layer_norm_rms_epsilon(
    prefix: &str,
    epsilon: Option<f32>,
) -> Result<(), GgufError> {
    if let Some(value) = epsilon {
        if !value.is_finite() || value < 0.0 {
            return Err(GgufError::new(format!(
                "{prefix}.attention.layer_norm_rms_epsilon must be finite and non-negative"
            )));
        }
    }

    Ok(())
}

fn align_offset(offset: u64, alignment: u64) -> Result<u64, GgufError> {
    let remainder = offset % alignment;
    if remainder == 0 {
        return Ok(offset);
    }

    offset
        .checked_add(alignment - remainder)
        .ok_or_else(|| GgufError::new("alignment offset overflow"))
}

fn validate_metadata_key(key: &str) -> Result<(), GgufError> {
    if key.is_empty() || key.len() > u16::MAX as usize || !key.is_ascii() {
        return Err(GgufError::new(
            "metadata key is not valid GGUF lower_snake_case hierarchy",
        ));
    }

    let valid = key.split('.').all(|segment| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    });

    if valid {
        Ok(())
    } else {
        Err(GgufError::new(
            "metadata key is not valid GGUF lower_snake_case hierarchy",
        ))
    }
}

pub(super) fn usize_from_u64(value: u64, context: &str) -> Result<usize, GgufError> {
    usize::try_from(value)
        .map_err(|_error| GgufError::new(format!("{context} does not fit in usize")))
}
