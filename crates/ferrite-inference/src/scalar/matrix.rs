use super::{math::dot, InferenceError};

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: MatrixData,
}

#[derive(Clone, Debug, PartialEq)]
enum MatrixData {
    F32(Vec<f32>),
    Q5_0(Vec<u8>),
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
            MatrixData::Q5_0(bytes) => bytes.len() as u128,
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

        let mut output = Vec::with_capacity(self.rows);
        for row_index in 0..self.rows {
            let row = self.row_values(row_index)?;
            output.push(dot(&row, vector)?);
        }
        Ok(output)
    }
}

const Q5_0_BLOCK_VALUES: usize = 32;
const Q5_0_BLOCK_BYTES: usize = 22;
const Q8_0_BLOCK_VALUES: usize = 32;
const Q8_0_BLOCK_BYTES: usize = 34;

fn q5_0_row_bytes(cols: usize) -> Result<usize, InferenceError> {
    cols.checked_div(Q5_0_BLOCK_VALUES)
        .and_then(|blocks| blocks.checked_mul(Q5_0_BLOCK_BYTES))
        .ok_or_else(|| InferenceError::new("Q5_0 row byte length overflow"))
}

fn q8_0_row_bytes(cols: usize) -> Result<usize, InferenceError> {
    cols.checked_div(Q8_0_BLOCK_VALUES)
        .and_then(|blocks| blocks.checked_mul(Q8_0_BLOCK_BYTES))
        .ok_or_else(|| InferenceError::new("Q8_0 row byte length overflow"))
}

fn decode_q5_0_row(bytes: &[u8], cols: usize) -> Result<Vec<f32>, InferenceError> {
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

fn decode_q8_0_row(bytes: &[u8], cols: usize) -> Result<Vec<f32>, InferenceError> {
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
