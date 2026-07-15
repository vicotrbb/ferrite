use super::{Matrix, MatrixBytes, MatrixData};
use crate::scalar::{
    dense16::{validate_bf16_values, validate_f16_values},
    q4_k::validate_q4_k_finite_scales,
    q5_0::validate_q5_0_finite_scales,
    q5_k::{q5_k_storage_bytes, validate_q5_k_finite_scales, Q5_K_BLOCK_VALUES},
    q6_k::validate_q6_k_finite_scales,
    q8_0::validate_q8_0_finite_scales,
    quantized::{
        q4_k_storage_bytes, q5_0_row_bytes, q6_k_storage_bytes, q8_0_row_bytes, Q5_0_BLOCK_VALUES,
        Q8_0_BLOCK_VALUES,
    },
    InferenceError,
};
use ferrite_model::model_file::MappedModelFile;
use std::ops::Range;

impl Matrix {
    /// Creates an F32 matrix from row-major values.
    ///
    /// # Errors
    ///
    /// Returns an error when the shape overflows, the data length does not
    /// match `rows * cols`, or any value is non-finite.
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

    /// Creates an F16 matrix from GGML row-major little-endian bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the shape overflows, the byte length does not
    /// match `rows * cols * 2`, or any decoded value is non-finite.
    pub fn from_f16_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        Self::from_f16_storage(rows, cols, MatrixBytes::Owned(data))
    }

    pub(in crate::scalar) fn from_f16_mapped_bytes(
        rows: usize,
        cols: usize,
        file: MappedModelFile,
        range: Range<usize>,
    ) -> Result<Self, InferenceError> {
        Self::from_f16_storage(rows, cols, MatrixBytes::mapped(file, range)?)
    }

    fn from_f16_storage(
        rows: usize,
        cols: usize,
        data: MatrixBytes,
    ) -> Result<Self, InferenceError> {
        validate_dense16_shape("F16", rows, cols, data.len())?;
        validate_f16_values(data.as_slice())?;
        Ok(Self {
            rows,
            cols,
            data: MatrixData::F16(data),
        })
    }

    /// Creates a BF16 matrix from GGML row-major little-endian bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the shape overflows, the byte length does not
    /// match `rows * cols * 2`, or any decoded value is non-finite.
    pub fn from_bf16_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        Self::from_bf16_storage(rows, cols, MatrixBytes::Owned(data))
    }

    pub(in crate::scalar) fn from_bf16_mapped_bytes(
        rows: usize,
        cols: usize,
        file: MappedModelFile,
        range: Range<usize>,
    ) -> Result<Self, InferenceError> {
        Self::from_bf16_storage(rows, cols, MatrixBytes::mapped(file, range)?)
    }

    fn from_bf16_storage(
        rows: usize,
        cols: usize,
        data: MatrixBytes,
    ) -> Result<Self, InferenceError> {
        validate_dense16_shape("BF16", rows, cols, data.len())?;
        validate_bf16_values(data.as_slice())?;
        Ok(Self {
            rows,
            cols,
            data: MatrixData::BF16(data),
        })
    }

    /// Creates a `Q8_0` matrix from GGML row-major block bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the column count is not block-aligned, byte size
    /// does not match the shape, arithmetic overflows, or a scale is non-finite.
    pub fn from_q8_0_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        Self::from_q8_0_storage(rows, cols, MatrixBytes::Owned(data))
    }

    pub(in crate::scalar) fn from_q8_0_mapped_bytes(
        rows: usize,
        cols: usize,
        file: MappedModelFile,
        range: Range<usize>,
    ) -> Result<Self, InferenceError> {
        Self::from_q8_0_storage(rows, cols, MatrixBytes::mapped(file, range)?)
    }

    fn from_q8_0_storage(
        rows: usize,
        cols: usize,
        data: MatrixBytes,
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
        validate_q8_0_finite_scales(data.as_slice())?;

        Ok(Self {
            rows,
            cols,
            data: MatrixData::Q8_0(data),
        })
    }

    /// Creates a `Q5_0` matrix from GGML row-major block bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the column count is not block-aligned, byte size
    /// does not match the shape, arithmetic overflows, or a scale is non-finite.
    pub fn from_q5_0_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        Self::from_q5_0_storage(rows, cols, MatrixBytes::Owned(data))
    }

    pub(in crate::scalar) fn from_q5_0_mapped_bytes(
        rows: usize,
        cols: usize,
        file: MappedModelFile,
        range: Range<usize>,
    ) -> Result<Self, InferenceError> {
        Self::from_q5_0_storage(rows, cols, MatrixBytes::mapped(file, range)?)
    }

    fn from_q5_0_storage(
        rows: usize,
        cols: usize,
        data: MatrixBytes,
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
        validate_q5_0_finite_scales(data.as_slice())?;

        Ok(Self {
            rows,
            cols,
            data: MatrixData::Q5_0(data),
        })
    }

    /// Creates a `Q5_K` matrix from GGML row-major block bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the shape is not block-aligned, byte size does
    /// not match the shape, arithmetic overflows, or a scale is non-finite.
    pub fn from_q5_k_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        Self::from_q5_k_storage(rows, cols, MatrixBytes::Owned(data))
    }

    pub(in crate::scalar) fn from_q5_k_mapped_bytes(
        rows: usize,
        cols: usize,
        file: MappedModelFile,
        range: Range<usize>,
    ) -> Result<Self, InferenceError> {
        Self::from_q5_k_storage(rows, cols, MatrixBytes::mapped(file, range)?)
    }

    fn from_q5_k_storage(
        rows: usize,
        cols: usize,
        data: MatrixBytes,
    ) -> Result<Self, InferenceError> {
        if !cols.is_multiple_of(Q5_K_BLOCK_VALUES) {
            return Err(InferenceError::new(format!(
                "Q5_K matrix columns {cols} must be divisible by {Q5_K_BLOCK_VALUES}"
            )));
        }
        let value_count = rows
            .checked_mul(cols)
            .ok_or_else(|| InferenceError::new("Q5_K matrix value count overflow"))?;
        let expected = q5_k_storage_bytes(value_count)?;
        if data.len() != expected {
            return Err(InferenceError::new(format!(
                "Q5_K matrix byte length {} does not match shape {rows}x{cols}",
                data.len()
            )));
        }
        validate_q5_k_finite_scales(data.as_slice())?;

        Ok(Self {
            rows,
            cols,
            data: MatrixData::Q5K(data),
        })
    }

    /// Creates a `Q4_K` matrix from GGML row-major block bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the value count is not block-aligned, byte size
    /// does not match the shape, arithmetic overflows, or a scale is non-finite.
    pub fn from_q4_k_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        Self::from_q4_k_storage(rows, cols, MatrixBytes::Owned(data))
    }

    pub(in crate::scalar) fn from_q4_k_mapped_bytes(
        rows: usize,
        cols: usize,
        file: MappedModelFile,
        range: Range<usize>,
    ) -> Result<Self, InferenceError> {
        Self::from_q4_k_storage(rows, cols, MatrixBytes::mapped(file, range)?)
    }

    fn from_q4_k_storage(
        rows: usize,
        cols: usize,
        data: MatrixBytes,
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
        validate_q4_k_finite_scales(data.as_slice())?;

        Ok(Self {
            rows,
            cols,
            data: MatrixData::Q4K(data),
        })
    }

    /// Creates a `Q6_K` matrix from GGML row-major block bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the value count is not block-aligned, byte size
    /// does not match the shape, arithmetic overflows, or a scale is non-finite.
    pub fn from_q6_k_row_major_bytes(
        rows: usize,
        cols: usize,
        data: Vec<u8>,
    ) -> Result<Self, InferenceError> {
        Self::from_q6_k_storage(rows, cols, MatrixBytes::Owned(data))
    }

    pub(in crate::scalar) fn from_q6_k_mapped_bytes(
        rows: usize,
        cols: usize,
        file: MappedModelFile,
        range: Range<usize>,
    ) -> Result<Self, InferenceError> {
        Self::from_q6_k_storage(rows, cols, MatrixBytes::mapped(file, range)?)
    }

    fn from_q6_k_storage(
        rows: usize,
        cols: usize,
        data: MatrixBytes,
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
        validate_q6_k_finite_scales(data.as_slice())?;

        Ok(Self {
            rows,
            cols,
            data: MatrixData::Q6K(data),
        })
    }
}

fn validate_dense16_shape(
    label: &str,
    rows: usize,
    cols: usize,
    actual_bytes: usize,
) -> Result<(), InferenceError> {
    let expected = rows
        .checked_mul(cols)
        .and_then(|values| values.checked_mul(2))
        .ok_or_else(|| InferenceError::new(format!("{label} matrix byte length overflow")))?;
    if actual_bytes != expected {
        return Err(InferenceError::new(format!(
            "{label} matrix byte length {actual_bytes} does not match shape {rows}x{cols}"
        )));
    }
    Ok(())
}
