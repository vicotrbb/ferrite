use super::{dot, InferenceError};

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<f32>,
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

        Ok(Self { rows, cols, data })
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

        let start = index
            .checked_mul(self.cols)
            .ok_or_else(|| InferenceError::new("matrix row offset overflow"))?;
        let end = start
            .checked_add(self.cols)
            .ok_or_else(|| InferenceError::new("matrix row end overflow"))?;
        Ok(&self.data[start..end])
    }

    pub fn mul_vec(&self, vector: &[f32]) -> Result<Vec<f32>, InferenceError> {
        if self.cols != vector.len() {
            return Err(InferenceError::new(format!(
                "matrix columns {} do not match vector length {}",
                self.cols,
                vector.len()
            )));
        }

        let mut output = Vec::with_capacity(self.rows);
        for row_index in 0..self.rows {
            let row = self.row(row_index)?;
            output.push(dot(row, vector)?);
        }
        Ok(output)
    }
}
