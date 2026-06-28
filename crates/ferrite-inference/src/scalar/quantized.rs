#[cfg(test)]
pub(super) use super::q4_k::{accumulate_q4_k_block, q4_k_mul_vec};
pub(super) use super::q4_k::{decode_q4_k_values, q4_k_storage_bytes};
pub(super) use super::q5_0::{decode_q5_0_row, q5_0_mul_vec, q5_0_row_bytes, Q5_0_BLOCK_VALUES};
#[cfg(test)]
pub(super) use super::q6_k::{accumulate_q6_k_block, q6_k_mul_vec};
pub(super) use super::q6_k::{decode_q6_k_values, q6_k_storage_bytes};
pub(super) use super::q8_0::{decode_q8_0_row, q8_0_mul_vec, q8_0_row_bytes, Q8_0_BLOCK_VALUES};
#[cfg(test)]
use super::InferenceError;

#[cfg(test)]
mod tests {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    use super::super::q4_k::{q4_k_mul_vec_with_backend, Q4KMatVecBackend};
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    use super::super::q5_0::{q5_0_mul_vec_with_backend, Q5_0MatVecBackend};
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    use super::super::q6_k::{q6_k_mul_vec_with_backend, Q6KMatVecBackend};
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    use super::super::q8_0::{q8_0_mul_vec_with_backend, Q8_0MatVecBackend};
    use super::{
        accumulate_q4_k_block, accumulate_q6_k_block, decode_q6_k_values, q4_k_mul_vec,
        q5_0_mul_vec, q6_k_mul_vec, q8_0_mul_vec, InferenceError,
    };

    #[test]
    fn q4_k_mul_vec_accumulates_rows_without_full_row_decodes() -> Result<(), InferenceError> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
        block.extend_from_slice(&[0x11; 128]);

        let actual = q4_k_mul_vec(&block, 2, 128, &[1.0; 128])?;

        assert_eq!(actual, vec![128.0, 128.0]);
        Ok(())
    }

    #[test]
    fn q4_k_block_accumulation_updates_rows_without_decoded_matrix() -> Result<(), InferenceError> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
        block.extend_from_slice(&[0x11; 64]);
        block.extend_from_slice(&[0x22; 64]);

        let mut output = vec![0.0; 2];
        accumulate_q4_k_block(&block, 0, 2, 128, &[1.0; 128], &mut output)?;

        assert_eq!(output, vec![128.0, 256.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn q4_k_matvec_uses_neon_backend_on_aarch64() -> Result<(), InferenceError> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
        block.extend_from_slice(&[0x11; 128]);

        let output = q4_k_mul_vec_with_backend(&block, 1, 256, &[1.0; 256])?;

        assert_eq!(output.backend, Q4KMatVecBackend::Aarch64Neon);
        assert_eq!(output.values, vec![256.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn q4_k_matvec_uses_avx2_backend_on_x86_64() -> Result<(), InferenceError> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
        block.extend_from_slice(&[0x11; 128]);

        let output = q4_k_mul_vec_with_backend(&block, 1, 256, &[1.0; 256])?;

        assert_eq!(output.backend, Q4KMatVecBackend::X86_64Avx2);
        assert_eq!(output.values, vec![256.0]);
        Ok(())
    }

    #[test]
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    fn q4_k_simd_matvec_preserves_parallel_row_order() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q4_k_block_with_value(1));
        bytes.extend(q4_k_block_with_value(2));
        bytes.extend(q4_k_block_with_value(3));

        let output = q4_k_mul_vec_with_backend(&bytes, 3, 256, &[1.0; 256])?;

        #[cfg(target_arch = "aarch64")]
        assert_eq!(output.backend, Q4KMatVecBackend::Aarch64Neon);
        #[cfg(target_arch = "x86_64")]
        assert_eq!(output.backend, Q4KMatVecBackend::X86_64Avx2);
        assert_eq!(output.values, vec![256.0, 512.0, 768.0]);
        Ok(())
    }

    #[test]
    fn q6_k_decoder_reconstructs_signed_block_values() -> Result<(), InferenceError> {
        let mut block = vec![0u8; 128 + 64];
        block[32] = 0xff;
        block[128] = 0xe4;
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());

        let values = decode_q6_k_values(&block, 256)?;

        assert_eq!(values[0], -32.0);
        assert_eq!(values[32], -1.0);
        assert_eq!(values[64], 0.0);
        assert_eq!(values[96], 31.0);
        Ok(())
    }

    #[test]
    fn q6_k_mul_vec_accumulates_rows_without_full_row_decodes() -> Result<(), InferenceError> {
        let mut block = vec![0u8; 128 + 64];
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());

        let actual = q6_k_mul_vec(&block, 2, 128, &[1.0; 128])?;

        assert_eq!(actual, vec![-4096.0, -4096.0]);
        Ok(())
    }

    #[test]
    fn q6_k_block_accumulation_updates_rows_without_decoded_matrix() -> Result<(), InferenceError> {
        let mut block = vec![0u8; 128 + 64];
        block[32] = 0xff;
        block[128] = 0xe4;
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());

        let mut output = vec![0.0; 2];
        accumulate_q6_k_block(&block, 0, 2, 128, &[1.0; 128], &mut output)?;

        assert_eq!(output, vec![-3970.0, -4096.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn q6_k_matvec_uses_neon_backend_on_aarch64() -> Result<(), InferenceError> {
        let mut block = vec![0u8; 128 + 64];
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());

        let output = q6_k_mul_vec_with_backend(&block, 1, 256, &[1.0; 256])?;

        assert_eq!(output.backend, Q6KMatVecBackend::Aarch64Neon);
        assert_eq!(output.values, vec![-8192.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn q6_k_matvec_uses_avx2_backend_on_x86_64() -> Result<(), InferenceError> {
        let mut block = vec![0u8; 128 + 64];
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());

        let output = q6_k_mul_vec_with_backend(&block, 1, 256, &[1.0; 256])?;

        assert_eq!(output.backend, Q6KMatVecBackend::X86_64Avx2);
        assert_eq!(output.values, vec![-8192.0]);
        Ok(())
    }

    #[test]
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    fn q6_k_simd_matvec_preserves_parallel_row_order() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q6_k_block_with_first_group_pattern());
        bytes.extend(q6_k_block_with_minus_32());
        bytes.extend(q6_k_block_with_first_group_pattern());

        let output = q6_k_mul_vec_with_backend(&bytes, 3, 256, &[1.0; 256])?;

        #[cfg(target_arch = "aarch64")]
        assert_eq!(output.backend, Q6KMatVecBackend::Aarch64Neon);
        #[cfg(target_arch = "x86_64")]
        assert_eq!(output.backend, Q6KMatVecBackend::X86_64Avx2);
        assert_eq!(output.values, vec![-8066.0, -8192.0, -8066.0]);
        Ok(())
    }

    #[test]
    fn q8_0_mul_vec_accumulates_rows_without_row_decodes() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x3c00u16.to_le_bytes());
        bytes.extend([1u8; 32]);
        bytes.extend_from_slice(&0x3c00u16.to_le_bytes());
        bytes.extend([2u8; 32]);

        let actual = q8_0_mul_vec(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(actual, vec![32.0, 64.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn q8_0_matvec_uses_neon_backend_on_aarch64() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q8_0_block_with_value(1));
        bytes.extend(q8_0_block_with_value(-2));

        let output = q8_0_mul_vec_with_backend(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(output.backend, Q8_0MatVecBackend::Aarch64Neon);
        assert_eq!(output.values, vec![32.0, -64.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn q8_0_matvec_uses_avx2_backend_on_x86_64() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q8_0_block_with_value(1));
        bytes.extend(q8_0_block_with_value(-2));

        let output = q8_0_mul_vec_with_backend(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(output.backend, Q8_0MatVecBackend::X86_64Avx2);
        assert_eq!(output.values, vec![32.0, -64.0]);
        Ok(())
    }

    #[test]
    fn q5_0_mul_vec_accumulates_rows_without_row_decodes() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q5_0_block_with_value(1));
        bytes.extend(q5_0_block_with_value(2));

        let actual = q5_0_mul_vec(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(actual, vec![32.0, 64.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn q5_0_matvec_uses_neon_backend_on_aarch64() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q5_0_block_with_value(1));
        bytes.extend(q5_0_block_with_value(-2));

        let output = q5_0_mul_vec_with_backend(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(output.backend, Q5_0MatVecBackend::Aarch64Neon);
        assert_eq!(output.values, vec![32.0, -64.0]);
        Ok(())
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn q5_0_matvec_uses_avx2_backend_on_x86_64() -> Result<(), InferenceError> {
        let mut bytes = Vec::new();
        bytes.extend(q5_0_block_with_value(1));
        bytes.extend(q5_0_block_with_value(-2));

        let output = q5_0_mul_vec_with_backend(&bytes, 2, 32, &[1.0; 32])?;

        assert_eq!(output.backend, Q5_0MatVecBackend::X86_64Avx2);
        assert_eq!(output.values, vec![32.0, -64.0]);
        Ok(())
    }

    fn q5_0_block_with_value(value: i32) -> Vec<u8> {
        let quantized = (value + 16) as u8;
        let mut high_bits = 0u32;
        for index in 0..16 {
            if quantized & 0x10 != 0 {
                high_bits |= 1 << index;
                high_bits |= 1 << (index + 16);
            }
        }
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&high_bits.to_le_bytes());
        block.extend([(quantized & 0x0f) | ((quantized & 0x0f) << 4); 16]);
        block
    }

    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    fn q4_k_block_with_value(value: u8) -> Vec<u8> {
        let quantized = value & 0x0f;
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
        block.extend_from_slice(&[quantized | (quantized << 4); 128]);
        block
    }

    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    fn q6_k_block_with_minus_32() -> Vec<u8> {
        let mut block = vec![0u8; 128 + 64];
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block
    }

    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    fn q6_k_block_with_first_group_pattern() -> Vec<u8> {
        let mut block = vec![0u8; 128 + 64];
        block[32] = 0xff;
        block[128] = 0xe4;
        block.extend(vec![1u8; 16]);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block
    }

    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    fn q8_0_block_with_value(value: i8) -> Vec<u8> {
        let mut block = Vec::new();
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend([value as u8; 32]);
        block
    }
}
