use super::GgufError;

#[derive(Clone, Debug, PartialEq)]
/// A typed value from the GGUF metadata table.
pub enum MetadataValue {
    /// An unsigned 8-bit integer.
    UInt8(u8),
    /// A signed 8-bit integer.
    Int8(i8),
    /// An unsigned 16-bit integer.
    UInt16(u16),
    /// A signed 16-bit integer.
    Int16(i16),
    /// An unsigned 32-bit integer.
    UInt32(u32),
    /// A signed 32-bit integer.
    Int32(i32),
    /// A 32-bit floating-point number.
    Float32(f32),
    /// A Boolean value.
    Bool(bool),
    /// A UTF-8 string.
    String(String),
    /// A homogeneous array of metadata values.
    Array {
        /// The declared element type.
        value_type: MetadataValueType,
        /// The decoded elements in file order.
        values: Vec<MetadataValue>,
    },
    /// An unsigned 64-bit integer.
    UInt64(u64),
    /// A signed 64-bit integer.
    Int64(i64),
    /// A 64-bit floating-point number.
    Float64(f64),
}

impl MetadataValue {
    pub(super) fn as_count(&self) -> Result<u64, GgufError> {
        match self {
            Self::UInt32(value) => Ok(u64::from(*value)),
            Self::UInt64(value) => Ok(*value),
            other => Err(GgufError::new(format!(
                "metadata value {other:?} is not a supported count"
            ))),
        }
    }

    pub(super) fn as_f32(&self) -> Result<f32, GgufError> {
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
/// The type tag attached to a GGUF metadata value.
pub enum MetadataValueType {
    /// Unsigned 8-bit integer.
    UInt8 = 0,
    /// Signed 8-bit integer.
    Int8 = 1,
    /// Unsigned 16-bit integer.
    UInt16 = 2,
    /// Signed 16-bit integer.
    Int16 = 3,
    /// Unsigned 32-bit integer.
    UInt32 = 4,
    /// Signed 32-bit integer.
    Int32 = 5,
    /// 32-bit floating-point number.
    Float32 = 6,
    /// Boolean value.
    Bool = 7,
    /// UTF-8 string.
    String = 8,
    /// Homogeneous array.
    Array = 9,
    /// Unsigned 64-bit integer.
    UInt64 = 10,
    /// Signed 64-bit integer.
    Int64 = 11,
    /// 64-bit floating-point number.
    Float64 = 12,
}

impl MetadataValueType {
    pub(super) fn from_u32(value: u32) -> Result<Self, GgufError> {
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
/// A GGML tensor storage type declared by a GGUF tensor descriptor.
///
/// Parsing recognizes every listed type. Byte-range validation reports an
/// error for formats whose block sizing is not implemented by this crate.
pub enum GgmlType {
    /// IEEE 754 single-precision values.
    F32 = 0,
    /// IEEE 754 half-precision values.
    F16 = 1,
    /// GGML Q4_0 quantized blocks.
    Q4_0 = 2,
    /// GGML Q4_1 quantized blocks.
    Q4_1 = 3,
    /// GGML Q5_0 quantized blocks.
    Q5_0 = 6,
    /// GGML Q5_1 quantized blocks.
    Q5_1 = 7,
    /// GGML Q8_0 quantized blocks.
    Q8_0 = 8,
    /// GGML Q8_1 quantized blocks.
    Q8_1 = 9,
    /// GGML Q2_K quantized blocks.
    Q2K = 10,
    /// GGML Q3_K quantized blocks.
    Q3K = 11,
    /// GGML Q4_K quantized blocks.
    Q4K = 12,
    /// GGML Q5_K quantized blocks.
    Q5K = 13,
    /// GGML Q6_K quantized blocks.
    Q6K = 14,
    /// GGML Q8_K quantized blocks.
    Q8K = 15,
    /// GGML IQ2_XXS quantized blocks.
    IQ2Xxs = 16,
    /// GGML IQ2_XS quantized blocks.
    IQ2Xs = 17,
    /// GGML IQ3_XXS quantized blocks.
    IQ3Xxs = 18,
    /// GGML IQ1_S quantized blocks.
    IQ1S = 19,
    /// GGML IQ4_NL quantized blocks.
    IQ4Nl = 20,
    /// GGML IQ3_S quantized blocks.
    IQ3S = 21,
    /// GGML IQ2_S quantized blocks.
    IQ2S = 22,
    /// GGML IQ4_XS quantized blocks.
    IQ4Xs = 23,
    /// Signed 8-bit integers.
    I8 = 24,
    /// Signed 16-bit integers.
    I16 = 25,
    /// Signed 32-bit integers.
    I32 = 26,
    /// Signed 64-bit integers.
    I64 = 27,
    /// IEEE 754 double-precision values.
    F64 = 28,
    /// GGML IQ1_M quantized blocks.
    IQ1M = 29,
    /// Brain floating-point 16-bit values.
    BF16 = 30,
    /// GGML ternary TQ1_0 quantized blocks.
    TQ1_0 = 34,
    /// GGML ternary TQ2_0 quantized blocks.
    TQ2_0 = 35,
    /// GGML MXFP4 quantized blocks.
    Mxfp4 = 39,
}

impl GgmlType {
    pub(super) fn from_u32(value: u32) -> Result<Self, GgufError> {
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

    pub(super) fn storage_bytes(self, element_count: u64) -> Result<u64, GgufError> {
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
