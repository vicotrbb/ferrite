use super::InferenceError;
use ferrite_model::gguf::{GgmlType, TensorInfo};

pub(super) fn f32_values(tensor: &TensorInfo, bytes: &[u8]) -> Result<Vec<f32>, InferenceError> {
    let slice = bytes.get(tensor.data_range.clone()).ok_or_else(|| {
        InferenceError::new(format!("tensor {} byte range is invalid", tensor.name))
    })?;

    match tensor.ty {
        GgmlType::F32 => f32_values_from_le_bytes(&tensor.name, slice),
        GgmlType::F16 => f16_values_from_le_bytes(&tensor.name, slice),
        GgmlType::BF16 => bf16_values_from_le_bytes(&tensor.name, slice),
        GgmlType::Q8_0 => q8_0_values_from_le_bytes(&tensor.name, slice),
        other => Err(InferenceError::new(format!(
            "tensor {} has type {:?}; expected F32, F16, BF16, or Q8_0",
            tensor.name, other
        ))),
    }
}

fn f32_values_from_le_bytes(name: &str, slice: &[u8]) -> Result<Vec<f32>, InferenceError> {
    if !slice.len().is_multiple_of(4) {
        return Err(InferenceError::new(format!(
            "tensor {name} byte length {} is not divisible by 4",
            slice.len()
        )));
    }

    let mut values = Vec::with_capacity(slice.len() / 4);
    for chunk in slice.chunks_exact(4) {
        values.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(values)
}

fn f16_values_from_le_bytes(name: &str, slice: &[u8]) -> Result<Vec<f32>, InferenceError> {
    if !slice.len().is_multiple_of(2) {
        return Err(InferenceError::new(format!(
            "tensor {name} byte length {} is not divisible by 2",
            slice.len()
        )));
    }

    let mut values = Vec::with_capacity(slice.len() / 2);
    for chunk in slice.chunks_exact(2) {
        values.push(f16_bits_to_f32(u16::from_le_bytes([chunk[0], chunk[1]])));
    }
    Ok(values)
}

fn bf16_values_from_le_bytes(name: &str, slice: &[u8]) -> Result<Vec<f32>, InferenceError> {
    if !slice.len().is_multiple_of(2) {
        return Err(InferenceError::new(format!(
            "tensor {name} byte length {} is not divisible by 2",
            slice.len()
        )));
    }

    let mut values = Vec::with_capacity(slice.len() / 2);
    for chunk in slice.chunks_exact(2) {
        let bits = u32::from(u16::from_le_bytes([chunk[0], chunk[1]])) << 16;
        values.push(f32::from_bits(bits));
    }
    Ok(values)
}

fn q8_0_values_from_le_bytes(name: &str, slice: &[u8]) -> Result<Vec<f32>, InferenceError> {
    const Q8_0_BLOCK_BYTES: usize = 34;
    const Q8_0_BLOCK_VALUES: usize = 32;

    if !slice.len().is_multiple_of(Q8_0_BLOCK_BYTES) {
        return Err(InferenceError::new(format!(
            "tensor {name} byte length {} is not divisible by Q8_0 block size {Q8_0_BLOCK_BYTES}",
            slice.len()
        )));
    }

    let mut values = Vec::with_capacity(slice.len() / Q8_0_BLOCK_BYTES * Q8_0_BLOCK_VALUES);
    for block in slice.chunks_exact(Q8_0_BLOCK_BYTES) {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        for quantized in &block[2..] {
            values.push(scale * f32::from(*quantized as i8));
        }
    }
    Ok(values)
}

fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = ((bits & 0x8000) as u32) << 16;
    let exponent = ((bits >> 10) & 0x1f) as u32;
    let mantissa = (bits & 0x03ff) as u32;

    let f32_bits = match exponent {
        0 => {
            if mantissa == 0 {
                sign
            } else {
                let mut normalized_mantissa = mantissa;
                let mut exponent_adjust = -14i32;
                while normalized_mantissa & 0x0400 == 0 {
                    normalized_mantissa <<= 1;
                    exponent_adjust -= 1;
                }
                normalized_mantissa &= 0x03ff;
                let exponent_bits = ((exponent_adjust + 127) as u32) << 23;
                sign | exponent_bits | (normalized_mantissa << 13)
            }
        }
        0x1f => sign | 0x7f80_0000 | (mantissa << 13),
        _ => {
            let exponent_bits = (exponent + 112) << 23;
            sign | exponent_bits | (mantissa << 13)
        }
    };

    f32::from_bits(f32_bits)
}
