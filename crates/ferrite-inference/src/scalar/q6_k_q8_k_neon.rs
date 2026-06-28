#![allow(unsafe_code)]

use super::{float::f16_bits_to_f32, q6_k::Q6_K_BLOCK_BYTES, q8_k::BlockQ8K, InferenceError};
use std::arch::aarch64::{
    int32x4_t, int8x16_t, vaddq_s32, vaddvq_s32, vget_high_s8, vget_low_s8, vld1q_s8, vmull_s8,
    vpaddlq_s16,
};

pub(in crate::scalar) fn neon_q6_k_q8_k_block_dot(
    block: &[u8],
    activation: &BlockQ8K,
) -> Result<f32, InferenceError> {
    if block.len() != Q6_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q6_K block byte length {} does not match {Q6_K_BLOCK_BYTES}",
            block.len()
        )));
    }

    // SAFETY: this module is compiled only for aarch64, where NEON is part of
    // the baseline architecture. `block` has been length-checked above, and
    // `BlockQ8K` always owns exactly one 256-value activation block.
    Ok(unsafe { neon_q6_k_q8_k_block_dot_unchecked(block, activation) })
}

#[target_feature(enable = "neon")]
unsafe fn neon_q6_k_q8_k_block_dot_unchecked(block: &[u8], activation: &BlockQ8K) -> f32 {
    let low_bits = &block[0..128];
    let high_bits = &block[128..192];
    let scales = &block[192..208];
    let super_scale = f16_bits_to_f32(u16::from_le_bytes([block[208], block[209]]));
    let mut weighted_sum = 0i32;
    let mut correction_sum = 0i32;

    for half in 0..2 {
        let value_base = half * 128;
        let low_base = half * 64;
        let high_base = half * 32;
        let scale_base = half * 8;

        for group_half in 0..2 {
            let index_base = group_half * 16;
            let scale_index = scale_base + group_half;
            weighted_sum += i32::from(scales[scale_index] as i8)
                * q6_group_dot(
                    low_bits,
                    high_bits,
                    low_base,
                    high_base,
                    index_base,
                    Q6LaneGroup::Q1,
                    activation.qs.as_ptr().add(value_base + index_base),
                );
            correction_sum += i32::from(scales[scale_index] as i8)
                * i32::from(activation.bsums[(value_base + index_base) / 16]);

            let scale_index = scale_base + group_half + 2;
            weighted_sum += i32::from(scales[scale_index] as i8)
                * q6_group_dot(
                    low_bits,
                    high_bits,
                    low_base,
                    high_base,
                    index_base,
                    Q6LaneGroup::Q2,
                    activation.qs.as_ptr().add(value_base + index_base + 32),
                );
            correction_sum += i32::from(scales[scale_index] as i8)
                * i32::from(activation.bsums[(value_base + index_base + 32) / 16]);

            let scale_index = scale_base + group_half + 4;
            weighted_sum += i32::from(scales[scale_index] as i8)
                * q6_group_dot(
                    low_bits,
                    high_bits,
                    low_base,
                    high_base,
                    index_base,
                    Q6LaneGroup::Q3,
                    activation.qs.as_ptr().add(value_base + index_base + 64),
                );
            correction_sum += i32::from(scales[scale_index] as i8)
                * i32::from(activation.bsums[(value_base + index_base + 64) / 16]);

            let scale_index = scale_base + group_half + 6;
            weighted_sum += i32::from(scales[scale_index] as i8)
                * q6_group_dot(
                    low_bits,
                    high_bits,
                    low_base,
                    high_base,
                    index_base,
                    Q6LaneGroup::Q4,
                    activation.qs.as_ptr().add(value_base + index_base + 96),
                );
            correction_sum += i32::from(scales[scale_index] as i8)
                * i32::from(activation.bsums[(value_base + index_base + 96) / 16]);
        }
    }

    activation.d * super_scale * (weighted_sum - 32 * correction_sum) as f32
}

#[derive(Clone, Copy)]
enum Q6LaneGroup {
    Q1,
    Q2,
    Q3,
    Q4,
}

#[target_feature(enable = "neon")]
unsafe fn q6_group_dot(
    low_bits: &[u8],
    high_bits: &[u8],
    low_base: usize,
    high_base: usize,
    index_base: usize,
    lane_group: Q6LaneGroup,
    q8: *const i8,
) -> i32 {
    let mut q6 = [0i8; 16];
    for (lane, target) in q6.iter_mut().enumerate() {
        let offset = index_base + lane;
        let high = high_bits[high_base + offset];
        let raw = match lane_group {
            Q6LaneGroup::Q1 => (low_bits[low_base + offset] & 0x0f) | ((high & 3) << 4),
            Q6LaneGroup::Q2 => (low_bits[low_base + offset + 32] & 0x0f) | (((high >> 2) & 3) << 4),
            Q6LaneGroup::Q3 => (low_bits[low_base + offset] >> 4) | (((high >> 4) & 3) << 4),
            Q6LaneGroup::Q4 => (low_bits[low_base + offset + 32] >> 4) | (((high >> 6) & 3) << 4),
        };
        *target = raw as i8;
    }

    let q6_lanes = vld1q_s8(q6.as_ptr());
    let q8_lanes = vld1q_s8(q8);
    vaddvq_s32(dot_i8x16(q6_lanes, q8_lanes))
}

#[target_feature(enable = "neon")]
unsafe fn dot_i8x16(left: int8x16_t, right: int8x16_t) -> int32x4_t {
    let products_low = vmull_s8(vget_low_s8(left), vget_low_s8(right));
    let products_high = vmull_s8(vget_high_s8(left), vget_high_s8(right));
    vaddq_s32(vpaddlq_s16(products_low), vpaddlq_s16(products_high))
}

#[cfg(test)]
mod tests {
    use super::neon_q6_k_q8_k_block_dot;
    use crate::scalar::{
        q6_k::Q6_K_BLOCK_VALUES, q6_k_q8_k::q6_k_q8_k_block_dot, q8_k::BlockQ8K, InferenceError,
    };

    #[test]
    fn neon_q6_k_q8_k_block_dot_matches_scalar_q8_k_dot() -> Result<(), InferenceError> {
        let block = patterned_q6_k_block();
        let vector = patterned_activation();
        let activation = BlockQ8K::quantize(&vector)?;

        let actual = neon_q6_k_q8_k_block_dot(&block, &activation)?;
        let expected = q6_k_q8_k_block_dot(&block, &activation)?;

        assert!(
            (actual - expected).abs() < 0.001,
            "actual={actual} expected={expected}"
        );
        Ok(())
    }

    fn patterned_q6_k_block() -> Vec<u8> {
        let mut block = Vec::new();
        block.extend((0..128).map(|index| (index * 37 + 11) as u8));
        block.extend((0..64).map(|index| (index * 19 + 7) as u8));
        block.extend(
            [-4i8, 3, -6, 5, -8, 7, -10, 9, 10, -9, 8, -7, 6, -5, 4, -3].map(|value| value as u8),
        );
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block
    }

    fn patterned_activation() -> [f32; Q6_K_BLOCK_VALUES] {
        let mut values = [0.0; Q6_K_BLOCK_VALUES];
        for (index, value) in values.iter_mut().enumerate() {
            let wave = (index % 43) as f32 - 21.0;
            *value = wave / 17.0;
        }
        values
    }
}
