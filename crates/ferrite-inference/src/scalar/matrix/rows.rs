use super::MatrixData;
use crate::scalar::{
    dense16::{bf16_row_values, f16_row_values},
    q4_k::Q4_K_BLOCK_VALUES,
    q5_k::Q5_K_BLOCK_VALUES,
    q6_k::Q6_K_BLOCK_VALUES,
    quantized::{
        decode_q4_k_values, decode_q5_0_row, decode_q5_k_values, decode_q6_k_values,
        decode_q8_0_row, q4_k_storage_bytes, q5_0_row_bytes, q5_k_storage_bytes,
        q6_k_storage_bytes, q8_0_row_bytes,
    },
    InferenceError,
};

pub(super) fn row_values(
    data: &MatrixData,
    rows: usize,
    cols: usize,
    index: usize,
) -> Result<Vec<f32>, InferenceError> {
    if index >= rows {
        return Err(InferenceError::new(format!(
            "matrix row {index} is out of bounds for {rows} rows"
        )));
    }

    match data {
        MatrixData::F32(values) => f32_row_values(values, cols, index),
        MatrixData::F16(bytes) => Ok(f16_row_values(bytes, cols, index)),
        MatrixData::BF16(bytes) => Ok(bf16_row_values(bytes, cols, index)),
        MatrixData::Q4K(bytes) => q4_k_row_values(bytes, rows, cols, index),
        MatrixData::Q5_0(bytes) => q5_0_row_values(bytes, cols, index),
        MatrixData::Q5K(bytes) => q5_k_row_values(bytes, rows, cols, index),
        MatrixData::Q6K(bytes) => q6_k_row_values(bytes, rows, cols, index),
        MatrixData::Q8_0(bytes) => q8_0_row_values(bytes, cols, index),
    }
}

fn f32_row_values(values: &[f32], cols: usize, index: usize) -> Result<Vec<f32>, InferenceError> {
    let start = index
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("matrix row offset overflow"))?;
    let end = start
        .checked_add(cols)
        .ok_or_else(|| InferenceError::new("matrix row end overflow"))?;
    Ok(values[start..end].to_vec())
}

fn q4_k_row_values(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    index: usize,
) -> Result<Vec<f32>, InferenceError> {
    quantized_k_row_values(
        bytes,
        rows,
        cols,
        index,
        QuantizedKCodec {
            label: "Q4_K",
            block_values: Q4_K_BLOCK_VALUES,
            storage_bytes: q4_k_storage_bytes,
            decode_values: decode_q4_k_values,
        },
    )
}

fn q5_0_row_values(bytes: &[u8], cols: usize, index: usize) -> Result<Vec<f32>, InferenceError> {
    let row_bytes = q5_0_row_bytes(cols)?;
    let start = index
        .checked_mul(row_bytes)
        .ok_or_else(|| InferenceError::new("Q5_0 row offset overflow"))?;
    let end = start
        .checked_add(row_bytes)
        .ok_or_else(|| InferenceError::new("Q5_0 row end overflow"))?;
    decode_q5_0_row(&bytes[start..end], cols)
}

fn q5_k_row_values(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    index: usize,
) -> Result<Vec<f32>, InferenceError> {
    quantized_k_row_values(
        bytes,
        rows,
        cols,
        index,
        QuantizedKCodec {
            label: "Q5_K",
            block_values: Q5_K_BLOCK_VALUES,
            storage_bytes: q5_k_storage_bytes,
            decode_values: decode_q5_k_values,
        },
    )
}

fn q6_k_row_values(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    index: usize,
) -> Result<Vec<f32>, InferenceError> {
    quantized_k_row_values(
        bytes,
        rows,
        cols,
        index,
        QuantizedKCodec {
            label: "Q6_K",
            block_values: Q6_K_BLOCK_VALUES,
            storage_bytes: q6_k_storage_bytes,
            decode_values: decode_q6_k_values,
        },
    )
}

#[derive(Clone, Copy)]
struct QuantizedKCodec {
    label: &'static str,
    block_values: usize,
    storage_bytes: fn(usize) -> Result<usize, InferenceError>,
    decode_values: fn(&[u8], usize) -> Result<Vec<f32>, InferenceError>,
}

fn quantized_k_row_values(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    index: usize,
    codec: QuantizedKCodec,
) -> Result<Vec<f32>, InferenceError> {
    let label = codec.label;
    let block_values = codec.block_values;
    let value_count = rows
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new(format!("{label} matrix value count overflow")))?;
    let expected = (codec.storage_bytes)(value_count)?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "{label} byte length {} does not match {expected}",
            bytes.len()
        )));
    }

    let row_start = index
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new(format!("{label} row offset overflow")))?;
    let row_end = row_start
        .checked_add(cols)
        .ok_or_else(|| InferenceError::new(format!("{label} row end overflow")))?;
    let first_block = row_start / block_values;
    let last_block = row_end
        .checked_add(block_values - 1)
        .ok_or_else(|| InferenceError::new(format!("{label} row block range overflow")))?
        / block_values;
    let block_bytes = (codec.storage_bytes)(block_values)?;
    let byte_start = first_block
        .checked_mul(block_bytes)
        .ok_or_else(|| InferenceError::new(format!("{label} row byte offset overflow")))?;
    let byte_end = last_block
        .checked_mul(block_bytes)
        .ok_or_else(|| InferenceError::new(format!("{label} row byte end overflow")))?;
    let selected = bytes.get(byte_start..byte_end).ok_or_else(|| {
        InferenceError::new(format!(
            "{label} row byte range {byte_start}..{byte_end} is out of bounds for {} bytes",
            bytes.len()
        ))
    })?;
    let selected_value_count = last_block
        .checked_sub(first_block)
        .and_then(|blocks| blocks.checked_mul(block_values))
        .ok_or_else(|| InferenceError::new(format!("{label} row value window overflow")))?;
    let values = (codec.decode_values)(selected, selected_value_count)?;
    let local_start = row_start % block_values;
    let local_end = local_start
        .checked_add(cols)
        .ok_or_else(|| InferenceError::new(format!("{label} local row end overflow")))?;
    values
        .get(local_start..local_end)
        .map(<[f32]>::to_vec)
        .ok_or_else(|| {
            InferenceError::new(format!(
                "{label} decoded row range {local_start}..{local_end} is out of bounds for {} values",
                values.len()
            ))
        })
}

fn q8_0_row_values(bytes: &[u8], cols: usize, index: usize) -> Result<Vec<f32>, InferenceError> {
    let row_bytes = q8_0_row_bytes(cols)?;
    let start = index
        .checked_mul(row_bytes)
        .ok_or_else(|| InferenceError::new("Q8_0 row offset overflow"))?;
    let end = start
        .checked_add(row_bytes)
        .ok_or_else(|| InferenceError::new("Q8_0 row end overflow"))?;
    decode_q8_0_row(&bytes[start..end], cols)
}

#[cfg(test)]
mod tests {
    use super::super::Matrix;
    use super::{quantized_k_row_values, QuantizedKCodec};
    use crate::scalar::InferenceError;

    #[test]
    fn quantized_row_decode_bounds_the_decoder_to_intersecting_blocks() -> Result<(), InferenceError>
    {
        let row = quantized_k_row_values(
            &[0; 768],
            4,
            192,
            1,
            QuantizedKCodec {
                label: "TEST",
                block_values: 256,
                storage_bytes: identity_storage_bytes,
                decode_values: decode_at_most_two_blocks,
            },
        )?;

        assert_eq!(row, vec![0.0; 192]);
        Ok(())
    }

    #[test]
    fn q4_k_row_decode_reads_only_intersecting_blocks() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q4_k_block_with_value(1));
        bytes.extend(q4_k_block_with_value(2));
        bytes.extend(q4_k_block_with_value(3));
        let matrix = Matrix::from_q4_k_row_major_bytes(4, 192, bytes)?;

        let row = matrix.row_values(1)?;

        assert_eq!(row.len(), 192);
        assert_eq!(&row[..64], vec![1.0; 64]);
        assert_eq!(&row[64..], vec![2.0; 128]);
        Ok(())
    }

    #[test]
    fn q6_k_row_decode_reads_only_intersecting_blocks() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q6_k_block_with_scale(1.0));
        bytes.extend(q6_k_block_with_scale(2.0));
        bytes.extend(q6_k_block_with_scale(3.0));
        let matrix = Matrix::from_q6_k_row_major_bytes(4, 192, bytes)?;

        let row = matrix.row_values(2)?;

        assert_eq!(row.len(), 192);
        assert_eq!(&row[..128], vec![-64.0; 128]);
        assert_eq!(&row[128..], vec![-96.0; 64]);
        Ok(())
    }

    fn q4_k_block_with_value(value: u8) -> Vec<u8> {
        let quantized = value & 0x0f;
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
        block.extend_from_slice(&[quantized | (quantized << 4); 128]);
        block
    }

    fn q6_k_block_with_scale(scale: f32) -> Vec<u8> {
        let scale_bits = match scale {
            1.0 => 0x3c00u16,
            2.0 => 0x4000u16,
            3.0 => 0x4200u16,
            _ => unreachable!("test helper only supports exact small F16 values"),
        };
        let mut block = vec![0u8; 128 + 64];
        block.extend([1u8; 16]);
        block.extend_from_slice(&scale_bits.to_le_bytes());
        block
    }

    fn identity_storage_bytes(value_count: usize) -> Result<usize, InferenceError> {
        Ok(value_count)
    }

    fn decode_at_most_two_blocks(
        bytes: &[u8],
        value_count: usize,
    ) -> Result<Vec<f32>, InferenceError> {
        if value_count > 512 {
            return Err(InferenceError::new(
                "row decoder was given more than two intersecting blocks",
            ));
        }
        if bytes.len() != value_count {
            return Err(InferenceError::new(
                "test decoder byte and value counts must match",
            ));
        }
        Ok(vec![0.0; value_count])
    }
}
