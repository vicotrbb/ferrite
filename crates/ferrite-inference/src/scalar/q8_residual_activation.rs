#![allow(unsafe_code)]

use super::{q8_k::BlockQ8K, InferenceError};

use std::arch::aarch64::{
    vcombine_s16, vcvtq_f32_s32, vcvtq_s32_f32, vdupq_n_f32, vld1q_f32, vmaxq_f32, vminq_f32,
    vmulq_f32, vqmovn_s16, vqmovn_s32, vrndaq_f32, vst1_s8, vst1q_f32, vsubq_f32,
};

pub(super) const Q8_RESIDUAL_BLOCK_VALUES: usize = 32;
pub(super) const Q8_RESIDUAL_PASSES: usize = 2;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BlockQ8Residual {
    pub(super) scales: [f32; Q8_RESIDUAL_PASSES],
    pub(super) quants: [[i8; Q8_RESIDUAL_BLOCK_VALUES]; Q8_RESIDUAL_PASSES],
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BlockQ8KResidual {
    pub(super) passes: [BlockQ8K; Q8_RESIDUAL_PASSES],
}

impl BlockQ8KResidual {
    pub(super) fn quantize_blocks(values: &[f32]) -> Result<Vec<Self>, InferenceError> {
        if values.is_empty() || !values.len().is_multiple_of(256) {
            return Err(InferenceError::new(format!(
                "residual Q8_K activation length {} must be a positive multiple of 256",
                values.len()
            )));
        }
        values.chunks_exact(256).map(Self::quantize).collect()
    }

    fn quantize(values: &[f32]) -> Result<Self, InferenceError> {
        let first = BlockQ8K::quantize(values)?;
        let residual = values
            .iter()
            .zip(first.qs)
            .map(|(value, quantized)| *value - first.d * f32::from(quantized))
            .collect::<Vec<_>>();
        let second = BlockQ8K::quantize(&residual)?;
        Ok(Self {
            passes: [first, second],
        })
    }
}

impl BlockQ8Residual {
    pub(super) fn quantize_blocks(values: &[f32]) -> Result<Vec<Self>, InferenceError> {
        if values.is_empty() || !values.len().is_multiple_of(Q8_RESIDUAL_BLOCK_VALUES) {
            return Err(InferenceError::new(format!(
                "residual Q8 activation length {} must be a positive multiple of {Q8_RESIDUAL_BLOCK_VALUES}",
                values.len()
            )));
        }
        values
            .chunks_exact(Q8_RESIDUAL_BLOCK_VALUES)
            .map(Self::quantize)
            .collect()
    }

    fn quantize(values: &[f32]) -> Result<Self, InferenceError> {
        let mut residual = [0.0; Q8_RESIDUAL_BLOCK_VALUES];
        residual.copy_from_slice(values);
        if let Some(index) = residual.iter().position(|value| !value.is_finite()) {
            return Err(InferenceError::new(format!(
                "residual Q8 activation value {index} is not finite"
            )));
        }

        let mut scales = [0.0; Q8_RESIDUAL_PASSES];
        let mut quants = [[0; Q8_RESIDUAL_BLOCK_VALUES]; Q8_RESIDUAL_PASSES];
        for pass in 0..Q8_RESIDUAL_PASSES {
            let mut signed_max = 0.0f32;
            let mut absolute_max = 0.0f32;
            for value in residual {
                if value.abs() > absolute_max {
                    absolute_max = value.abs();
                    signed_max = value;
                }
            }
            if absolute_max == 0.0 {
                break;
            }
            let inverse_scale = -127.0 / signed_max;
            let scale = 1.0 / inverse_scale;
            if !inverse_scale.is_finite() || !scale.is_finite() {
                return Err(InferenceError::new(
                    "residual Q8 activation scale is not finite",
                ));
            }
            scales[pass] = scale;
            if std::arch::is_aarch64_feature_detected!("neon") {
                // SAFETY: the runtime check establishes NEON support and both
                // arrays contain exactly 32 values processed in 8-lane chunks.
                unsafe {
                    quantize_residual_pass_neon(
                        &mut residual,
                        &mut quants[pass],
                        inverse_scale,
                        scale,
                    );
                }
            } else {
                for index in 0..Q8_RESIDUAL_BLOCK_VALUES {
                    let quantized = (inverse_scale * residual[index])
                        .round()
                        .clamp(-127.0, 127.0) as i8;
                    quants[pass][index] = quantized;
                    residual[index] -= scale * f32::from(quantized);
                }
            }
        }
        Ok(Self { scales, quants })
    }
}

#[target_feature(enable = "neon")]
unsafe fn quantize_residual_pass_neon(
    residual: &mut [f32; Q8_RESIDUAL_BLOCK_VALUES],
    quants: &mut [i8; Q8_RESIDUAL_BLOCK_VALUES],
    inverse_scale: f32,
    scale: f32,
) {
    unsafe {
        let inverse = vdupq_n_f32(inverse_scale);
        let scale = vdupq_n_f32(scale);
        let minimum = vdupq_n_f32(-127.0);
        let maximum = vdupq_n_f32(127.0);
        for offset in (0..Q8_RESIDUAL_BLOCK_VALUES).step_by(8) {
            let values_0 = vld1q_f32(residual.as_ptr().add(offset));
            let values_1 = vld1q_f32(residual.as_ptr().add(offset + 4));
            let rounded_0 = vmaxq_f32(
                minimum,
                vminq_f32(maximum, vrndaq_f32(vmulq_f32(values_0, inverse))),
            );
            let rounded_1 = vmaxq_f32(
                minimum,
                vminq_f32(maximum, vrndaq_f32(vmulq_f32(values_1, inverse))),
            );
            let quantized_0 = vcvtq_s32_f32(rounded_0);
            let quantized_1 = vcvtq_s32_f32(rounded_1);
            let quantized_i16 = vcombine_s16(vqmovn_s32(quantized_0), vqmovn_s32(quantized_1));
            vst1_s8(quants.as_mut_ptr().add(offset), vqmovn_s16(quantized_i16));
            vst1q_f32(
                residual.as_mut_ptr().add(offset),
                vsubq_f32(values_0, vmulq_f32(vcvtq_f32_s32(quantized_0), scale)),
            );
            vst1q_f32(
                residual.as_mut_ptr().add(offset + 4),
                vsubq_f32(values_1, vmulq_f32(vcvtq_f32_s32(quantized_1), scale)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residual_reconstruction_is_close_to_f32_input() -> Result<(), InferenceError> {
        let values = (0..Q8_RESIDUAL_BLOCK_VALUES)
            .map(|index| ((index * 17 % 31) as f32 - 15.0) / 7.0)
            .collect::<Vec<_>>();
        let block = BlockQ8Residual::quantize_blocks(&values)?
            .into_iter()
            .next()
            .ok_or_else(|| InferenceError::new("missing residual Q8 block"))?;

        for (index, expected) in values.iter().enumerate() {
            let actual = (0..Q8_RESIDUAL_PASSES)
                .map(|pass| block.scales[pass] * f32::from(block.quants[pass][index]))
                .sum::<f32>();
            assert!((actual - expected).abs() < 0.001);
        }
        Ok(())
    }

    #[test]
    fn neon_quantization_matches_scalar_rounding() -> Result<(), InferenceError> {
        for seed in 0..64 {
            let values = (0..Q8_RESIDUAL_BLOCK_VALUES)
                .map(|index| {
                    let mixed = index * 1_103 + seed * 7_919;
                    ((mixed % 2_003) as f32 - 1_001.0) / ((seed % 7 + 1) as f32 * 19.0)
                })
                .collect::<Vec<_>>();
            let actual = BlockQ8Residual::quantize(&values)?;
            let expected = scalar_quantize(&values)?;
            assert_eq!(actual, expected, "seed={seed}");
        }
        Ok(())
    }

    fn scalar_quantize(values: &[f32]) -> Result<BlockQ8Residual, InferenceError> {
        let mut residual = [0.0; Q8_RESIDUAL_BLOCK_VALUES];
        residual.copy_from_slice(values);
        let mut scales = [0.0; Q8_RESIDUAL_PASSES];
        let mut quants = [[0; Q8_RESIDUAL_BLOCK_VALUES]; Q8_RESIDUAL_PASSES];
        for pass in 0..Q8_RESIDUAL_PASSES {
            let mut signed_max = 0.0f32;
            let mut absolute_max = 0.0f32;
            for value in residual {
                if value.abs() > absolute_max {
                    absolute_max = value.abs();
                    signed_max = value;
                }
            }
            if absolute_max == 0.0 {
                break;
            }
            let inverse_scale = -127.0 / signed_max;
            let scale = 1.0 / inverse_scale;
            scales[pass] = scale;
            for index in 0..Q8_RESIDUAL_BLOCK_VALUES {
                let quantized = (inverse_scale * residual[index])
                    .round()
                    .clamp(-127.0, 127.0) as i8;
                quants[pass][index] = quantized;
                residual[index] -= scale * f32::from(quantized);
            }
        }
        Ok(BlockQ8Residual { scales, quants })
    }
}
