#![allow(
    unsafe_code,
    reason = "audited dense-16 SIMD loads are bounds-checked before architecture dispatch"
)]

use super::{
    tensor::{bf16_bits_to_f32, f16_bits_to_f32},
    InferenceError, ScalarExecutionOptions,
};

pub(super) fn f16_mul_vec_with_options(
    data: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
    options: ScalarExecutionOptions,
) -> Result<Vec<f32>, InferenceError> {
    validate_dense16_matvec("F16", data, rows, cols, vector)?;
    let dispatch = options.kernel_dispatch();

    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    if dispatch.neon() {
        // SAFETY: the shape check above proves every row and activation load is
        // in bounds. Runtime dispatch proves NEON support before entry.
        let output = unsafe { aarch64::neon_f16_mul_vec(data, rows, cols, vector) };
        return validate_output("F16", output);
    }
    #[cfg(target_arch = "x86_64")]
    if dispatch.avx2() && dispatch.f16c() {
        // SAFETY: the shape check above proves every 16-byte weight load and
        // eight-lane activation load is in bounds. Dispatch proves AVX2 and
        // F16C support before entry.
        let output = unsafe { x86_64::avx2_f16_mul_vec(data, rows, cols, vector) };
        return validate_output("F16", output);
    }

    scalar_dense16_mul_vec(data, rows, cols, vector, f16_bits_to_f32)
}

pub(super) fn bf16_mul_vec_with_options(
    data: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
    options: ScalarExecutionOptions,
) -> Result<Vec<f32>, InferenceError> {
    validate_dense16_matvec("BF16", data, rows, cols, vector)?;
    let dispatch = options.kernel_dispatch();

    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    if dispatch.neon() {
        // SAFETY: the shape check above proves every row and activation load is
        // in bounds. Runtime dispatch proves NEON support before entry.
        let output = unsafe { aarch64::neon_bf16_mul_vec(data, rows, cols, vector) };
        return validate_output("BF16", output);
    }
    #[cfg(target_arch = "x86_64")]
    if dispatch.avx2() {
        // SAFETY: the shape check above proves every 16-byte weight load and
        // eight-lane activation load is in bounds. Dispatch proves AVX2
        // support before entry.
        let output = unsafe { x86_64::avx2_bf16_mul_vec(data, rows, cols, vector) };
        return validate_output("BF16", output);
    }

    scalar_dense16_mul_vec(data, rows, cols, vector, bf16_bits_to_f32)
}

pub(super) fn validate_f16_values(data: &[u8]) -> Result<(), InferenceError> {
    validate_dense16_values("F16", data, f16_bits_to_f32)
}

pub(super) fn validate_bf16_values(data: &[u8]) -> Result<(), InferenceError> {
    validate_dense16_values("BF16", data, bf16_bits_to_f32)
}

pub(super) fn f16_row_values(data: &[u8], cols: usize, index: usize) -> Vec<f32> {
    dense16_row_values(data, cols, index, f16_bits_to_f32)
}

pub(super) fn bf16_row_values(data: &[u8], cols: usize, index: usize) -> Vec<f32> {
    dense16_row_values(data, cols, index, bf16_bits_to_f32)
}

fn validate_dense16_matvec(
    label: &str,
    data: &[u8],
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
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new(format!(
            "{label} matvec vector values must be finite"
        )));
    }
    let expected = rows
        .checked_mul(cols)
        .and_then(|values| values.checked_mul(2))
        .ok_or_else(|| InferenceError::new(format!("{label} matrix byte length overflow")))?;
    if data.len() != expected {
        return Err(InferenceError::new(format!(
            "{label} matrix byte length {} does not match shape {rows}x{cols}",
            data.len()
        )));
    }
    Ok(())
}

fn validate_dense16_values(
    label: &str,
    data: &[u8],
    decode: fn(u16) -> f32,
) -> Result<(), InferenceError> {
    if !data.len().is_multiple_of(2) {
        return Err(InferenceError::new(format!(
            "{label} matrix byte length {} is not divisible by 2",
            data.len()
        )));
    }
    for (index, bytes) in data.chunks_exact(2).enumerate() {
        let value = decode(u16::from_le_bytes([bytes[0], bytes[1]]));
        if !value.is_finite() {
            return Err(InferenceError::new(format!(
                "{label} matrix value {index} must be finite"
            )));
        }
    }
    Ok(())
}

fn dense16_row_values(data: &[u8], cols: usize, index: usize, decode: fn(u16) -> f32) -> Vec<f32> {
    let start = index * cols * 2;
    data[start..start + cols * 2]
        .chunks_exact(2)
        .map(|bytes| decode(u16::from_le_bytes([bytes[0], bytes[1]])))
        .collect()
}

fn scalar_dense16_mul_vec(
    data: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
    decode: fn(u16) -> f32,
) -> Result<Vec<f32>, InferenceError> {
    let row_bytes = cols * 2;
    let mut output = Vec::with_capacity(rows);
    for row in data.chunks_exact(row_bytes) {
        let mut sum = 0.0f32;
        for (bytes, activation) in row.chunks_exact(2).zip(vector) {
            let weight = decode(u16::from_le_bytes([bytes[0], bytes[1]]));
            let product = weight * activation;
            if !product.is_finite() {
                return Err(InferenceError::new("dense-16 matvec result must be finite"));
            }
            sum += product;
            if !sum.is_finite() {
                return Err(InferenceError::new("dense-16 matvec result must be finite"));
            }
        }
        output.push(sum);
    }
    Ok(output)
}

fn validate_output(label: &str, output: Vec<f32>) -> Result<Vec<f32>, InferenceError> {
    if output.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new(format!(
            "{label} matvec result must be finite"
        )));
    }
    Ok(output)
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
mod aarch64 {
    use std::arch::aarch64::{
        vaddvq_f32, vcvt_f32_f16, vdupq_n_f32, vfmaq_f32, vld1_u16, vld1q_f32, vmovl_u16,
        vreinterpret_f16_u16, vreinterpretq_f32_u32, vshlq_n_u32,
    };

    use super::{bf16_bits_to_f32, f16_bits_to_f32};

    #[target_feature(enable = "neon")]
    pub(super) unsafe fn neon_f16_mul_vec(
        data: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> Vec<f32> {
        // SAFETY: the public module entry validates all slice and shape
        // invariants before entering this NEON target-feature function.
        unsafe { dense16_mul_vec(data, rows, cols, vector, f16_dot) }
    }

    #[target_feature(enable = "neon")]
    pub(super) unsafe fn neon_bf16_mul_vec(
        data: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> Vec<f32> {
        // SAFETY: the public module entry validates all slice and shape
        // invariants before entering this NEON target-feature function.
        unsafe { dense16_mul_vec(data, rows, cols, vector, bf16_dot) }
    }

    #[target_feature(enable = "neon")]
    unsafe fn dense16_mul_vec(
        data: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
        dot: unsafe fn(&[u8], &[f32]) -> f32,
    ) -> Vec<f32> {
        let row_bytes = cols * 2;
        let mut output = Vec::with_capacity(rows);
        for row in data.chunks_exact(row_bytes) {
            // SAFETY: callers validate each row and vector length before
            // entering this target-feature function.
            output.push(unsafe { dot(row, vector) });
        }
        output
    }

    #[target_feature(enable = "neon")]
    unsafe fn f16_dot(row: &[u8], vector: &[f32]) -> f32 {
        let chunked_len = vector.len() - (vector.len() % 4);
        let mut lanes = vdupq_n_f32(0.0);
        let mut index = 0usize;
        while index < chunked_len {
            // SAFETY: the caller validates `row.len() == vector.len() * 2`.
            // NEON loads support unaligned addresses and advance only through
            // complete four-value chunks.
            let (weights, activations) = unsafe {
                let bits = vld1_u16(row.as_ptr().add(index * 2).cast::<u16>());
                (
                    vcvt_f32_f16(vreinterpret_f16_u16(bits)),
                    vld1q_f32(vector.as_ptr().add(index)),
                )
            };
            lanes = vfmaq_f32(lanes, weights, activations);
            index += 4;
        }
        let mut sum = vaddvq_f32(lanes);
        for (tail, activation) in vector.iter().enumerate().skip(index) {
            let offset = tail * 2;
            let bits = u16::from_le_bytes([row[offset], row[offset + 1]]);
            sum += f16_bits_to_f32(bits) * activation;
        }
        sum
    }

    #[target_feature(enable = "neon")]
    unsafe fn bf16_dot(row: &[u8], vector: &[f32]) -> f32 {
        let chunked_len = vector.len() - (vector.len() % 4);
        let mut lanes = vdupq_n_f32(0.0);
        let mut index = 0usize;
        while index < chunked_len {
            // SAFETY: the caller validates `row.len() == vector.len() * 2`.
            // NEON loads support unaligned addresses and advance only through
            // complete four-value chunks.
            let (weights, activations) = unsafe {
                let bits = vld1_u16(row.as_ptr().add(index * 2).cast::<u16>());
                let widened = vmovl_u16(bits);
                (
                    vreinterpretq_f32_u32(vshlq_n_u32::<16>(widened)),
                    vld1q_f32(vector.as_ptr().add(index)),
                )
            };
            lanes = vfmaq_f32(lanes, weights, activations);
            index += 4;
        }
        let mut sum = vaddvq_f32(lanes);
        for (tail, activation) in vector.iter().enumerate().skip(index) {
            let offset = tail * 2;
            let bits = u16::from_le_bytes([row[offset], row[offset + 1]]);
            sum += bf16_bits_to_f32(bits) * activation;
        }
        sum
    }
}

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use std::arch::x86_64::{
        __m128i, _mm256_add_ps, _mm256_castsi256_ps, _mm256_cvtepu16_epi32, _mm256_cvtph_ps,
        _mm256_loadu_ps, _mm256_mul_ps, _mm256_setzero_ps, _mm256_slli_epi32, _mm256_storeu_ps,
        _mm_loadu_si128,
    };

    use super::{bf16_bits_to_f32, f16_bits_to_f32};

    #[target_feature(enable = "avx2,f16c")]
    pub(super) unsafe fn avx2_f16_mul_vec(
        data: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> Vec<f32> {
        // SAFETY: the public module entry validates all slice and shape
        // invariants before entering this AVX2/F16C target-feature function.
        unsafe { dense16_mul_vec(data, rows, cols, vector, f16_dot) }
    }

    #[target_feature(enable = "avx2")]
    pub(super) unsafe fn avx2_bf16_mul_vec(
        data: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
    ) -> Vec<f32> {
        // SAFETY: the public module entry validates all slice and shape
        // invariants before entering this AVX2 target-feature function.
        unsafe { dense16_mul_vec(data, rows, cols, vector, bf16_dot) }
    }

    unsafe fn dense16_mul_vec(
        data: &[u8],
        rows: usize,
        cols: usize,
        vector: &[f32],
        dot: unsafe fn(&[u8], &[f32]) -> f32,
    ) -> Vec<f32> {
        let row_bytes = cols * 2;
        let mut output = Vec::with_capacity(rows);
        for row in data.chunks_exact(row_bytes) {
            // SAFETY: callers validate each row and vector length before
            // entering the corresponding target-feature function.
            output.push(unsafe { dot(row, vector) });
        }
        output
    }

    #[target_feature(enable = "avx2,f16c")]
    unsafe fn f16_dot(row: &[u8], vector: &[f32]) -> f32 {
        let chunked_len = vector.len() - (vector.len() % 8);
        let mut lanes = _mm256_setzero_ps();
        let mut index = 0usize;
        while index < chunked_len {
            // SAFETY: the caller validates `row.len() == vector.len() * 2`.
            // The unaligned loads advance only through complete eight-value
            // chunks.
            let (weights, activations) = unsafe {
                let bits = _mm_loadu_si128(row.as_ptr().add(index * 2).cast::<__m128i>());
                (
                    _mm256_cvtph_ps(bits),
                    _mm256_loadu_ps(vector.as_ptr().add(index)),
                )
            };
            lanes = _mm256_add_ps(lanes, _mm256_mul_ps(weights, activations));
            index += 8;
        }
        finish_dot(row, vector, index, lanes, f16_bits_to_f32)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn bf16_dot(row: &[u8], vector: &[f32]) -> f32 {
        let chunked_len = vector.len() - (vector.len() % 8);
        let mut lanes = _mm256_setzero_ps();
        let mut index = 0usize;
        while index < chunked_len {
            // SAFETY: the caller validates `row.len() == vector.len() * 2`.
            // The unaligned loads advance only through complete eight-value
            // chunks.
            let (weights, activations) = unsafe {
                let bits = _mm_loadu_si128(row.as_ptr().add(index * 2).cast::<__m128i>());
                let widened = _mm256_cvtepu16_epi32(bits);
                (
                    _mm256_castsi256_ps(_mm256_slli_epi32::<16>(widened)),
                    _mm256_loadu_ps(vector.as_ptr().add(index)),
                )
            };
            lanes = _mm256_add_ps(lanes, _mm256_mul_ps(weights, activations));
            index += 8;
        }
        finish_dot(row, vector, index, lanes, bf16_bits_to_f32)
    }

    #[target_feature(enable = "avx2")]
    fn finish_dot(
        row: &[u8],
        vector: &[f32],
        start: usize,
        lanes: std::arch::x86_64::__m256,
        decode: fn(u16) -> f32,
    ) -> f32 {
        let mut partial = [0.0f32; 8];
        // SAFETY: `partial` contains eight writable lanes and the unaligned
        // store requires no stronger alignment.
        unsafe { _mm256_storeu_ps(partial.as_mut_ptr(), lanes) };
        let mut sum = partial.iter().sum::<f32>();
        for (tail, activation) in vector.iter().enumerate().skip(start) {
            let offset = tail * 2;
            let bits = u16::from_le_bytes([row[offset], row[offset + 1]]);
            sum += decode(bits) * activation;
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scalar::{matvec::f32_mul_vec_with_options, KernelProvider, ScalarExecutionOptions};
    use crate::scalar::{Matrix, MatrixStorageKind};

    fn f16_bytes(bits: &[u16]) -> Vec<u8> {
        bits.iter().flat_map(|value| value.to_le_bytes()).collect()
    }

    fn bf16_bytes(values: &[f32]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| ((value.to_bits() >> 16) as u16).to_le_bytes())
            .collect()
    }

    #[test]
    fn dense16_kernels_match_existing_f32_accumulation() -> Result<(), InferenceError> {
        let f16_bits = [
            0x3c00, 0xc000, 0x3800, 0x0000, 0x4400, 0xb400, 0x4000, 0xbc00, 0x3400, 0x3e00, 0xc200,
            0x4200, 0x3c00, 0x3800, 0xc000, 0x4400,
        ];
        let f16_values = f16_bits.map(f16_bits_to_f32);
        let bf16_values = [
            1.0, -2.0, 0.5, 0.0, 4.0, -0.25, 2.0, -1.0, 0.25, 1.5, -3.0, 3.0, 1.0, 0.5, -2.0, 4.0,
        ];
        let vector = [0.5, -1.0, 2.0, 0.25, -0.5, 1.5, 0.75, -2.0];

        for provider in [KernelProvider::Portable, KernelProvider::Auto] {
            let options = ScalarExecutionOptions::default().with_kernel_provider(provider);
            let f16_expected =
                f32_mul_vec_with_options(2, 8, &f16_values, &vector, options)?.into_values();
            let bf16_expected =
                f32_mul_vec_with_options(2, 8, &bf16_values, &vector, options)?.into_values();

            assert_eq!(
                f16_mul_vec_with_options(&f16_bytes(&f16_bits), 2, 8, &vector, options)?,
                f16_expected
            );
            assert_eq!(
                bf16_mul_vec_with_options(&bf16_bytes(&bf16_values), 2, 8, &vector, options)?,
                bf16_expected
            );
        }
        Ok(())
    }

    #[test]
    fn dense16_validation_rejects_non_finite_values() {
        assert!(validate_f16_values(&0x7e00u16.to_le_bytes()).is_err());
        assert!(validate_bf16_values(&0x7fc0u16.to_le_bytes()).is_err());
    }

    #[test]
    fn matrix_dense16_storage_preserves_rows_and_byte_accounting() -> Result<(), InferenceError> {
        let f16_bits = [
            0x3c00, 0xc000, 0x3800, 0x0000, 0x4400, 0xb400, 0x4000, 0xbc00,
        ];
        let f16 = Matrix::from_f16_row_major_bytes(2, 4, f16_bytes(&f16_bits))?;
        assert_eq!(f16.storage_kind(), MatrixStorageKind::F16);
        assert_eq!(f16.storage_bytes(), 16);
        assert_eq!(f16.row_values(1)?, vec![4.0, -0.25, 2.0, -1.0]);
        let selected = f16.row_range(1..2)?;
        assert_eq!(selected.storage_kind(), MatrixStorageKind::F16);
        assert_eq!(selected.storage_bytes(), 8);
        assert_eq!(selected.row_values(0)?, f16.row_values(1)?);

        let bf16_values = [1.0, -2.0, 0.5, 0.0, 4.0, -0.25, 2.0, -1.0];
        let bf16 = Matrix::from_bf16_row_major_bytes(2, 4, bf16_bytes(&bf16_values))?;
        assert_eq!(bf16.storage_kind(), MatrixStorageKind::BF16);
        assert_eq!(bf16.storage_bytes(), 16);
        assert_eq!(bf16.row_values(0)?, bf16_values[..4]);
        Ok(())
    }
}
