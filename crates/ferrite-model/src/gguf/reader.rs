use super::{GgmlType, GgufError, MetadataValue, MetadataValueType, usize_from_u64};

const MAX_TENSOR_DIMENSIONS: usize = 4;
const MAX_METADATA_NESTING_DEPTH: usize = 64;
const MAX_METADATA_VALUE_NODES: usize = 1 << 20;
const MAX_METADATA_STRING_BYTES: usize = 16 << 20;
const MAX_TOTAL_STRING_BYTES: usize = 256 << 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RawTensorInfo {
    pub(super) name: String,
    pub(super) dimensions: Vec<u64>,
    pub(super) ty: GgmlType,
    pub(super) relative_offset: u64,
}

impl RawTensorInfo {
    pub(super) fn element_count(&self) -> Result<u64, GgufError> {
        self.dimensions
            .iter()
            .try_fold(1u64, |accumulator, dimension| {
                accumulator.checked_mul(*dimension).ok_or_else(|| {
                    GgufError::new(format!("tensor {} element count overflow", self.name))
                })
            })
    }
}

pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
    metadata_value_nodes: usize,
    decoded_string_bytes: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            cursor: 0,
            metadata_value_nodes: 0,
            decoded_string_bytes: 0,
        }
    }

    pub(super) fn position(&self) -> usize {
        self.cursor
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }

    pub(super) fn read_exact(&mut self, len: usize) -> Result<&'a [u8], GgufError> {
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

    pub(super) fn read_u32(&mut self) -> Result<u32, GgufError> {
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

    pub(super) fn read_u64(&mut self) -> Result<u64, GgufError> {
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

    pub(super) fn read_string(&mut self) -> Result<String, GgufError> {
        self.read_string_with_limit(MAX_METADATA_STRING_BYTES, "metadata string")
    }

    pub(super) fn read_string_with_limit(
        &mut self,
        max_bytes: usize,
        context: &str,
    ) -> Result<String, GgufError> {
        let len = usize_from_u64(self.read_u64()?, "string length")?;
        if len > max_bytes {
            return Err(GgufError::new(format!(
                "{context} length {len} exceeds parser limit {max_bytes}"
            )));
        }
        let decoded_string_bytes = self
            .decoded_string_bytes
            .checked_add(len)
            .ok_or_else(|| GgufError::new("decoded GGUF string byte count overflow"))?;
        if decoded_string_bytes > MAX_TOTAL_STRING_BYTES {
            return Err(GgufError::new(format!(
                "decoded GGUF strings exceed parser limit {MAX_TOTAL_STRING_BYTES} bytes"
            )));
        }
        let bytes = self.read_exact(len)?;
        let text = std::str::from_utf8(bytes)
            .map_err(|error| GgufError::new(format!("invalid UTF-8 string: {error}")))?;
        let mut value = String::new();
        value.try_reserve_exact(len).map_err(|error| {
            GgufError::new(format!(
                "failed to reserve {len} bytes for {context}: {error}"
            ))
        })?;
        value.push_str(text);
        self.decoded_string_bytes = decoded_string_bytes;
        Ok(value)
    }

    pub(super) fn read_metadata_value(
        &mut self,
        value_type: MetadataValueType,
    ) -> Result<MetadataValue, GgufError> {
        self.read_metadata_value_at_depth(value_type, 0)
    }

    fn read_metadata_value_at_depth(
        &mut self,
        value_type: MetadataValueType,
        depth: usize,
    ) -> Result<MetadataValue, GgufError> {
        if value_type == MetadataValueType::Array && depth >= MAX_METADATA_NESTING_DEPTH {
            return Err(GgufError::new(format!(
                "metadata array nesting exceeds parser limit {MAX_METADATA_NESTING_DEPTH}"
            )));
        }
        self.metadata_value_nodes = self
            .metadata_value_nodes
            .checked_add(1)
            .ok_or_else(|| GgufError::new("decoded metadata value count overflow"))?;
        if self.metadata_value_nodes > MAX_METADATA_VALUE_NODES {
            return Err(GgufError::new(format!(
                "decoded metadata values exceed parser limit {MAX_METADATA_VALUE_NODES}"
            )));
        }

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
                let available_nodes = MAX_METADATA_VALUE_NODES - self.metadata_value_nodes;
                if len > available_nodes {
                    return Err(GgufError::new(format!(
                        "metadata array length {len} exceeds remaining decoded-value budget {available_nodes}"
                    )));
                }
                let minimum_bytes = len
                    .checked_mul(minimum_metadata_value_bytes(item_type))
                    .ok_or_else(|| GgufError::new("metadata array byte count overflow"))?;
                if minimum_bytes > self.remaining() {
                    return Err(GgufError::new(format!(
                        "metadata array requires at least {minimum_bytes} bytes, but only {} remain",
                        self.remaining()
                    )));
                }
                let mut values = Vec::new();
                values.try_reserve_exact(len).map_err(|error| {
                    GgufError::new(format!(
                        "failed to reserve metadata array capacity for {len} values: {error}"
                    ))
                })?;
                for _ in 0..len {
                    values.push(self.read_metadata_value_at_depth(item_type, depth + 1)?);
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

    pub(super) fn read_tensor_info(&mut self, alignment: u64) -> Result<RawTensorInfo, GgufError> {
        let name = self.read_string_with_limit(64, "tensor name")?;

        let dimension_count = usize_from_u64(u64::from(self.read_u32()?), "dimension count")?;
        if dimension_count == 0 {
            return Err(GgufError::new(format!(
                "tensor {name} must have at least one dimension"
            )));
        }
        if dimension_count > MAX_TENSOR_DIMENSIONS {
            return Err(GgufError::new(format!(
                "tensor {name} has {dimension_count} dimensions; maximum supported is {MAX_TENSOR_DIMENSIONS}"
            )));
        }

        let mut dimensions = Vec::with_capacity(dimension_count);
        for _ in 0..dimension_count {
            let dimension = self.read_u64()?;
            if dimension == 0 {
                return Err(GgufError::new(format!("tensor {name} has zero dimension")));
            }
            dimensions.push(dimension);
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

fn minimum_metadata_value_bytes(value_type: MetadataValueType) -> usize {
    match value_type {
        MetadataValueType::UInt8 | MetadataValueType::Int8 | MetadataValueType::Bool => 1,
        MetadataValueType::UInt16 | MetadataValueType::Int16 => 2,
        MetadataValueType::UInt32 | MetadataValueType::Int32 | MetadataValueType::Float32 => 4,
        MetadataValueType::String
        | MetadataValueType::UInt64
        | MetadataValueType::Int64
        | MetadataValueType::Float64 => 8,
        MetadataValueType::Array => 12,
    }
}
