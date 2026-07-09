//! Shared NEON helpers for quantized block-dot kernels.
#![allow(unsafe_code)]

use std::arch::aarch64::{
    float32x4_t, int8x16_t, vcvtq_f32_s32, vget_high_s16, vget_high_s8, vget_low_s16, vget_low_s8,
    vmovl_s16, vmovl_s8,
};

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
