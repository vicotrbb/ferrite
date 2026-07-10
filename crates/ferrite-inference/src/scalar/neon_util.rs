//! Shared NEON helpers for quantized block-dot kernels.
#![allow(
    unsafe_code,
    reason = "audited aarch64 half conversion intrinsics are isolated in this module"
)]

use std::arch::aarch64::{
    float32x4_t, int8x16_t, vcvt_f32_f16, vcvtq_f32_s32, vdup_n_u16, vget_high_s16, vget_high_s8,
    vget_low_s16, vget_low_s8, vgetq_lane_f32, vmovl_s16, vmovl_s8, vreinterpret_f16_u16,
};

/// Converts an IEEE-754 binary16 scale with AArch64's native widening
/// instruction. This avoids repeating scalar exponent/mantissa decoding in
/// every quantized block-dot kernel.
#[inline(always)]
pub(super) unsafe fn native_f16_bits_to_f32(bits: u16) -> f32 {
    // SAFETY: this module is aarch64-only, NEON is part of the baseline, and
    // these register-only intrinsics have no pointer or alignment preconditions.
    unsafe { vgetq_lane_f32::<0>(vcvt_f32_f16(vreinterpret_f16_u16(vdup_n_u16(bits)))) }
}

/// Widens 16 signed bytes into four 4-lane f32 vectors (exact conversion
/// for the -64..64 magnitudes quantized formats produce).
#[target_feature(enable = "neon")]
pub(super) unsafe fn widen_s8_lanes(values: int8x16_t) -> [float32x4_t; 4] {
    let low_half = vmovl_s8(vget_low_s8(values));
    let high_half = vmovl_s8(vget_high_s8(values));
    [
        vcvtq_f32_s32(vmovl_s16(vget_low_s16(low_half))),
        vcvtq_f32_s32(vmovl_s16(vget_high_s16(low_half))),
        vcvtq_f32_s32(vmovl_s16(vget_low_s16(high_half))),
        vcvtq_f32_s32(vmovl_s16(vget_high_s16(high_half))),
    ]
}

#[cfg(test)]
mod tests {
    use super::{native_f16_bits_to_f32, widen_s8_lanes};
    use crate::scalar::float::f16_bits_to_f32;
    use std::arch::aarch64::{vld1q_s8, vst1q_f32};

    #[test]
    fn native_f16_conversion_matches_every_finite_value() {
        for bits in 0..=u16::MAX {
            let expected = f16_bits_to_f32(bits);
            if expected.is_finite() {
                // SAFETY: the test runs only on aarch64, where NEON is baseline.
                let actual = unsafe { native_f16_bits_to_f32(bits) };
                assert_eq!(actual.to_bits(), expected.to_bits(), "bits={bits:#06x}");
            }
        }
    }

    #[test]
    fn signed_byte_widening_is_exact_for_every_value() {
        for start in (i8::MIN..=i8::MAX).step_by(16) {
            let input = std::array::from_fn::<_, 16, _>(|lane| start.wrapping_add(lane as i8));
            // SAFETY: `input` contains the 16 lanes loaded by the intrinsic,
            // and this test module is compiled only for aarch64.
            let quads = unsafe { widen_s8_lanes(vld1q_s8(input.as_ptr())) };
            let mut actual = [0.0f32; 16];
            for (index, quad) in quads.into_iter().enumerate() {
                // SAFETY: each store writes four lanes into one disjoint
                // four-value window of the 16-value output array.
                unsafe { vst1q_f32(actual[index * 4..].as_mut_ptr(), quad) };
            }
            for (actual, expected) in actual.into_iter().zip(input) {
                assert_eq!(actual.to_bits(), f32::from(expected).to_bits());
            }
        }
    }
}
