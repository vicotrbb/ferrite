#![allow(
    unsafe_code,
    reason = "audited architecture-specific SIMD intrinsics are isolated in this module"
)]

use super::{InferenceError, ScalarExecutionOptions, kernels::KernelDispatch, math::dot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum F32MatVecBackend {
    Scalar,
    #[cfg(target_arch = "aarch64")]
    Aarch64Neon,
    #[cfg(target_arch = "x86_64")]
    X86_64Avx2,
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

#[cfg(test)]
pub(super) fn f32_mul_vec(
    rows: usize,
    cols: usize,
    data: &[f32],
    vector: &[f32],
) -> Result<F32MatVecOutput, InferenceError> {
    f32_mul_vec_with_options(rows, cols, data, vector, ScalarExecutionOptions::default())
}

pub(super) fn f32_mul_vec_with_options(
    rows: usize,
    cols: usize,
    data: &[f32],
    vector: &[f32],
    options: ScalarExecutionOptions,
) -> Result<F32MatVecOutput, InferenceError> {
    f32_mul_vec_with_dispatch(rows, cols, data, vector, options.kernel_dispatch())
}

fn f32_mul_vec_with_dispatch(
    rows: usize,
    cols: usize,
    data: &[f32],
    vector: &[f32],
    dispatch: KernelDispatch,
) -> Result<F32MatVecOutput, InferenceError> {
    if vector.len() != cols {
        return Err(InferenceError::new(format!(
            "matrix columns {cols} do not match vector length {}",
            vector.len()
        )));
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new(
            "F32 matvec vector values must be finite",
        ));
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
        if dispatch.neon() {
            return Ok(aarch64::neon_f32_mul_vec(rows, cols, data, vector));
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if dispatch.avx2() {
            return Ok(x86_64::avx2_f32_mul_vec(rows, cols, data, vector));
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

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn f32_matvec_uses_detected_backend_on_x86_64() -> Result<(), InferenceError> {
        let output = f32_mul_vec(
            2,
            8,
            &[
                1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0, //
                0.5, 0.25, -0.5, -0.25, 2.0, 3.0, -2.0, -3.0,
            ],
            &[1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 1.5, -1.5],
        )?;

        let expected_backend = if crate::scalar::CpuKernelCapabilities::detect().avx2() {
            F32MatVecBackend::X86_64Avx2
        } else {
            F32MatVecBackend::Scalar
        };
        assert_eq!(output.backend, expected_backend);
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
            // SAFETY: the caller guarantees that both pointers address at
            // least `len` values, and the loop advances in four-value chunks.
            let (left_lanes, right_lanes) =
                unsafe { (vld1q_f32(left.add(index)), vld1q_f32(right.add(index))) };
            lanes = vfmaq_f32(lanes, left_lanes, right_lanes);
            index += 4;
        }
        vaddvq_f32(lanes)
    }
}

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use super::{F32MatVecBackend, F32MatVecOutput};
    use std::arch::x86_64::{
        _mm256_add_ps, _mm256_loadu_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_storeu_ps,
    };

    pub(super) fn avx2_f32_mul_vec(
        rows: usize,
        cols: usize,
        data: &[f32],
        vector: &[f32],
    ) -> F32MatVecOutput {
        let mut values = Vec::with_capacity(rows);
        for row in data.chunks_exact(cols) {
            values.push(avx2_dot(row, vector));
        }

        F32MatVecOutput {
            values,
            backend: F32MatVecBackend::X86_64Avx2,
        }
    }

    fn avx2_dot(left: &[f32], right: &[f32]) -> f32 {
        let chunked_len = left.len() - (left.len() % 8);
        // SAFETY: callers pass equally sized slices; `chunked_len` is rounded
        // down to a multiple of eight, so every 8-lane unaligned load is in
        // bounds. The public caller checks AVX2 support before dispatching.
        let mut sum = unsafe { avx2_dot_chunked(left.as_ptr(), right.as_ptr(), chunked_len) };

        for index in chunked_len..left.len() {
            sum += left[index] * right[index];
        }
        sum
    }

    #[target_feature(enable = "avx2")]
    unsafe fn avx2_dot_chunked(left: *const f32, right: *const f32, len: usize) -> f32 {
        let mut lanes = _mm256_setzero_ps();
        let mut index = 0usize;
        while index < len {
            // SAFETY: the caller guarantees that both pointers address at
            // least `len` values, and the loop advances in eight-value
            // chunks. Unaligned loads are valid for these intrinsics.
            let (left_lanes, right_lanes) = unsafe {
                (
                    _mm256_loadu_ps(left.add(index)),
                    _mm256_loadu_ps(right.add(index)),
                )
            };
            lanes = _mm256_add_ps(lanes, _mm256_mul_ps(left_lanes, right_lanes));
            index += 8;
        }

        let mut partial = [0.0f32; 8];
        // SAFETY: `partial` contains exactly eight writable `f32` lanes, and
        // the unaligned store does not require a stronger alignment.
        unsafe { _mm256_storeu_ps(partial.as_mut_ptr(), lanes) };
        partial.iter().sum()
    }
}
