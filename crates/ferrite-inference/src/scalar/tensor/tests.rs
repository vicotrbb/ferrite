use super::{
    InferenceError, bf16_values_from_le_bytes, f16_bits_to_f32, f16_values_from_le_bytes,
    f32_values_from_le_bytes, q4_k_values_from_le_bytes, q5_0_values_from_le_bytes,
    q6_k_values_from_le_bytes, q8_0_values_from_le_bytes,
};

type DecodeResult = Result<(), InferenceError>;

#[test]
fn dense_tensor_decoders_reject_non_finite_values() -> DecodeResult {
    let f32_cases = [
        f32::NAN.to_le_bytes(),
        f32::INFINITY.to_le_bytes(),
        f32::NEG_INFINITY.to_le_bytes(),
    ];
    for bytes in f32_cases {
        let error = match f32_values_from_le_bytes("dense", &bytes) {
            Ok(_) => {
                return Err(InferenceError::new(
                    "non-finite F32 tensor value should fail",
                ));
            }
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("tensor dense value 0 must be finite")
        );
    }

    for bits in [0x7e00u16, 0x7c00, 0xfc00] {
        let error = match f16_values_from_le_bytes("dense", &bits.to_le_bytes()) {
            Ok(_) => {
                return Err(InferenceError::new(
                    "non-finite F16 tensor value should fail",
                ));
            }
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("tensor dense value 0 must be finite")
        );
    }

    for bits in [0x7fc0u16, 0x7f80, 0xff80] {
        let error = match bf16_values_from_le_bytes("dense", &bits.to_le_bytes()) {
            Ok(_) => {
                return Err(InferenceError::new(
                    "non-finite BF16 tensor value should fail",
                ));
            }
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("tensor dense value 0 must be finite")
        );
    }

    Ok(())
}

#[test]
fn quantized_tensor_decoders_reject_non_finite_scale_values() -> DecodeResult {
    for scale_bits in [0x7e00u16, 0x7c00, 0xfc00] {
        let error =
            match q8_0_values_from_le_bytes("quantized", &q8_0_block_with_scale_bits(scale_bits)) {
                Ok(_) => {
                    return Err(InferenceError::new(
                        "non-finite Q8_0 tensor scale should fail",
                    ));
                }
                Err(error) => error,
            };
        assert!(
            error
                .to_string()
                .contains("tensor quantized Q8_0 scale values must be finite")
        );
    }

    for scale_bits in [0x7e00u16, 0x7c00, 0xfc00] {
        let error =
            match q5_0_values_from_le_bytes("quantized", &q5_0_block_with_scale_bits(scale_bits)) {
                Ok(_) => {
                    return Err(InferenceError::new(
                        "non-finite Q5_0 tensor scale should fail",
                    ));
                }
                Err(error) => error,
            };
        assert!(
            error
                .to_string()
                .contains("tensor quantized Q5_0 scale values must be finite")
        );
    }

    for scale_bits in [0x7e00u16, 0x7c00, 0xfc00] {
        let error = match q4_k_values_from_le_bytes(
            "quantized",
            &q4_k_block_with_scale_bits(scale_bits, 0),
        ) {
            Ok(_) => {
                return Err(InferenceError::new(
                    "non-finite Q4K tensor scale should fail",
                ));
            }
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("tensor quantized Q4K scale values must be finite")
        );
    }

    for scale_bits in [0x7e00u16, 0x7c00, 0xfc00] {
        let error =
            match q6_k_values_from_le_bytes("quantized", &q6_k_block_with_scale_bits(scale_bits)) {
                Ok(_) => {
                    return Err(InferenceError::new(
                        "non-finite Q6K tensor scale should fail",
                    ));
                }
                Err(error) => error,
            };
        assert!(
            error
                .to_string()
                .contains("tensor quantized Q6K scale values must be finite")
        );
    }

    Ok(())
}

#[test]
fn q5_0_decoder_reconstructs_signed_block_values() -> DecodeResult {
    let block = q5_0_reconstruction_block();

    let values = q5_0_values_from_le_bytes("q5", &block)?;

    let expected = (-16..16).map(|value| value as f32).collect::<Vec<_>>();
    assert_eq!(f16_bits_to_f32(0x3c00), 1.0);
    assert_eq!(values, expected);
    Ok(())
}

#[test]
fn q6_k_decoder_reconstructs_signed_block_values() -> DecodeResult {
    let mut block = vec![0u8; 128 + 64];
    block[32] = 0xff;
    block[128] = 0xe4;
    block.extend(vec![1u8; 16]);
    block.extend_from_slice(&0x3c00u16.to_le_bytes());

    let values = q6_k_values_from_le_bytes("q6", &block)?;

    assert_eq!(values[0], -32.0);
    assert_eq!(values[32], -1.0);
    assert_eq!(values[64], 0.0);
    assert_eq!(values[96], 31.0);
    Ok(())
}

fn q8_0_block_with_scale_bits(scale_bits: u16) -> Vec<u8> {
    let mut block = Vec::new();
    block.extend_from_slice(&scale_bits.to_le_bytes());
    block.extend([1u8; 32]);
    block
}

fn q5_0_block_with_scale_bits(scale_bits: u16) -> Vec<u8> {
    let mut block = Vec::new();
    block.extend_from_slice(&scale_bits.to_le_bytes());
    block.extend_from_slice(&0xffff_0000u32.to_le_bytes());
    block.extend([0u8; 16]);
    block
}

fn q5_0_reconstruction_block() -> Vec<u8> {
    let mut block = Vec::new();
    block.extend_from_slice(&0x3c00u16.to_le_bytes());
    block.extend_from_slice(&0xffff_0000u32.to_le_bytes());
    for index in 0..16u8 {
        block.push(index | (index << 4));
    }
    block
}

fn q4_k_block_with_scale_bits(scale_bits: u16, min_bits: u16) -> Vec<u8> {
    let mut block = Vec::new();
    block.extend_from_slice(&scale_bits.to_le_bytes());
    block.extend_from_slice(&min_bits.to_le_bytes());
    block.extend([1u8; 12]);
    block.extend([0u8; 128]);
    block
}

fn q6_k_block_with_scale_bits(scale_bits: u16) -> Vec<u8> {
    let mut block = vec![0u8; 128 + 64];
    block.extend([1u8; 16]);
    block.extend_from_slice(&scale_bits.to_le_bytes());
    block
}
