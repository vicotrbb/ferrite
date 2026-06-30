use super::{usize_from_u64, GgmlType, GgufError, MetadataValue, MetadataValueType};

const MAX_TENSOR_DIMENSIONS: usize = 4;

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
}

impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    pub(super) fn position(&self) -> usize {
        self.cursor
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
        let len = usize_from_u64(self.read_u64()?, "string length")?;
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| GgufError::new(format!("invalid UTF-8 string: {error}")))
    }

    pub(super) fn read_metadata_value(
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

    pub(super) fn read_tensor_info(&mut self, alignment: u64) -> Result<RawTensorInfo, GgufError> {
        let name = self.read_string()?;
        if name.len() > 64 {
            return Err(GgufError::new(format!(
                "tensor name {name} exceeds GGUF 64-byte limit"
            )));
        }

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
