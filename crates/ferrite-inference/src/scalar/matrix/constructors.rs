use super::{Matrix, MatrixData};
use crate::scalar::{
    q4_k::validate_q4_k_finite_scales,
    q5_0::validate_q5_0_finite_scales,
    q8_0::validate_q8_0_finite_scales,
    quantized::{
        q4_k_storage_bytes, q5_0_row_bytes, q6_k_storage_bytes, q8_0_row_bytes, Q5_0_BLOCK_VALUES,
        Q8_0_BLOCK_VALUES,
    },
    InferenceError,
};

impl Matrix {
    pub fn from_row_major(
        rows: usize,
        cols: usize,
        data: Vec<f32>,
    ) -> Result<Self, InferenceError> {
        let expected = rows
            .checked_mul(cols)
            .ok_or_else(|| InferenceError::new("matrix shape overflow"))?;
        if data.len() != expected {
            return Err(InferenceError::new(format!(
                "matrix data length {} does not match shape {rows}x{cols}",
                data.len()
            )));
        }
        if data.iter().any(|value| !value.is_finite()) {
            return Err(InferenceError::new("matrix data values must be finite"));
        }

        Ok(Self {
            rows,
            cols,
            data: MatrixData::F32(data),
        })
    }

    pub fn from_q8_0_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        if !cols.is_multiple_of(Q8_0_BLOCK_VALUES) {
            return Err(InferenceError::new(format!(
                "Q8_0 matrix columns {cols} must be divisible by {Q8_0_BLOCK_VALUES}"
            )));
        }
        let row_bytes = q8_0_row_bytes(cols)?;
        let expected = rows
            .checked_mul(row_bytes)
            .ok_or_else(|| InferenceError::new("Q8_0 matrix byte length overflow"))?;
        if data.len() != expected {
            return Err(InferenceError::new(format!(
                "Q8_0 matrix byte length {} does not match shape {rows}x{cols}",
                data.len()
            )));
        }
        validate_q8_0_finite_scales(&data)?;

        Ok(Self {
            rows,
            cols,
            data: MatrixData::Q8_0(data),
        })
    }

    pub fn from_q5_0_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        if !cols.is_multiple_of(Q5_0_BLOCK_VALUES) {
            return Err(InferenceError::new(format!(
                "Q5_0 matrix columns {cols} must be divisible by {Q5_0_BLOCK_VALUES}"
            )));
        }
        let row_bytes = q5_0_row_bytes(cols)?;
        let expected = rows
            .checked_mul(row_bytes)
            .ok_or_else(|| InferenceError::new("Q5_0 matrix byte length overflow"))?;
        if data.len() != expected {
            return Err(InferenceError::new(format!(
                "Q5_0 matrix byte length {} does not match shape {rows}x{cols}",
                data.len()
            )));
        }
        validate_q5_0_finite_scales(&data)?;

        Ok(Self {
            rows,
            cols,
            data: MatrixData::Q5_0(data),
        })
    }

    pub fn from_q4_k_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        let value_count = rows
            .checked_mul(cols)
            .ok_or_else(|| InferenceError::new("Q4_K matrix value count overflow"))?;
        let expected = q4_k_storage_bytes(value_count)?;
        if data.len() != expected {
            return Err(InferenceError::new(format!(
                "Q4_K matrix byte length {} does not match shape {rows}x{cols}",
                data.len()
            )));
        }
        validate_q4_k_finite_scales(&data)?;

        Ok(Self {
            rows,
            cols,
            data: MatrixData::Q4K(data),
        })
    }

    pub fn from_q6_k_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        let value_count = rows
            .checked_mul(cols)
            .ok_or_else(|| InferenceError::new("Q6_K matrix value count overflow"))?;
        let expected = q6_k_storage_bytes(value_count)?;
        if data.len() != expected {
            return Err(InferenceError::new(format!(
                "Q6_K matrix byte length {} does not match shape {rows}x{cols}",
                data.len()
            )));
        }

        Ok(Self {
            rows,
            cols,
            data: MatrixData::Q6K(data),
        })
    }
}
