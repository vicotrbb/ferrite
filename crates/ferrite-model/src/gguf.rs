use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;

mod reader;
mod types;

pub use crate::gguf_config::{LlamaConfig, ModelArchitecture, ModelConfig};
use reader::Reader;
pub use types::{GgmlType, MetadataValue, MetadataValueType};

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
const GGUF_VERSION: u32 = 3;
const DEFAULT_ALIGNMENT: u64 = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct GgufFile {
    pub version: u32,
    pub alignment: u64,
    pub metadata: BTreeMap<String, MetadataValue>,
    pub tensors: Vec<TensorInfo>,
}

impl GgufFile {
    pub fn tensor(&self, name: &str) -> Option<&TensorInfo> {
        self.tensors.iter().find(|tensor| tensor.name == name)
    }

    pub fn model_config(&self) -> Result<ModelConfig, GgufError> {
        let architecture = self.model_architecture()?;
        let config = self.transformer_config(architecture)?;

        match architecture {
            ModelArchitecture::Llama => Ok(ModelConfig::Llama(config)),
            ModelArchitecture::Qwen2 => Ok(ModelConfig::Qwen2(config)),
        }
    }

    pub fn llama_config(&self) -> Result<LlamaConfig, GgufError> {
        let architecture = self.model_architecture()?;
        if architecture != ModelArchitecture::Llama {
            return Err(GgufError::new(format!(
                "expected llama architecture, found {}",
                architecture.metadata_value()
            )));
        }

        self.transformer_config(ModelArchitecture::Llama)
    }

    fn model_architecture(&self) -> Result<ModelArchitecture, GgufError> {
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
                None if architecture == ModelArchitecture::Qwen2 => key_length,
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
pub struct TensorInfo {
    pub name: String,
    pub dimensions: Vec<u64>,
    pub ty: GgmlType,
    pub relative_offset: u64,
    pub data_range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

    let tensor_count = reader.read_u64()?;
    let metadata_count = reader.read_u64()?;

    let mut metadata = BTreeMap::new();
    for _ in 0..metadata_count {
        let key = reader.read_string()?;
        validate_metadata_key(&key)?;
        let value_type = MetadataValueType::from_u32(reader.read_u32()?)?;
        let value = reader.read_metadata_value(value_type)?;
        if metadata.insert(key.clone(), value).is_some() {
            return Err(GgufError::new(format!("duplicate metadata key {key}")));
        }
    }

    let alignment = read_alignment(&metadata)?;
    let mut raw_tensors = Vec::with_capacity(usize_from_u64(tensor_count, "tensor count")?);
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
        u64::try_from(reader.position()).map_err(|_| GgufError::new("reader position overflow"))?,
        alignment,
    )?;
    let file_len =
        u64::try_from(bytes.len()).map_err(|_| GgufError::new("file length overflow"))?;
    if data_start > file_len && !raw_tensors.is_empty() {
        return Err(GgufError::new("tensor data start is past end of file"));
    }

    let mut tensors = Vec::with_capacity(raw_tensors.len());
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
    usize::try_from(value).map_err(|_| GgufError::new(format!("{context} does not fit in usize")))
}
