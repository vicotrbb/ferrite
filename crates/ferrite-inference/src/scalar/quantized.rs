use super::{float::f16_bits_to_f32, InferenceError};

pub(super) use super::q5_0::{decode_q5_0_row, q5_0_mul_vec, q5_0_row_bytes, Q5_0_BLOCK_VALUES};
pub(super) use super::q8_0::{decode_q8_0_row, q8_0_mul_vec, q8_0_row_bytes, Q8_0_BLOCK_VALUES};

pub(super) const Q4_K_BLOCK_VALUES: usize = 256;
pub(super) const Q4_K_BLOCK_BYTES: usize = 144;
pub(super) const Q6_K_BLOCK_VALUES: usize = 256;
pub(super) const Q6_K_BLOCK_BYTES: usize = 210;
pub(super) fn q4_k_storage_bytes(value_count: usize) -> Result<usize, InferenceError> {
    storage_bytes(value_count, Q4_K_BLOCK_VALUES, Q4_K_BLOCK_BYTES, "Q4_K")
}

pub(super) fn q6_k_storage_bytes(value_count: usize) -> Result<usize, InferenceError> {
    storage_bytes(value_count, Q6_K_BLOCK_VALUES, Q6_K_BLOCK_BYTES, "Q6_K")
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
    let expected = q4_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q4_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }
    let mut output = vec![0.0; rows];

    for (block_index, block) in bytes.chunks_exact(Q4_K_BLOCK_BYTES).enumerate() {
        let value_offset = block_index
            .checked_mul(Q4_K_BLOCK_VALUES)
            .ok_or_else(|| InferenceError::new("Q4_K block value offset overflow"))?;
        accumulate_q4_k_block(block, value_offset, rows, cols, vector, &mut output)?;
    }

    Ok(output)
}

fn accumulate_q4_k_block(
    block: &[u8],
    value_offset: usize,
    rows: usize,
    cols: usize,
    vector: &[f32],
    output: &mut [f32],
) -> Result<(), InferenceError> {
    if block.len() != Q4_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q4_K block byte length {} does not match {Q4_K_BLOCK_BYTES}",
            block.len()
        )));
    }
    if output.len() != rows {
        return Err(InferenceError::new(format!(
            "Q4_K output rows {} do not match {rows}",
            output.len()
        )));
    }

    let d = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
    let dmin = f16_bits_to_f32(u16::from_le_bytes([block[2], block[3]]));
    let scales = &block[4..16];
    let quants = &block[16..];
    let mut scale_index = 0usize;
    let mut local_offset = 0usize;

    for quant_chunk in quants.chunks_exact(32) {
        let (scale_low, min_low) = q4_k_scale_min(scale_index, scales);
        let (scale_high, min_high) = q4_k_scale_min(scale_index + 1, scales);
        let d_low = d * f32::from(scale_low);
        let d_high = d * f32::from(scale_high);
        let min_low = dmin * f32::from(min_low);
        let min_high = dmin * f32::from(min_high);

        for quant in quant_chunk {
            accumulate_matrix_value(
                value_offset + local_offset,
                d_low * f32::from(quant & 0x0f) - min_low,
                cols,
                vector,
                output,
            )?;
            local_offset += 1;
        }
        for quant in quant_chunk {
            accumulate_matrix_value(
                value_offset + local_offset,
                d_high * f32::from(quant >> 4) - min_high,
                cols,
                vector,
                output,
            )?;
            local_offset += 1;
        }
        scale_index += 2;
    }

    Ok(())
}

fn accumulate_matrix_value(
    index: usize,
    value: f32,
    cols: usize,
    vector: &[f32],
    output: &mut [f32],
) -> Result<(), InferenceError> {
    let row = index / cols;
    let col = index % cols;
    let target = output
        .get_mut(row)
        .ok_or_else(|| InferenceError::new("quantized matrix row index out of bounds"))?;
    let vector_value = vector
        .get(col)
        .ok_or_else(|| InferenceError::new("quantized matrix column index out of bounds"))?;
    *target += value * vector_value;
    Ok(())
}

pub(super) fn q6_k_mul_vec(
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
        .ok_or_else(|| InferenceError::new("Q6_K matrix value count overflow"))?;
    let expected = q6_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q6_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }
    let mut output = vec![0.0; rows];

    for (block_index, block) in bytes.chunks_exact(Q6_K_BLOCK_BYTES).enumerate() {
        let value_offset = block_index
            .checked_mul(Q6_K_BLOCK_VALUES)
            .ok_or_else(|| InferenceError::new("Q6_K block value offset overflow"))?;
        accumulate_q6_k_block(block, value_offset, rows, cols, vector, &mut output)?;
    }

    Ok(output)
}

fn accumulate_q6_k_block(
    block: &[u8],
    value_offset: usize,
    rows: usize,
    cols: usize,
    vector: &[f32],
    output: &mut [f32],
) -> Result<(), InferenceError> {
    if block.len() != Q6_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q6_K block byte length {} does not match {Q6_K_BLOCK_BYTES}",
            block.len()
        )));
    }
    if output.len() != rows {
        return Err(InferenceError::new(format!(
            "Q6_K output rows {} do not match {rows}",
            output.len()
        )));
    }

    let low_bits = &block[0..128];
    let high_bits = &block[128..192];
    let scales = &block[192..208];
    let super_scale = f16_bits_to_f32(u16::from_le_bytes([block[208], block[209]]));

    for half in 0..2 {
        let value_base = half * 128;
        let low_base = half * 64;
        let high_base = half * 32;
        let scale_base = half * 8;

        for index in 0..32 {
            let scale_index = index / 16;
            let high = high_bits[high_base + index];
            let q1 = i32::from((low_bits[low_base + index] & 0x0f) | ((high & 3) << 4)) - 32;
            let q2 =
                i32::from((low_bits[low_base + index + 32] & 0x0f) | (((high >> 2) & 3) << 4)) - 32;
            let q3 = i32::from((low_bits[low_base + index] >> 4) | (((high >> 4) & 3) << 4)) - 32;
            let q4 =
                i32::from((low_bits[low_base + index + 32] >> 4) | (((high >> 6) & 3) << 4)) - 32;

            accumulate_matrix_value(
                value_offset + value_base + index,
                super_scale * f32::from(scales[scale_base + scale_index] as i8) * q1 as f32,
                cols,
                vector,
                output,
            )?;
            accumulate_matrix_value(
                value_offset + value_base + index + 32,
                super_scale * f32::from(scales[scale_base + scale_index + 2] as i8) * q2 as f32,
                cols,
                vector,
                output,
            )?;
            accumulate_matrix_value(
                value_offset + value_base + index + 64,
                super_scale * f32::from(scales[scale_base + scale_index + 4] as i8) * q3 as f32,
                cols,
                vector,
                output,
            )?;
            accumulate_matrix_value(
                value_offset + value_base + index + 96,
                super_scale * f32::from(scales[scale_base + scale_index + 6] as i8) * q4 as f32,
                cols,
                vector,
                output,
            )?;
        }
    }

    Ok(())
}

pub(super) fn decode_q6_k_values(
    bytes: &[u8],
    value_count: usize,
) -> Result<Vec<f32>, InferenceError> {
    let expected = q6_k_storage_bytes(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q6_K byte length {} does not match {expected}",
            bytes.len()
        )));
    }

    let mut values = Vec::with_capacity(value_count);
    for block in bytes.chunks_exact(Q6_K_BLOCK_BYTES) {
        let low_bits = &block[0..128];
        let high_bits = &block[128..192];
        let scales = &block[192..208];
        let super_scale = f16_bits_to_f32(u16::from_le_bytes([block[208], block[209]]));
        let mut block_values = vec![0.0; Q6_K_BLOCK_VALUES];

        for half in 0..2 {
            let value_base = half * 128;
            let low_base = half * 64;
            let high_base = half * 32;
            let scale_base = half * 8;

            for index in 0..32 {
                let scale_index = index / 16;
                let high = high_bits[high_base + index];
                let q1 = i32::from((low_bits[low_base + index] & 0x0f) | ((high & 3) << 4)) - 32;
                let q2 =
                    i32::from((low_bits[low_base + index + 32] & 0x0f) | (((high >> 2) & 3) << 4))
                        - 32;
                let q3 =
                    i32::from((low_bits[low_base + index] >> 4) | (((high >> 4) & 3) << 4)) - 32;
                let q4 =
                    i32::from((low_bits[low_base + index + 32] >> 4) | (((high >> 6) & 3) << 4))
                        - 32;

                block_values[value_base + index] =
                    super_scale * f32::from(scales[scale_base + scale_index] as i8) * q1 as f32;
                block_values[value_base + index + 32] =
                    super_scale * f32::from(scales[scale_base + scale_index + 2] as i8) * q2 as f32;
                block_values[value_base + index + 64] =
                    super_scale * f32::from(scales[scale_base + scale_index + 4] as i8) * q3 as f32;
                block_values[value_base + index + 96] =
                    super_scale * f32::from(scales[scale_base + scale_index + 6] as i8) * q4 as f32;
            }
        }

        values.extend(block_values);
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

#[cfg(test)]
mod tests {
    use super::super::q5_0::{q5_0_mul_vec_with_backend, Q5_0MatVecBackend};
    use super::super::q8_0::{q8_0_mul_vec_with_backend, Q8_0MatVecBackend};
    use super::{
        accumulate_q4_k_block, accumulate_q6_k_block, decode_q6_k_values, q4_k_mul_vec,
        q5_0_mul_vec, q6_k_mul_vec, q8_0_mul_vec, InferenceError,
    };

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

    #[test]
    fn q4_k_block_accumulation_updates_rows_without_decoded_matrix() -> Result<(), InferenceError> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
        block.extend_from_slice(&[0x11; 64]);
        block.extend_from_slice(&[0x22; 64]);

        let mut output = vec![0.0; 2];
        accumulate_q4_k_block(&block, 0, 2, 128, &[1.0; 128], &mut output)?;

        assert_eq!(output, vec![128.0, 256.0]);
        Ok(())
    }

    #[test]
    fn q6_k_decoder_reconstructs_signed_block_values() -> Result<(), InferenceError> {
        let mut block = vec![0u8; 128 + 64];
        block[32] = 0xff;
        block[128] = 0xe4;
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());

        let values = decode_q6_k_values(&block, 256)?;

        assert_eq!(values[0], -32.0);
        assert_eq!(values[32], -1.0);
        assert_eq!(values[64], 0.0);
        assert_eq!(values[96], 31.0);
        Ok(())
    }

    #[test]
    fn q6_k_mul_vec_accumulates_rows_without_full_row_decodes() -> Result<(), InferenceError> {
        let mut block = vec![0u8; 128 + 64];
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());

        let actual = q6_k_mul_vec(&block, 2, 128, &[1.0; 128])?;

        assert_eq!(actual, vec![-4096.0, -4096.0]);
        Ok(())
    }

    #[test]
    fn q6_k_block_accumulation_updates_rows_without_decoded_matrix() -> Result<(), InferenceError> {
        let mut block = vec![0u8; 128 + 64];
        block[32] = 0xff;
        block[128] = 0xe4;
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());

        let mut output = vec![0.0; 2];
        accumulate_q6_k_block(&block, 0, 2, 128, &[1.0; 128], &mut output)?;

        assert_eq!(output, vec![-3970.0, -4096.0]);
        Ok(())
    }

    #[test]
    fn q8_0_mul_vec_accumulates_rows_without_row_decodes() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x3c00u16.to_le_bytes());
        bytes.extend([1u8; 32]);
        bytes.extend_from_slice(&0x3c00u16.to_le_bytes());
        bytes.extend([2u8; 32]);

        let actual = q8_0_mul_vec(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(actual, vec![32.0, 64.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn q8_0_matvec_uses_neon_backend_on_aarch64() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q8_0_block_with_value(1));
        bytes.extend(q8_0_block_with_value(-2));

        let output = q8_0_mul_vec_with_backend(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(output.backend, Q8_0MatVecBackend::Aarch64Neon);
        assert_eq!(output.values, vec![32.0, -64.0]);
        Ok(())
    }

    #[test]
    fn q5_0_mul_vec_accumulates_rows_without_row_decodes() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q5_0_block_with_value(1));
        bytes.extend(q5_0_block_with_value(2));

        let actual = q5_0_mul_vec(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(actual, vec![32.0, 64.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn q5_0_matvec_uses_neon_backend_on_aarch64() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q5_0_block_with_value(1));
        bytes.extend(q5_0_block_with_value(-2));

        let output = q5_0_mul_vec_with_backend(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(output.backend, Q5_0MatVecBackend::Aarch64Neon);
        assert_eq!(output.values, vec![32.0, -64.0]);
        Ok(())
    }

    fn q5_0_block_with_value(value: i32) -> Vec<u8> {
        let quantized = (value + 16) as u8;
        let mut high_bits = 0u32;
        for index in 0..16 {
            if quantized & 0x10 != 0 {
                high_bits |= 1 << index;
                high_bits |= 1 << (index + 16);
            }
        }
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&high_bits.to_le_bytes());
        block.extend([(quantized & 0x0f) | ((quantized & 0x0f) << 4); 16]);
        block
    }

    fn q8_0_block_with_value(value: i8) -> Vec<u8> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend([value as u8; 32]);
        block
    }
}
