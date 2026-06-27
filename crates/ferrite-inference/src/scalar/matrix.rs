use super::{
    math::dot,
    quantized::{
        decode_q4_k_values, decode_q5_0_row, decode_q6_k_values, decode_q8_0_row, q4_k_mul_vec,
        q4_k_storage_bytes, q5_0_row_bytes, q6_k_mul_vec, q6_k_storage_bytes, q8_0_row_bytes,
        Q5_0_BLOCK_VALUES, Q8_0_BLOCK_VALUES,
    },
    InferenceError,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: MatrixData,
}

#[derive(Clone, Debug, PartialEq)]
enum MatrixData {
    F32(Vec<f32>),
    Q4K(Vec<u8>),
    Q5_0(Vec<u8>),
    Q6K(Vec<u8>),
    Q8_0(Vec<u8>),
}

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

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn row(&self, index: usize) -> Result<&[f32], InferenceError> {
        if index >= self.rows {
            return Err(InferenceError::new(format!(
                "matrix row {index} is out of bounds for {} rows",
                self.rows
            )));
        }
        let MatrixData::F32(data) = &self.data else {
            return Err(InferenceError::new(
                "borrowed matrix rows are only available for F32 storage",
            ));
        };

        let start = index
            .checked_mul(self.cols)
            .ok_or_else(|| InferenceError::new("matrix row offset overflow"))?;
        let end = start
            .checked_add(self.cols)
            .ok_or_else(|| InferenceError::new("matrix row end overflow"))?;
        Ok(&data[start..end])
    }

    pub fn row_values(&self, index: usize) -> Result<Vec<f32>, InferenceError> {
        if index >= self.rows {
            return Err(InferenceError::new(format!(
                "matrix row {index} is out of bounds for {} rows",
                self.rows
            )));
        }

        match &self.data {
            MatrixData::F32(_) => Ok(self.row(index)?.to_vec()),
            MatrixData::Q4K(data) => {
                let value_count = self
                    .rows
                    .checked_mul(self.cols)
                    .ok_or_else(|| InferenceError::new("Q4_K matrix value count overflow"))?;
                let values = decode_q4_k_values(data, value_count)?;
                let start = index
                    .checked_mul(self.cols)
                    .ok_or_else(|| InferenceError::new("Q4_K row offset overflow"))?;
                let end = start
                    .checked_add(self.cols)
                    .ok_or_else(|| InferenceError::new("Q4_K row end overflow"))?;
                Ok(values[start..end].to_vec())
            }
            MatrixData::Q5_0(data) => {
                let row_bytes = q5_0_row_bytes(self.cols)?;
                let start = index
                    .checked_mul(row_bytes)
                    .ok_or_else(|| InferenceError::new("Q5_0 row offset overflow"))?;
                let end = start
                    .checked_add(row_bytes)
                    .ok_or_else(|| InferenceError::new("Q5_0 row end overflow"))?;
                decode_q5_0_row(&data[start..end], self.cols)
            }
            MatrixData::Q6K(data) => {
                let value_count = self
                    .rows
                    .checked_mul(self.cols)
                    .ok_or_else(|| InferenceError::new("Q6_K matrix value count overflow"))?;
                let values = decode_q6_k_values(data, value_count)?;
                let start = index
                    .checked_mul(self.cols)
                    .ok_or_else(|| InferenceError::new("Q6_K row offset overflow"))?;
                let end = start
                    .checked_add(self.cols)
                    .ok_or_else(|| InferenceError::new("Q6_K row end overflow"))?;
                Ok(values[start..end].to_vec())
            }
            MatrixData::Q8_0(data) => {
                let row_bytes = q8_0_row_bytes(self.cols)?;
                let start = index
                    .checked_mul(row_bytes)
                    .ok_or_else(|| InferenceError::new("Q8_0 row offset overflow"))?;
                let end = start
                    .checked_add(row_bytes)
                    .ok_or_else(|| InferenceError::new("Q8_0 row end overflow"))?;
                decode_q8_0_row(&data[start..end], self.cols)
            }
        }
    }

    pub fn storage_bytes(&self) -> u128 {
        match &self.data {
            MatrixData::F32(values) => values.len() as u128 * std::mem::size_of::<f32>() as u128,
            MatrixData::Q4K(bytes) => bytes.len() as u128,
            MatrixData::Q5_0(bytes) => bytes.len() as u128,
            MatrixData::Q6K(bytes) => bytes.len() as u128,
            MatrixData::Q8_0(bytes) => bytes.len() as u128,
        }
    }

    pub fn mul_vec(&self, vector: &[f32]) -> Result<Vec<f32>, InferenceError> {
        if self.cols != vector.len() {
            return Err(InferenceError::new(format!(
                "matrix columns {} do not match vector length {}",
                self.cols,
                vector.len()
            )));
        }

        if let MatrixData::Q4K(data) = &self.data {
            return q4_k_mul_vec(data, self.rows, self.cols, vector);
        }
        if let MatrixData::Q6K(data) = &self.data {
            return q6_k_mul_vec(data, self.rows, self.cols, vector);
        }

        let mut output = Vec::with_capacity(self.rows);
        for row_index in 0..self.rows {
            let row = self.row_values(row_index)?;
            output.push(dot(&row, vector)?);
        }
        Ok(output)
    }
}
