use super::InferenceError;

pub(super) const Q8_K_BLOCK_VALUES: usize = 256;
pub(super) const Q8_K_GROUP_SIZE: usize = 16;
pub(super) const Q8_K_GROUPS: usize = Q8_K_BLOCK_VALUES / Q8_K_GROUP_SIZE;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BlockQ8K {
    pub(super) d: f32,
    pub(super) qs: [i8; Q8_K_BLOCK_VALUES],
    pub(super) bsums: [i16; Q8_K_GROUPS],
}

impl BlockQ8K {
    pub(super) fn quantize(values: &[f32]) -> Result<Self, InferenceError> {
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
        let mut qs = [0i8; Q8_K_BLOCK_VALUES];
        for (index, value) in values.iter().enumerate() {
            let quantized = (inverse_scale * *value).round() as i32;
            qs[index] = quantized.clamp(-127, 127) as i8;
        }

        let mut bsums = [0i16; Q8_K_GROUPS];
        for (group_index, group) in qs.chunks_exact(Q8_K_GROUP_SIZE).enumerate() {
            bsums[group_index] = group.iter().map(|value| i16::from(*value)).sum();
        }

        Ok(Self {
            d: 1.0 / inverse_scale,
            qs,
            bsums,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockQ8K, Q8_K_BLOCK_VALUES, Q8_K_GROUPS, Q8_K_GROUP_SIZE};
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

    fn patterned_values() -> [f32; Q8_K_BLOCK_VALUES] {
        let mut values = [0.0; Q8_K_BLOCK_VALUES];
        for (index, value) in values.iter_mut().enumerate() {
            let centered = index as f32 - 127.5;
            *value = centered / 17.0;
        }
        values
    }
}
