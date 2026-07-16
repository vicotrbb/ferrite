#![allow(
    unsafe_code,
    reason = "audited SIMD half conversion is isolated in this quantization module"
)]

use super::{InferenceError, ScalarExecutionOptions, float::f16_bits_to_f32};
#[cfg(any(target_arch = "aarch64", test))]
use rayon::prelude::*;

pub(super) const Q8_0_BLOCK_VALUES: usize = 32;
pub(super) const Q8_0_BLOCK_BYTES: usize = 34;
#[cfg(target_arch = "aarch64")]
const PARALLEL_ARGMAX_MIN_ROWS: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Q8_0MatVecBackend {
    Scalar,
    #[cfg(target_arch = "aarch64")]
    Aarch64Neon,
    #[cfg(target_arch = "aarch64")]
    Aarch64NeonRowParallel,
    #[cfg(target_arch = "x86_64")]
    X86_64Avx2,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Q8_0MatVecOutput {
    pub(super) values: Vec<f32>,
    pub(super) backend: Q8_0MatVecBackend,
}

pub(super) fn q8_0_row_bytes(cols: usize) -> Result<usize, InferenceError> {
    if !cols.is_multiple_of(Q8_0_BLOCK_VALUES) {
        return Err(InferenceError::new(format!(
            "Q8_0 value count {cols} must be divisible by {Q8_0_BLOCK_VALUES}"
        )));
    }
    cols.checked_div(Q8_0_BLOCK_VALUES)
        .and_then(|blocks| blocks.checked_mul(Q8_0_BLOCK_BYTES))
        .ok_or_else(|| InferenceError::new("Q8_0 byte length overflow"))
}

pub(super) fn validate_q8_0_finite_scales(bytes: &[u8]) -> Result<(), InferenceError> {
    for block in bytes.chunks_exact(Q8_0_BLOCK_BYTES) {
        let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
        if !scale.is_finite() {
            return Err(InferenceError::new(
                "Q8_0 matrix scale values must be finite",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn q8_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    Ok(q8_0_mul_vec_with_backend(bytes, rows, cols, vector)?.values)
}

pub(super) fn q8_0_mul_vec_with_options(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
    options: ScalarExecutionOptions,
) -> Result<Vec<f32>, InferenceError> {
    Ok(q8_0_mul_vec_with_backend_and_options(bytes, rows, cols, vector, options)?.values)
}

#[cfg(test)]
pub(super) fn q8_0_argmax_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<usize, InferenceError> {
    q8_0_argmax_mul_vec_with_options(bytes, rows, cols, vector, ScalarExecutionOptions::default())
}

pub(super) fn q8_0_argmax_mul_vec_with_options(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
    options: ScalarExecutionOptions,
) -> Result<usize, InferenceError> {
    validate_q8_0_mul_vec(bytes, rows, cols, vector)?;
    if rows == 0 {
        return Err(InferenceError::new("argmax input must not be empty"));
    }

    #[cfg(target_arch = "aarch64")]
    {
        if options.kernel_dispatch().neon() {
            if rows >= PARALLEL_ARGMAX_MIN_ROWS {
                return Ok(super::q8_0_neon::neon_q8_0_parallel_argmax_mul_vec(
                    bytes, cols, vector,
                ));
            }
            return Ok(super::q8_0_neon::neon_q8_0_argmax_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if options.kernel_dispatch().avx2() {
            return Ok(super::q8_0_avx2::avx2_q8_0_argmax_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }

    scalar_q8_0_argmax_mul_vec(bytes, rows, cols, vector)
}

/// Batched matvec across several activation vectors using one provider policy.
pub(super) fn q8_0_mul_vec_batch_with_options(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vectors: &[&[f32]],
    options: ScalarExecutionOptions,
) -> Result<Vec<Vec<f32>>, InferenceError> {
    let Some(first) = vectors.first() else {
        return Ok(Vec::new());
    };
    validate_q8_0_mul_vec(bytes, rows, cols, first)?;

    #[cfg(target_arch = "aarch64")]
    {
        if options.kernel_dispatch().neon() {
            return Ok(super::q8_0_neon::neon_q8_0_mul_vec_batch(
                bytes, rows, cols, vectors,
            ));
        }
    }

    vectors
        .iter()
        .map(|vector| q8_0_mul_vec_with_options(bytes, rows, cols, vector, options))
        .collect()
}

/// Batched greedy argmax over the logits matvec; each stream's result is
/// identical to `q8_0_argmax_mul_vec` with that vector.
#[cfg(test)]
pub(super) fn q8_0_argmax_mul_vec_batch(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vectors: &[&[f32]],
) -> Result<Vec<usize>, InferenceError> {
    q8_0_argmax_mul_vec_batch_with_options(
        bytes,
        rows,
        cols,
        vectors,
        ScalarExecutionOptions::default(),
    )
}

pub(super) fn q8_0_argmax_mul_vec_batch_with_options(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vectors: &[&[f32]],
    options: ScalarExecutionOptions,
) -> Result<Vec<usize>, InferenceError> {
    let Some(first) = vectors.first() else {
        return Ok(Vec::new());
    };
    validate_q8_0_mul_vec(bytes, rows, cols, first)?;
    if rows == 0 {
        return Err(InferenceError::new("argmax input must not be empty"));
    }

    #[cfg(target_arch = "aarch64")]
    {
        if options.kernel_dispatch().neon() {
            return Ok(super::q8_0_neon::neon_q8_0_argmax_mul_vec_batch(
                bytes, cols, vectors,
            ));
        }
    }

    vectors
        .iter()
        .map(|vector| q8_0_argmax_mul_vec_with_options(bytes, rows, cols, vector, options))
        .collect()
}

#[cfg(test)]
pub(super) fn q8_0_mul_vec_with_backend(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q8_0MatVecOutput, InferenceError> {
    q8_0_mul_vec_with_backend_and_options(
        bytes,
        rows,
        cols,
        vector,
        ScalarExecutionOptions::default(),
    )
}

fn q8_0_mul_vec_with_backend_and_options(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
    options: ScalarExecutionOptions,
) -> Result<Q8_0MatVecOutput, InferenceError> {
    validate_q8_0_mul_vec(bytes, rows, cols, vector)?;

    #[cfg(target_arch = "aarch64")]
    {
        if options.kernel_dispatch().neon() {
            return Ok(super::q8_0_neon::neon_q8_0_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if options.kernel_dispatch().avx2() {
            return Ok(super::q8_0_avx2::avx2_q8_0_mul_vec(
                bytes, rows, cols, vector,
            ));
        }
    }

    scalar_q8_0_mul_vec(bytes, rows, cols, vector)
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

fn validate_q8_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<(), InferenceError> {
    if vector.len() != cols {
        return Err(InferenceError::new(format!(
            "matrix columns {cols} do not match vector length {}",
            vector.len()
        )));
    }
    let row_bytes = q8_0_row_bytes(cols)?;
    let expected = rows
        .checked_mul(row_bytes)
        .ok_or_else(|| InferenceError::new("Q8_0 matrix byte length overflow"))?;
    if bytes.len() != expected {
        return Err(InferenceError::new(format!(
            "Q8_0 matrix byte length {} does not match shape {rows}x{cols}",
            bytes.len()
        )));
    }
    Ok(())
}

fn scalar_q8_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Q8_0MatVecOutput, InferenceError> {
    let row_bytes = q8_0_row_bytes(cols)?;
    let mut values = vec![0.0; rows];
    for (row, row_bytes) in bytes.chunks_exact(row_bytes).enumerate() {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let col_base = block_index * Q8_0_BLOCK_VALUES;
            for (offset, quantized) in block[2..].iter().enumerate() {
                sum += scale * f32::from(*quantized as i8) * vector[col_base + offset];
            }
        }
        values[row] = sum;
    }

    Ok(Q8_0MatVecOutput {
        values,
        backend: Q8_0MatVecBackend::Scalar,
    })
}

fn scalar_q8_0_argmax_mul_vec(
    bytes: &[u8],
    _rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<usize, InferenceError> {
    let row_bytes = q8_0_row_bytes(cols)?;
    Ok(argmax_q8_0_rows(bytes, row_bytes, |row_bytes| {
        let mut sum = 0.0;
        for (block_index, block) in row_bytes.chunks_exact(Q8_0_BLOCK_BYTES).enumerate() {
            let scale = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
            let col_base = block_index * Q8_0_BLOCK_VALUES;
            for (offset, quantized) in block[2..].iter().enumerate() {
                sum += scale * f32::from(*quantized as i8) * vector[col_base + offset];
            }
        }
        sum
    }))
}

pub(super) fn argmax_q8_0_rows<F>(bytes: &[u8], row_bytes: usize, mut row_dot: F) -> usize
where
    F: FnMut(&[u8]) -> f32,
{
    let mut best_index = 0usize;
    let mut best_value = f32::NEG_INFINITY;

    for (row_index, row_chunk) in bytes.chunks_exact(row_bytes).enumerate() {
        let sum = row_dot(row_chunk);
        if row_index == 0 || sum > best_value {
            best_index = row_index;
            best_value = sum;
        }
    }

    best_index
}

#[cfg(any(target_arch = "aarch64", test))]
pub(super) fn parallel_argmax_q8_0_rows<F>(bytes: &[u8], row_bytes: usize, row_dot: F) -> usize
where
    F: Fn(&[u8]) -> f32 + Sync,
{
    bytes
        .par_chunks_exact(row_bytes)
        .enumerate()
        .map(|(row_index, row_chunk)| (row_index, row_dot(row_chunk)))
        .reduce(
            || (0usize, f32::NEG_INFINITY),
            |left, right| {
                if right.1 > left.1 { right } else { left }
            },
        )
        .0
}

#[cfg(test)]
mod tests {
    use super::{argmax_q8_0_rows, parallel_argmax_q8_0_rows};

    #[test]
    fn parallel_argmax_q8_0_rows_matches_sequential_argmax() {
        let rows = [-2.0f32, 1.0, 0.0, 4.5, -3.0, 4.25, 3.0, -1.0, 2.0, 0.5];
        let bytes = rows
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let row_bytes = std::mem::size_of::<f32>();
        let row_dot = |row: &[u8]| {
            let mut value = [0u8; 4];
            value.copy_from_slice(row);
            f32::from_le_bytes(value)
        };

        let expected = argmax_q8_0_rows(&bytes, row_bytes, row_dot);
        let actual = parallel_argmax_q8_0_rows(&bytes, row_bytes, row_dot);

        assert_eq!(actual, expected);
    }
}
