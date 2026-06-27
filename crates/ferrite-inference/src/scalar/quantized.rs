use super::InferenceError;

pub(super) const Q4_K_BLOCK_VALUES: usize = 256;
pub(super) const Q4_K_BLOCK_BYTES: usize = 144;
pub(super) const Q5_0_BLOCK_VALUES: usize = 32;
pub(super) const Q5_0_BLOCK_BYTES: usize = 22;
pub(super) const Q8_0_BLOCK_VALUES: usize = 32;
pub(super) const Q8_0_BLOCK_BYTES: usize = 34;

pub(super) fn q4_k_storage_bytes(value_count: usize) -> Result<usize, InferenceError> {
    storage_bytes(value_count, Q4_K_BLOCK_VALUES, Q4_K_BLOCK_BYTES, "Q4_K")
}

pub(super) fn q5_0_row_bytes(cols: usize) -> Result<usize, InferenceError> {
    storage_bytes(cols, Q5_0_BLOCK_VALUES, Q5_0_BLOCK_BYTES, "Q5_0")
}

pub(super) fn q8_0_row_bytes(cols: usize) -> Result<usize, InferenceError> {
    storage_bytes(cols, Q8_0_BLOCK_VALUES, Q8_0_BLOCK_BYTES, "Q8_0")
}

pub(super) fn decode_q4_k_values(
    bytes: &[u8],
    value_count: usize,
) -> Result<Vec<f32>, InferenceError> {
    let expected = q4_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q4_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }

    let mut values = Vec::with_capacity(value_count);
    for block in bytes.chunks_exact(Q4_K_BLOCK_BYTES) {
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

pub(super) fn q4_k_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    if vector.len() != cols {
        return Err(InferenceError::new(format!(
            "matrix columns {cols} do not match vector length {}",
            vector.len()
        )));
    }
    let value_count = rows
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("Q4_K matrix value count overflow"))?;
    let values = decode_q4_k_values(bytes, value_count)?;
    let mut output = vec![0.0; rows];

    for (index, value) in values.iter().enumerate() {
        let row = index / cols;
        let col = index % cols;
        output[row] += value * vector[col];
    }

    Ok(output)
}

pub(super) fn decode_q5_0_row(bytes: &[u8], cols: usize) -> Result<Vec<f32>, InferenceError> {
    let expected = q5_0_row_bytes(cols)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q5_0 row byte length {} does not match {expected}",
            bytes.len()
        )));
    }

    let mut values = Vec::with_capacity(cols);
    for block in bytes.chunks_exact(Q5_0_BLOCK_BYTES) {
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

pub(super) fn decode_q8_0_row(bytes: &[u8], cols: usize) -> Result<Vec<f32>, InferenceError> {
    let expected = q8_0_row_bytes(cols)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q8_0 row byte length {} does not match {expected}",
            bytes.len()
        )));
    }

    let mut values = Vec::with_capacity(cols);
    for block in bytes.chunks_exact(Q8_0_BLOCK_BYTES) {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        for quantized in &block[2..] {
            values.push(scale * f32::from(*quantized as i8));
        }
    }
    Ok(values)
}

fn storage_bytes(
    value_count: usize,
    block_values: usize,
    block_bytes: usize,
    name: &str,
) -> Result<usize, InferenceError> {
    if !value_count.is_multiple_of(block_values) {
        return Err(InferenceError::new(format!(
            "{name} value count {value_count} must be divisible by {block_values}"
        )));
    }

    value_count
        .checked_div(block_values)
        .and_then(|blocks| blocks.checked_mul(block_bytes))
        .ok_or_else(|| InferenceError::new(format!("{name} byte length overflow")))
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
    use super::{q4_k_mul_vec, InferenceError};

    #[test]
    fn q4_k_mul_vec_accumulates_rows_without_full_row_decodes() -> Result<(), InferenceError> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
        block.extend_from_slice(&[0x11; 128]);

        let actual = q4_k_mul_vec(&block, 2, 128, &[1.0; 128])?;

        assert_eq!(actual, vec![128.0, 128.0]);
        Ok(())
    }
}
