use super::MatrixData;
use crate::scalar::{
    quantized::{
        decode_q4_k_values, decode_q5_0_row, decode_q6_k_values, decode_q8_0_row, q5_0_row_bytes,
        q8_0_row_bytes,
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
        MatrixData::Q4K(bytes) => q4_k_row_values(bytes, rows, cols, index),
        MatrixData::Q5_0(bytes) => q5_0_row_values(bytes, cols, index),
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
    let value_count = rows
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("Q4_K matrix value count overflow"))?;
    let values = decode_q4_k_values(bytes, value_count)?;
    let start = index
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("Q4_K row offset overflow"))?;
    let end = start
        .checked_add(cols)
        .ok_or_else(|| InferenceError::new("Q4_K row end overflow"))?;
    Ok(values[start..end].to_vec())
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

fn q6_k_row_values(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    index: usize,
) -> Result<Vec<f32>, InferenceError> {
    let value_count = rows
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("Q6_K matrix value count overflow"))?;
    let values = decode_q6_k_values(bytes, value_count)?;
    let start = index
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("Q6_K row offset overflow"))?;
    let end = start
        .checked_add(cols)
        .ok_or_else(|| InferenceError::new("Q6_K row end overflow"))?;
    Ok(values[start..end].to_vec())
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
