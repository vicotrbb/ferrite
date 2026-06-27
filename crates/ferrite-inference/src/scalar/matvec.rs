#![allow(unsafe_code)]

use super::{math::dot, InferenceError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum F32MatVecBackend {
    Scalar,
    Aarch64Neon,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct F32MatVecOutput {
    values: Vec<f32>,
    backend: F32MatVecBackend,
}

impl F32MatVecOutput {
    pub(super) fn into_values(self) -> Vec<f32> {
        self.values
    }
}

pub(super) fn f32_mul_vec(
    rows: usize,
    cols: usize,
    data: &[f32],
    vector: &[f32],
) -> Result<F32MatVecOutput, InferenceError> {
    if vector.len() != cols {
        return Err(InferenceError::new(format!(
            "matrix columns {cols} do not match vector length {}",
            vector.len()
        )));
    }
    let expected = rows
        .checked_mul(cols)
        .ok_or_else(|| InferenceError::new("F32 matrix value count overflow"))?;
    if data.len() != expected {
        return Err(InferenceError::new(format!(
            "F32 matrix data length {} does not match shape {rows}x{cols}",
            data.len()
        )));
    }

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            return Ok(aarch64::neon_f32_mul_vec(rows, cols, data, vector));
        }
    }

    scalar_f32_mul_vec(rows, cols, data, vector)
}

fn scalar_f32_mul_vec(
    rows: usize,
    cols: usize,
    data: &[f32],
    vector: &[f32],
) -> Result<F32MatVecOutput, InferenceError> {
    let mut values = Vec::with_capacity(rows);
    for row in data.chunks_exact(cols) {
        values.push(dot(row, vector)?);
    }

    Ok(F32MatVecOutput {
        values,
        backend: F32MatVecBackend::Scalar,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 0.0001,
            "expected {actual} to be within 0.0001 of {expected}; diff={diff}"
        );
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn f32_matvec_uses_neon_backend_on_aarch64() -> Result<(), InferenceError> {
        let output = f32_mul_vec(
            2,
            8,
            &[
                1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0, //
                0.5, 0.25, -0.5, -0.25, 2.0, 3.0, -2.0, -3.0,
            ],
            &[1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 1.5, -1.5],
        )?;

        assert_eq!(output.backend, F32MatVecBackend::Aarch64Neon);
        assert_close(output.values[0], -1.0);
        assert_close(output.values[1], 0.75);
        Ok(())
    }
}

#[cfg(target_arch = "aarch64")]
mod aarch64 {
    use super::{F32MatVecBackend, F32MatVecOutput};
    use std::arch::aarch64::{vaddvq_f32, vdupq_n_f32, vfmaq_f32, vld1q_f32};

    pub(super) fn neon_f32_mul_vec(
        rows: usize,
        cols: usize,
        data: &[f32],
        vector: &[f32],
    ) -> F32MatVecOutput {
        let mut values = Vec::with_capacity(rows);
        for row in data.chunks_exact(cols) {
            values.push(neon_dot(row, vector));
        }

        F32MatVecOutput {
            values,
            backend: F32MatVecBackend::Aarch64Neon,
        }
    }

    fn neon_dot(left: &[f32], right: &[f32]) -> f32 {
        let chunked_len = left.len() - (left.len() % 4);
        // SAFETY: callers pass equally sized slices; `chunked_len` is rounded
        // down to a multiple of four, so every 4-lane load is in bounds.
        let mut sum = unsafe { neon_dot_chunked(left.as_ptr(), right.as_ptr(), chunked_len) };

        for index in chunked_len..left.len() {
            sum += left[index] * right[index];
        }
        sum
    }

    #[target_feature(enable = "neon")]
    unsafe fn neon_dot_chunked(left: *const f32, right: *const f32, len: usize) -> f32 {
        let mut lanes = vdupq_n_f32(0.0);
        let mut index = 0usize;
        while index < len {
            let left_lanes = vld1q_f32(left.add(index));
            let right_lanes = vld1q_f32(right.add(index));
            lanes = vfmaq_f32(lanes, left_lanes, right_lanes);
            index += 4;
        }
        vaddvq_f32(lanes)
    }
}
