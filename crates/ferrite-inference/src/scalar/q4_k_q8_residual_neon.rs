//! Experimental Q4_K × two-pass residual-Q8_K matvec for FEAT_DotProd CPUs.
#![allow(
    unsafe_code,
    reason = "audited aarch64 SIMD intrinsics are isolated in this kernel module"
)]

use super::{
    neon_util::native_f16_bits_to_f32,
    q4_k::{q4_k_storage_bytes, Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES},
    q8_k::BlockQ8K,
    q8_residual_activation::BlockQ8KResidual,
    InferenceError,
};
use rayon::prelude::*;
use std::arch::aarch64::{
    int32x4_t, int8x16_t, uint8x16_t, vaddq_s32, vaddvq_s32, vandq_u8, vdupq_n_s32, vdupq_n_u8,
    vld1q_s8, vld1q_u8, vreinterpretq_s8_u8, vshrq_n_u8,
};
use std::arch::asm;

pub(super) fn neon_q4_k_q8_residual_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Result<Vec<f32>, InferenceError> {
    if cols == 0 || !cols.is_multiple_of(Q4_K_BLOCK_VALUES) || vector.len() != cols {
        return Err(InferenceError::new(
            "invalid Q4_K x residual-Q8_K matvec shape",
        ));
    }
    if bytes.len() != q4_k_storage_bytes(rows * cols)? {
        return Err(InferenceError::new(
            "invalid Q4_K x residual-Q8_K matrix byte length",
        ));
    }
    let activation = BlockQ8KResidual::quantize_blocks(vector)?;
    let row_bytes = cols / Q4_K_BLOCK_VALUES * Q4_K_BLOCK_BYTES;
    bytes
        .par_chunks_exact(row_bytes)
        .with_min_len(64)
        .map(|row| {
            row.chunks_exact(Q4_K_BLOCK_BYTES)
                .zip(&activation)
                // SAFETY: exact-sized blocks and FEAT_DotProd dispatch.
                .map(|(weights, activation)| unsafe { block_dot(weights, activation) })
                .sum()
        })
        .collect()
}

#[target_feature(enable = "dotprod")]
unsafe fn block_dot(weights: &[u8], activation: &BlockQ8KResidual) -> Result<f32, InferenceError> {
    if weights.len() != Q4_K_BLOCK_BYTES {
        return Err(InferenceError::new(
            "invalid Q4_K residual-Q8_K block length",
        ));
    }
    // SAFETY: FEAT_DotProd implies NEON, and the length check above makes the
    // two half-precision scale reads valid.
    let (d, dmin) = unsafe {
        (
            native_f16_bits_to_f32(u16::from_le_bytes([weights[0], weights[1]])),
            native_f16_bits_to_f32(u16::from_le_bytes([weights[2], weights[3]])),
        )
    };
    let scales = &weights[4..16];
    let quants = &weights[16..];
    let mut result = 0.0;

    for activation in &activation.passes {
        // SAFETY: this function enables FEAT_DotProd, and each pass owns one
        // complete 256-value Q8_K activation block.
        result += unsafe { block_dot_pass(d, dmin, scales, quants, activation) };
    }
    Ok(result)
}

#[target_feature(enable = "dotprod")]
unsafe fn block_dot_pass(
    d: f32,
    dmin: f32,
    scales: &[u8],
    quants: &[u8],
    activation: &BlockQ8K,
) -> f32 {
    let mask = vdupq_n_u8(0x0f);
    let mut weighted_sum = 0i32;
    let mut min_sum = 0i32;
    let mut scale_index = 0usize;
    let mut activation_offset = 0usize;

    for quant_chunk in quants.chunks_exact(32) {
        let (scale_low, min_low) = q4_k_scale_min(scale_index, scales);
        let (scale_high, min_high) = q4_k_scale_min(scale_index + 1, scales);
        // SAFETY: `quant_chunk` contains 32 packed bytes and the activation
        // offset selects two in-bounds 32-value windows.
        weighted_sum += i32::from(scale_low)
            * unsafe {
                q4_nibble_dot_32(
                    quant_chunk.as_ptr(),
                    activation.qs.as_ptr().add(activation_offset),
                    mask,
                    false,
                )
            };
        // SAFETY: the same validated 32-byte quant chunk and activation block
        // provide the in-bounds high-nibble activation window.
        weighted_sum += i32::from(scale_high)
            * unsafe {
                q4_nibble_dot_32(
                    quant_chunk.as_ptr(),
                    activation.qs.as_ptr().add(activation_offset + 32),
                    mask,
                    true,
                )
            };
        min_sum += i32::from(min_low)
            * (i32::from(activation.bsums[scale_index * 2])
                + i32::from(activation.bsums[scale_index * 2 + 1]));
        min_sum += i32::from(min_high)
            * (i32::from(activation.bsums[(scale_index + 1) * 2])
                + i32::from(activation.bsums[(scale_index + 1) * 2 + 1]));
        scale_index += 2;
        activation_offset += 64;
    }
    activation.d * (d * weighted_sum as f32 - dmin * min_sum as f32)
}

#[target_feature(enable = "dotprod")]
unsafe fn q4_nibble_dot_32(q4: *const u8, q8: *const i8, mask: uint8x16_t, high: bool) -> i32 {
    // SAFETY: callers provide readable 32-value Q4 and Q8 windows, and this
    // function is entered only with FEAT_DotProd enabled.
    unsafe {
        let mut q4_a = vld1q_u8(q4);
        let mut q4_b = vld1q_u8(q4.add(16));
        if high {
            q4_a = vshrq_n_u8(q4_a, 4);
            q4_b = vshrq_n_u8(q4_b, 4);
        } else {
            q4_a = vandq_u8(q4_a, mask);
            q4_b = vandq_u8(q4_b, mask);
        }
        let q8_a = vld1q_s8(q8);
        let q8_b = vld1q_s8(q8.add(16));
        vaddvq_s32(vaddq_s32(
            dot_i8x16(vdupq_n_s32(0), vreinterpretq_s8_u8(q4_a), q8_a),
            dot_i8x16(vdupq_n_s32(0), vreinterpretq_s8_u8(q4_b), q8_b),
        ))
    }
}

#[inline(always)]
unsafe fn dot_i8x16(mut sum: int32x4_t, left: int8x16_t, right: int8x16_t) -> int32x4_t {
    // SAFETY: callers enter only after FEAT_DotProd detection, and the
    // assembly touches registers only, with no memory or stack effects.
    unsafe {
        asm!(
            "sdot {sum:v}.4s, {left:v}.16b, {right:v}.16b",
            sum = inout(vreg) sum,
            left = in(vreg) left,
            right = in(vreg) right,
            options(nostack, nomem, pure),
        );
    }
    sum
}

fn q4_k_scale_min(index: usize, scales: &[u8]) -> (u8, u8) {
    if index < 4 {
        (scales[index] & 63, scales[index + 4] & 63)
    } else {
        (
            (scales[index + 4] & 0x0f) | ((scales[index - 4] >> 6) << 4),
            (scales[index + 4] >> 4) | ((scales[index] >> 6) << 4),
        )
    }
}
