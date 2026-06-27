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
        GgmlType::Q5_0 => q5_0_values_from_le_bytes(&tensor.name, slice),
        GgmlType::Q4K => q4_k_values_from_le_bytes(&tensor.name, slice),
        other => Err(InferenceError::new(format!(
            "tensor {} has type {:?}; expected F32, F16, BF16, Q8_0, Q5_0, or Q4K",
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

fn q5_0_values_from_le_bytes(name: &str, slice: &[u8]) -> Result<Vec<f32>, InferenceError> {
    const Q5_0_BLOCK_BYTES: usize = 22;
    const Q5_0_BLOCK_VALUES: usize = 32;

    if !slice.len().is_multiple_of(Q5_0_BLOCK_BYTES) {
        return Err(InferenceError::new(format!(
            "tensor {name} byte length {} is not divisible by Q5_0 block size {Q5_0_BLOCK_BYTES}",
            slice.len()
        )));
    }

    let mut values = Vec::with_capacity(slice.len() / Q5_0_BLOCK_BYTES * Q5_0_BLOCK_VALUES);
    for block in slice.chunks_exact(Q5_0_BLOCK_BYTES) {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let high_bits = u32::from_le_bytes([block[2], block[3], block[4], block[5]]);
        let quants = &block[6..];

        for (index, quant) in quants.iter().enumerate() {
            let high = ((high_bits >> index) << 4) as u8 & 0x10;
            let signed = i32::from((quant & 0x0f) | high) - 16;
            values.push(scale * signed as f32);
        }

        for (index, quant) in quants.iter().enumerate() {
            let high = (high_bits >> (index + 12)) as u8 & 0x10;
            let signed = i32::from((quant >> 4) | high) - 16;
            values.push(scale * signed as f32);
        }
    }
    Ok(values)
}

fn q4_k_values_from_le_bytes(name: &str, slice: &[u8]) -> Result<Vec<f32>, InferenceError> {
    const Q4_K_BLOCK_BYTES: usize = 144;
    const Q4_K_BLOCK_VALUES: usize = 256;

    if !slice.len().is_multiple_of(Q4_K_BLOCK_BYTES) {
        return Err(InferenceError::new(format!(
            "tensor {name} byte length {} is not divisible by Q4K block size {Q4_K_BLOCK_BYTES}",
            slice.len()
        )));
    }

    let mut values = Vec::with_capacity(slice.len() / Q4_K_BLOCK_BYTES * Q4_K_BLOCK_VALUES);
    for block in slice.chunks_exact(Q4_K_BLOCK_BYTES) {
        let d = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let dmin = f16_bits_to_f32(u16::from_le_bytes([block[2], block[3]]));
        let scales = &block[4..16];
        let quants = &block[16..];
        let mut scale_index = 0usize;

        for quant_chunk in quants.chunks_exact(32) {
            let (scale_low, min_low) = q4_k_scale_min(scale_index, scales);
            let (scale_high, min_high) = q4_k_scale_min(scale_index + 1, scales);
            let d_low = d * f32::from(scale_low);
            let d_high = d * f32::from(scale_high);
            let min_low = dmin * f32::from(min_low);
            let min_high = dmin * f32::from(min_high);

            for quant in quant_chunk {
                values.push(d_low * f32::from(quant & 0x0f) - min_low);
            }
            for quant in quant_chunk {
                values.push(d_high * f32::from(quant >> 4) - min_high);
            }
            scale_index += 2;
        }
    }
    Ok(values)
}

fn q4_k_scale_min(index: usize, scales: &[u8]) -> (u8, u8) {
    if index < 4 {
        (scales[index] & 63, scales[index + 4] & 63)
    } else {
        (
            (scales[index + 4] & 0x0f) | ((scales[index - 4] >> 6) << 4),
            (scales[index + 4] >> 4) | ((scales[index] >> 6) << 4),
        )
    }
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

#[cfg(test)]
mod tests {
    use super::{f16_bits_to_f32, q5_0_values_from_le_bytes};

    #[test]
    fn q5_0_decoder_reconstructs_signed_block_values() -> Result<(), super::InferenceError> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0xffff_0000u32.to_le_bytes());
        for index in 0..16u8 {
            block.push(index | (index << 4));
        }

        let values = q5_0_values_from_le_bytes("q5", &block)?;

        let expected = (-16..16).map(|value| value as f32).collect::<Vec<_>>();
        assert_eq!(f16_bits_to_f32(0x3c00), 1.0);
        assert_eq!(values, expected);
        Ok(())
    }
}
