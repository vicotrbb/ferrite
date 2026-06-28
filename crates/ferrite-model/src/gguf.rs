use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

pub use crate::gguf_config::{LlamaConfig, ModelArchitecture, ModelConfig};

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
        let embedding_length = self.required_count(&format!("{prefix}.embedding_length"))?;
        let attention_head_count =
            self.required_count(&format!("{prefix}.attention.head_count"))?;
        let key_length = self
            .optional_count(&format!("{prefix}.attention.key_length"))?
            .unwrap_or(embedding_length / attention_head_count);
        let value_length = self
            .optional_count(&format!("{prefix}.attention.value_length"))?
            .unwrap_or(embedding_length / attention_head_count);
        let rope_dimension_count =
            match self.optional_count(&format!("{prefix}.rope.dimension_count"))? {
                Some(value) => value,
                None if architecture == ModelArchitecture::Qwen2 => key_length,
                None => {
                    return Err(GgufError::new(format!(
                        "missing required metadata {prefix}.rope.dimension_count"
                    )));
                }
            };

        Ok(LlamaConfig {
            architecture,
            context_length: self.required_count(&format!("{prefix}.context_length"))?,
            embedding_length,
            block_count: self.required_count(&format!("{prefix}.block_count"))?,
            feed_forward_length: self.required_count(&format!("{prefix}.feed_forward_length"))?,
            attention_head_count,
            attention_head_count_kv: self
                .optional_count(&format!("{prefix}.attention.head_count_kv"))?
                .unwrap_or(attention_head_count),
            key_length,
            value_length,
            attention_layer_norm_rms_epsilon: self
                .optional_f32(&format!("{prefix}.attention.layer_norm_rms_epsilon"))?,
            rope_dimension_count,
            rope_freq_base: self.optional_f32(&format!("{prefix}.rope.freq_base"))?,
        })
    }

    fn required_count(&self, key: &str) -> Result<u64, GgufError> {
        self.optional_count(key)?
            .ok_or_else(|| GgufError::new(format!("missing required metadata {key}")))
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

#[derive(Clone, Debug, PartialEq)]
pub enum MetadataValue {
    UInt8(u8),
    Int8(i8),
    UInt16(u16),
    Int16(i16),
    UInt32(u32),
    Int32(i32),
    Float32(f32),
    Bool(bool),
    String(String),
    Array {
        value_type: MetadataValueType,
        values: Vec<MetadataValue>,
    },
    UInt64(u64),
    Int64(i64),
    Float64(f64),
}

impl MetadataValue {
    fn as_count(&self) -> Result<u64, GgufError> {
        match self {
            Self::UInt32(value) => Ok(u64::from(*value)),
            Self::UInt64(value) => Ok(*value),
            other => Err(GgufError::new(format!(
                "metadata value {other:?} is not a supported count"
            ))),
        }
    }

    fn as_f32(&self) -> Result<f32, GgufError> {
        match self {
            Self::Float32(value) => Ok(*value),
            other => Err(GgufError::new(format!(
                "metadata value {other:?} is not a supported float32"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum MetadataValueType {
    UInt8 = 0,
    Int8 = 1,
    UInt16 = 2,
    Int16 = 3,
    UInt32 = 4,
    Int32 = 5,
    Float32 = 6,
    Bool = 7,
    String = 8,
    Array = 9,
    UInt64 = 10,
    Int64 = 11,
    Float64 = 12,
}

impl MetadataValueType {
    fn from_u32(value: u32) -> Result<Self, GgufError> {
        match value {
            0 => Ok(Self::UInt8),
            1 => Ok(Self::Int8),
            2 => Ok(Self::UInt16),
            3 => Ok(Self::Int16),
            4 => Ok(Self::UInt32),
            5 => Ok(Self::Int32),
            6 => Ok(Self::Float32),
            7 => Ok(Self::Bool),
            8 => Ok(Self::String),
            9 => Ok(Self::Array),
            10 => Ok(Self::UInt64),
            11 => Ok(Self::Int64),
            12 => Ok(Self::Float64),
            other => Err(GgufError::new(format!(
                "unknown GGUF metadata value type {other}"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum GgmlType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q5_0 = 6,
    Q5_1 = 7,
    Q8_0 = 8,
    Q8_1 = 9,
    Q2K = 10,
    Q3K = 11,
    Q4K = 12,
    Q5K = 13,
    Q6K = 14,
    Q8K = 15,
    IQ2Xxs = 16,
    IQ2Xs = 17,
    IQ3Xxs = 18,
    IQ1S = 19,
    IQ4Nl = 20,
    IQ3S = 21,
    IQ2S = 22,
    IQ4Xs = 23,
    I8 = 24,
    I16 = 25,
    I32 = 26,
    I64 = 27,
    F64 = 28,
    IQ1M = 29,
    BF16 = 30,
    TQ1_0 = 34,
    TQ2_0 = 35,
    Mxfp4 = 39,
}

impl GgmlType {
    fn from_u32(value: u32) -> Result<Self, GgufError> {
        match value {
            0 => Ok(Self::F32),
            1 => Ok(Self::F16),
            2 => Ok(Self::Q4_0),
            3 => Ok(Self::Q4_1),
            6 => Ok(Self::Q5_0),
            7 => Ok(Self::Q5_1),
            8 => Ok(Self::Q8_0),
            9 => Ok(Self::Q8_1),
            10 => Ok(Self::Q2K),
            11 => Ok(Self::Q3K),
            12 => Ok(Self::Q4K),
            13 => Ok(Self::Q5K),
            14 => Ok(Self::Q6K),
            15 => Ok(Self::Q8K),
            16 => Ok(Self::IQ2Xxs),
            17 => Ok(Self::IQ2Xs),
            18 => Ok(Self::IQ3Xxs),
            19 => Ok(Self::IQ1S),
            20 => Ok(Self::IQ4Nl),
            21 => Ok(Self::IQ3S),
            22 => Ok(Self::IQ2S),
            23 => Ok(Self::IQ4Xs),
            24 => Ok(Self::I8),
            25 => Ok(Self::I16),
            26 => Ok(Self::I32),
            27 => Ok(Self::I64),
            28 => Ok(Self::F64),
            29 => Ok(Self::IQ1M),
            30 => Ok(Self::BF16),
            34 => Ok(Self::TQ1_0),
            35 => Ok(Self::TQ2_0),
            39 => Ok(Self::Mxfp4),
            other => Err(GgufError::new(format!("unknown GGML tensor type {other}"))),
        }
    }

    fn block_shape(self) -> Result<(u64, u64), GgufError> {
        match self {
            Self::F32 => Ok((1, 4)),
            Self::F16 | Self::BF16 => Ok((1, 2)),
            Self::Q4_0 => Ok((32, 18)),
            Self::Q4_1 => Ok((32, 20)),
            Self::Q5_0 => Ok((32, 22)),
            Self::Q5_1 => Ok((32, 24)),
            Self::Q8_0 => Ok((32, 34)),
            Self::Q8_1 => Ok((32, 36)),
            Self::Q2K => Ok((256, 84)),
            Self::Q3K => Ok((256, 142)),
            Self::Q4K => Ok((256, 144)),
            Self::Q5K => Ok((256, 176)),
            Self::Q6K => Ok((256, 210)),
            Self::Q8K => Ok((256, 292)),
            Self::I8 => Ok((1, 1)),
            Self::I16 => Ok((1, 2)),
            Self::I32 => Ok((1, 4)),
            Self::I64 => Ok((1, 8)),
            Self::F64 => Ok((1, 8)),
            Self::IQ2Xxs
            | Self::IQ2Xs
            | Self::IQ3Xxs
            | Self::IQ1S
            | Self::IQ4Nl
            | Self::IQ3S
            | Self::IQ2S
            | Self::IQ4Xs
            | Self::IQ1M
            | Self::TQ1_0
            | Self::TQ2_0
            | Self::Mxfp4 => Err(GgufError::new(format!(
                "tensor byte sizing for {self:?} is not implemented"
            ))),
        }
    }

    fn storage_bytes(self, element_count: u64) -> Result<u64, GgufError> {
        let (block_size, type_size) = self.block_shape()?;
        if !element_count.is_multiple_of(block_size) {
            return Err(GgufError::new(format!(
                "tensor with {element_count} elements is not divisible by {self:?} block size {block_size}"
            )));
        }

        element_count
            .checked_div(block_size)
            .and_then(|blocks| blocks.checked_mul(type_size))
            .ok_or_else(|| GgufError::new("tensor byte size overflow"))
    }
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
        metadata.insert(key, value);
    }

    let alignment = read_alignment(&metadata)?;
    let mut raw_tensors = Vec::with_capacity(usize_from_u64(tensor_count, "tensor count")?);
    for _ in 0..tensor_count {
        raw_tensors.push(reader.read_tensor_info(alignment)?);
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

fn usize_from_u64(value: u64, context: &str) -> Result<usize, GgufError> {
    usize::try_from(value).map_err(|_| GgufError::new(format!("{context} does not fit in usize")))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RawTensorInfo {
    name: String,
    dimensions: Vec<u64>,
    ty: GgmlType,
    relative_offset: u64,
}

impl RawTensorInfo {
    fn element_count(&self) -> Result<u64, GgufError> {
        self.dimensions
            .iter()
            .try_fold(1u64, |accumulator, dimension| {
                accumulator.checked_mul(*dimension).ok_or_else(|| {
                    GgufError::new(format!("tensor {} element count overflow", self.name))
                })
            })
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn position(&self) -> usize {
        self.cursor
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], GgufError> {
        let end = self
            .cursor
            .checked_add(len)
            .ok_or_else(|| GgufError::new("reader cursor overflow"))?;
        if end > self.bytes.len() {
            return Err(GgufError::new("unexpected end of GGUF data"));
        }

        let value = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, GgufError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_i8(&mut self) -> Result<i8, GgufError> {
        Ok(i8::from_le_bytes([self.read_u8()?]))
    }

    fn read_u16(&mut self) -> Result<u16, GgufError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_i16(&mut self) -> Result<i16, GgufError> {
        let bytes = self.read_exact(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, GgufError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_i32(&mut self) -> Result<i32, GgufError> {
        let bytes = self.read_exact(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_f32(&mut self) -> Result<f32, GgufError> {
        let bytes = self.read_exact(4)?;
        Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, GgufError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_i64(&mut self) -> Result<i64, GgufError> {
        let bytes = self.read_exact(8)?;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_f64(&mut self) -> Result<f64, GgufError> {
        let bytes = self.read_exact(8)?;
        Ok(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_bool(&mut self) -> Result<bool, GgufError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(GgufError::new(format!(
                "invalid GGUF bool value {other}; expected 0 or 1"
            ))),
        }
    }

    fn read_string(&mut self) -> Result<String, GgufError> {
        let len = usize_from_u64(self.read_u64()?, "string length")?;
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| GgufError::new(format!("invalid UTF-8 string: {error}")))
    }

    fn read_metadata_value(
        &mut self,
        value_type: MetadataValueType,
    ) -> Result<MetadataValue, GgufError> {
        match value_type {
            MetadataValueType::UInt8 => Ok(MetadataValue::UInt8(self.read_u8()?)),
            MetadataValueType::Int8 => Ok(MetadataValue::Int8(self.read_i8()?)),
            MetadataValueType::UInt16 => Ok(MetadataValue::UInt16(self.read_u16()?)),
            MetadataValueType::Int16 => Ok(MetadataValue::Int16(self.read_i16()?)),
            MetadataValueType::UInt32 => Ok(MetadataValue::UInt32(self.read_u32()?)),
            MetadataValueType::Int32 => Ok(MetadataValue::Int32(self.read_i32()?)),
            MetadataValueType::Float32 => Ok(MetadataValue::Float32(self.read_f32()?)),
            MetadataValueType::Bool => Ok(MetadataValue::Bool(self.read_bool()?)),
            MetadataValueType::String => Ok(MetadataValue::String(self.read_string()?)),
            MetadataValueType::Array => {
                let item_type = MetadataValueType::from_u32(self.read_u32()?)?;
                let len = usize_from_u64(self.read_u64()?, "array length")?;
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(self.read_metadata_value(item_type)?);
                }
                Ok(MetadataValue::Array {
                    value_type: item_type,
                    values,
                })
            }
            MetadataValueType::UInt64 => Ok(MetadataValue::UInt64(self.read_u64()?)),
            MetadataValueType::Int64 => Ok(MetadataValue::Int64(self.read_i64()?)),
            MetadataValueType::Float64 => Ok(MetadataValue::Float64(self.read_f64()?)),
        }
    }

    fn read_tensor_info(&mut self, alignment: u64) -> Result<RawTensorInfo, GgufError> {
        let name = self.read_string()?;
        if name.len() > 64 {
            return Err(GgufError::new(format!(
                "tensor name {name} exceeds GGUF 64-byte limit"
            )));
        }

        let dimension_count = usize_from_u64(u64::from(self.read_u32()?), "dimension count")?;
        let mut dimensions = Vec::with_capacity(dimension_count);
        for _ in 0..dimension_count {
            dimensions.push(self.read_u64()?);
        }

        let ty = GgmlType::from_u32(self.read_u32()?)?;
        let relative_offset = self.read_u64()?;
        if relative_offset % alignment != 0 {
            return Err(GgufError::new(format!(
                "tensor offset {relative_offset} is not aligned to {alignment}"
            )));
        }

        Ok(RawTensorInfo {
            name,
            dimensions,
            ty,
            relative_offset,
        })
    }
}
