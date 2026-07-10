#![cfg_attr(
    target_arch = "aarch64",
    allow(
        unsafe_code,
        reason = "audited aarch64 SIMD intrinsics are isolated in this quantizer module"
    )
)]

use super::InferenceError;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::{
    vcombine_s16, vcvtq_s32_f32, vdupq_n_f32, vld1q_f32, vmaxq_f32, vminq_f32, vmulq_f32,
    vqmovn_s16, vqmovn_s32, vrndaq_f32, vst1_s8,
};

pub(in crate::scalar) const Q8_K_BLOCK_VALUES: usize = 256;
pub(in crate::scalar) const Q8_K_GROUP_SIZE: usize = 16;
pub(in crate::scalar) const Q8_K_GROUPS: usize = Q8_K_BLOCK_VALUES / Q8_K_GROUP_SIZE;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::scalar) struct BlockQ8K {
    pub(in crate::scalar) d: f32,
    pub(in crate::scalar) qs: [i8; Q8_K_BLOCK_VALUES],
    pub(in crate::scalar) bsums: [i16; Q8_K_GROUPS],
}

impl BlockQ8K {
    pub(in crate::scalar) fn quantize_blocks(values: &[f32]) -> Result<Vec<Self>, InferenceError> {
        if values.is_empty() {
            return Err(InferenceError::new(
                "Q8_K activation length must not be zero",
            ));
        }
        if !values.len().is_multiple_of(Q8_K_BLOCK_VALUES) {
            return Err(InferenceError::new(format!(
                "Q8_K activation length {} must be divisible by {Q8_K_BLOCK_VALUES}",
                values.len()
            )));
        }

        values
            .chunks_exact(Q8_K_BLOCK_VALUES)
            .map(Self::quantize)
            .collect()
    }

    pub(in crate::scalar) fn quantize(values: &[f32]) -> Result<Self, InferenceError> {
        if values.len() != Q8_K_BLOCK_VALUES {
            return Err(InferenceError::new(format!(
                "Q8_K activation length {} does not match {Q8_K_BLOCK_VALUES}",
                values.len()
            )));
        }

        let mut max = 0.0f32;
        let mut absolute_max = 0.0f32;
        for (index, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(InferenceError::new(format!(
                    "Q8_K activation value {index} is not finite"
                )));
            }
            let absolute = value.abs();
            if absolute > absolute_max {
                absolute_max = absolute;
                max = *value;
            }
        }

        if absolute_max == 0.0 {
            return Ok(Self {
                d: 0.0,
                qs: [0; Q8_K_BLOCK_VALUES],
                bsums: [0; Q8_K_GROUPS],
            });
        }

        let inverse_scale = -127.0 / max;
        let scale = 1.0 / inverse_scale;
        if !inverse_scale.is_finite() || !scale.is_finite() {
            return Err(InferenceError::new("Q8_K activation scale is not finite"));
        }

        let mut qs = [0i8; Q8_K_BLOCK_VALUES];
        #[cfg(target_arch = "aarch64")]
        if std::arch::is_aarch64_feature_detected!("neon") {
            // SAFETY: the runtime check establishes NEON support and the
            // input/output slices both contain exactly 256 values.
            unsafe { quantize_neon(values, inverse_scale, &mut qs) };
        } else {
            quantize_scalar(values, inverse_scale, &mut qs);
        }
        #[cfg(not(target_arch = "aarch64"))]
        quantize_scalar(values, inverse_scale, &mut qs);

        let mut bsums = [0i16; Q8_K_GROUPS];
        for (group_index, group) in qs.chunks_exact(Q8_K_GROUP_SIZE).enumerate() {
            bsums[group_index] = group.iter().map(|value| i16::from(*value)).sum();
        }

        Ok(Self {
            d: scale,
            qs,
            bsums,
        })
    }
}

fn quantize_scalar(values: &[f32], inverse_scale: f32, qs: &mut [i8; Q8_K_BLOCK_VALUES]) {
    for (index, value) in values.iter().enumerate() {
        let quantized = (inverse_scale * *value).round() as i32;
        qs[index] = quantized.clamp(-127, 127) as i8;
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn quantize_neon(values: &[f32], inverse_scale: f32, qs: &mut [i8; Q8_K_BLOCK_VALUES]) {
    // SAFETY: callers provide exactly 256 input and output values, and the
    // loop advances in eight-value chunks while NEON is enabled.
    unsafe {
        let inverse = vdupq_n_f32(inverse_scale);
        let minimum = vdupq_n_f32(-127.0);
        let maximum = vdupq_n_f32(127.0);
        for offset in (0..Q8_K_BLOCK_VALUES).step_by(8) {
            let values_0 = vld1q_f32(values.as_ptr().add(offset));
            let values_1 = vld1q_f32(values.as_ptr().add(offset + 4));
            let rounded_0 = vmaxq_f32(
                minimum,
                vminq_f32(maximum, vrndaq_f32(vmulq_f32(values_0, inverse))),
            );
            let rounded_1 = vmaxq_f32(
                minimum,
                vminq_f32(maximum, vrndaq_f32(vmulq_f32(values_1, inverse))),
            );
            let quantized_i16 = vcombine_s16(
                vqmovn_s32(vcvtq_s32_f32(rounded_0)),
                vqmovn_s32(vcvtq_s32_f32(rounded_1)),
            );
            vst1_s8(qs.as_mut_ptr().add(offset), vqmovn_s16(quantized_i16));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{quantize_scalar, BlockQ8K, Q8_K_BLOCK_VALUES, Q8_K_GROUPS, Q8_K_GROUP_SIZE};
    use crate::scalar::InferenceError;

    #[test]
    fn q8_k_quantizes_activation_block_with_group_sums() -> Result<(), InferenceError> {
        let values = patterned_values();

        let block = BlockQ8K::quantize(&values)?;

        assert_eq!(block.qs.len(), Q8_K_BLOCK_VALUES);
        assert_eq!(block.bsums.len(), Q8_K_GROUPS);
        assert!(block.d.is_finite());
        assert!(block.d != 0.0);
        assert!(block.qs.iter().all(|value| (-127..=127).contains(value)));
        for (group_index, group) in block.qs.chunks_exact(Q8_K_GROUP_SIZE).enumerate() {
            let expected = group.iter().map(|value| i16::from(*value)).sum::<i16>();
            assert_eq!(block.bsums[group_index], expected);
        }
        Ok(())
    }

    #[test]
    fn q8_k_neon_quantization_matches_scalar_rounding() -> Result<(), InferenceError> {
        for seed in 0..32 {
            let values = (0..Q8_K_BLOCK_VALUES)
                .map(|index| {
                    let mixed = index * 1_103 + seed * 7_919;
                    ((mixed % 2_003) as f32 - 1_001.0) / ((seed % 7 + 1) as f32 * 19.0)
                })
                .collect::<Vec<_>>();
            let actual = BlockQ8K::quantize(&values)?;
            let mut expected = [0; Q8_K_BLOCK_VALUES];
            quantize_scalar(&values, 1.0 / actual.d, &mut expected);
            assert_eq!(actual.qs, expected, "seed={seed}");
        }
        Ok(())
    }

    #[test]
    fn q8_k_quantization_matches_llama_signed_scale_for_positive_dominant_activation(
    ) -> Result<(), InferenceError> {
        let mut values = [0.0; Q8_K_BLOCK_VALUES];
        values[0] = 2.0;
        values[1] = -0.75;
        values[2] = 0.25;

        let block = BlockQ8K::quantize(&values)?;

        assert_eq!(block.d, -2.0 / 127.0);
        assert_eq!(block.qs[0], -127);
        assert_eq!(block.qs[1], 48);
        assert_eq!(block.qs[2], -16);
        assert_eq!(block.bsums[0], -95);
        assert!(block.bsums[1..].iter().all(|sum| *sum == 0));
        Ok(())
    }

    #[test]
    fn q8_k_quantization_matches_llama_signed_scale_for_negative_dominant_activation(
    ) -> Result<(), InferenceError> {
        let mut values = [0.0; Q8_K_BLOCK_VALUES];
        values[0] = -2.0;
        values[1] = 0.75;
        values[2] = -0.25;

        let block = BlockQ8K::quantize(&values)?;

        assert_eq!(block.d, 2.0 / 127.0);
        assert_eq!(block.qs[0], -127);
        assert_eq!(block.qs[1], 48);
        assert_eq!(block.qs[2], -16);
        assert_eq!(block.bsums[0], -95);
        assert!(block.bsums[1..].iter().all(|sum| *sum == 0));
        Ok(())
    }

    #[test]
    fn q8_k_quantization_matches_llama_zero_block_contract() -> Result<(), InferenceError> {
        let block = BlockQ8K::quantize(&[0.0; Q8_K_BLOCK_VALUES])?;

        assert_eq!(block.d, 0.0);
        assert!(block.qs.iter().all(|quantized| *quantized == 0));
        assert!(block.bsums.iter().all(|sum| *sum == 0));
        Ok(())
    }

    #[test]
    fn q8_k_rejects_wrong_activation_length() -> Result<(), InferenceError> {
        let err = match BlockQ8K::quantize(&[1.0, 2.0, 3.0]) {
            Ok(_) => return Err(InferenceError::new("wrong activation length must fail")),
            Err(err) => err,
        };

        assert_eq!(
            err.to_string(),
            "Q8_K activation length 3 does not match 256"
        );
        Ok(())
    }

    #[test]
    fn q8_k_rejects_non_finite_activation_values() -> Result<(), InferenceError> {
        let mut values = [0.0; Q8_K_BLOCK_VALUES];
        values[7] = f32::INFINITY;

        let err = match BlockQ8K::quantize(&values) {
            Ok(_) => return Err(InferenceError::new("non-finite activation must fail")),
            Err(err) => err,
        };

        assert_eq!(err.to_string(), "Q8_K activation value 7 is not finite");
        Ok(())
    }

    #[test]
    fn q8_k_rejects_non_finite_activation_scale() -> Result<(), InferenceError> {
        let mut values = [0.0; Q8_K_BLOCK_VALUES];
        values[0] = f32::MIN_POSITIVE;

        let err = match BlockQ8K::quantize(&values) {
            Ok(_) => return Err(InferenceError::new("non-finite activation scale must fail")),
            Err(err) => err,
        };

        assert_eq!(err.to_string(), "Q8_K activation scale is not finite");
        Ok(())
    }

    #[test]
    fn q8_k_quantizes_activation_blocks() -> Result<(), InferenceError> {
        let values = (0..Q8_K_BLOCK_VALUES * 2)
            .map(|index| index as f32 / 31.0 - 4.0)
            .collect::<Vec<_>>();

        let blocks = BlockQ8K::quantize_blocks(&values)?;

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0], BlockQ8K::quantize(&values[..Q8_K_BLOCK_VALUES])?);
        assert_eq!(blocks[1], BlockQ8K::quantize(&values[Q8_K_BLOCK_VALUES..])?);
        Ok(())
    }

    #[test]
    fn q8_k_rejects_empty_activation_block_collection() -> Result<(), InferenceError> {
        let err = match BlockQ8K::quantize_blocks(&[]) {
            Ok(_) => return Err(InferenceError::new("empty activation blocks must fail")),
            Err(err) => err,
        };

        assert_eq!(err.to_string(), "Q8_K activation length must not be zero");
        Ok(())
    }

    fn patterned_values() -> [f32; Q8_K_BLOCK_VALUES] {
        let mut values = [0.0; Q8_K_BLOCK_VALUES];
        for (index, value) in values.iter_mut().enumerate() {
            let centered = index as f32 - 127.5;
            *value = centered / 17.0;
        }
        values
    }
}
