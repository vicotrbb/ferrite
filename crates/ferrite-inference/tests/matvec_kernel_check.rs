use ferrite_inference::scalar::{Matrix, ScalarExecutionOptions};
use std::error::Error;
use std::io;

fn assert_close(actual: f32, expected: f32) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= 0.0001,
        "expected {actual} to be within 0.0001 of {expected}; diff={diff}"
    );
}

#[test]
fn f32_matvec_can_be_checked_against_scalar_reference() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_row_major(
        2,
        4,
        vec![
            1.0, -2.0, 3.0, 4.0, //
            -1.0, 0.5, 2.0, -3.0,
        ],
    )?;

    let output = matrix.mul_vec_checked_against_reference(&[0.5, 2.0, -1.0, 4.0], 0.001)?;

    assert_close(output[0], 9.5);
    assert_close(output[1], -13.5);
    Ok(())
}

#[test]
fn q8_matvec_check_uses_decoded_scalar_reference() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    bytes.extend(q8_row(1, 32));
    bytes.extend(q8_row(-1, 32));
    let matrix = Matrix::from_q8_0_row_major_bytes(2, 32, bytes)?;

    let vector: Vec<f32> = (1..=32).map(|value| value as f32).collect();
    let output = matrix.mul_vec_checked_against_reference(&vector, 0.001)?;

    assert_close(output[0], 528.0);
    assert_close(output[1], -528.0);
    Ok(())
}

#[test]
fn q8_matvec_rejects_non_finite_vector_values() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_q8_0_row_major_bytes(1, 32, q8_row(1, 32))?;

    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut vector = vec![1.0; 32];
        vector[0] = value;

        let error = match matrix.mul_vec(&vector) {
            Ok(_) => {
                return Err(io::Error::other("non-finite Q8_0 vector value should fail").into());
            }
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("matrix vector values must be finite")
        );
    }
    Ok(())
}

#[test]
fn q8_matrix_rejects_non_finite_scale_values() -> Result<(), Box<dyn Error>> {
    for scale_bits in [0x7c00, 0xfc00, 0x7e00] {
        let error = match Matrix::from_q8_0_row_major_bytes(
            1,
            32,
            q8_row_with_scale_bits(scale_bits, 1, 32),
        ) {
            Ok(_) => {
                return Err(io::Error::other("non-finite Q8_0 matrix scale should fail").into());
            }
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("Q8_0 matrix scale values must be finite")
        );
    }
    Ok(())
}

#[test]
fn q5_matvec_check_uses_decoded_scalar_reference() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    bytes.extend(q5_0_block_with_value(1));
    bytes.extend(q5_0_block_with_value(-2));
    let matrix = Matrix::from_q5_0_row_major_bytes(2, 32, bytes)?;

    let vector: Vec<f32> = (1..=32).map(|value| value as f32).collect();
    let output = matrix.mul_vec_checked_against_reference(&vector, 0.001)?;

    assert_close(output[0], 528.0);
    assert_close(output[1], -1056.0);
    Ok(())
}

#[test]
fn q5_matrix_rejects_non_finite_scale_values() -> Result<(), Box<dyn Error>> {
    for scale_bits in [0x7c00, 0xfc00, 0x7e00] {
        let error = match Matrix::from_q5_0_row_major_bytes(
            1,
            32,
            q5_0_block_with_scale_bits(scale_bits, 1),
        ) {
            Ok(_) => {
                return Err(io::Error::other("non-finite Q5_0 matrix scale should fail").into());
            }
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("Q5_0 matrix scale values must be finite")
        );
    }
    Ok(())
}

#[test]
fn q4_k_matvec_check_uses_decoded_scalar_reference() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_q4_k_row_major_bytes(1, 256, q4_k_block_with_value(1))?;

    let vector: Vec<f32> = (1..=256).map(|value| value as f32).collect();
    let output = matrix.mul_vec_checked_against_reference(&vector, 0.001)?;

    assert_close(output[0], 32896.0);
    Ok(())
}

#[test]
fn q4_k_matrix_rejects_non_finite_scale_values() -> Result<(), Box<dyn Error>> {
    for scale_bits in [0x7c00, 0xfc00, 0x7e00] {
        let error = match Matrix::from_q4_k_row_major_bytes(
            1,
            256,
            q4_k_block_with_scale_bits(scale_bits, 0, 1),
        ) {
            Ok(_) => {
                return Err(io::Error::other("non-finite Q4_K matrix scale should fail").into());
            }
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("Q4_K matrix scale values must be finite")
        );
    }
    Ok(())
}

#[test]
fn q4_k_matvec_accepts_q8_k_execution_options() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_q4_k_row_major_bytes(1, 256, q4_k_block_with_value(1))?;
    let vector: Vec<f32> = (1..=256).map(|value| value as f32).collect();

    let output = matrix.mul_vec_with_options(
        &vector,
        ScalarExecutionOptions::default().with_q8_k_activation_matvec(true),
    )?;

    assert_eq!(output.len(), 1);
    assert!(output[0].is_finite());
    Ok(())
}

#[test]
fn q6_k_matvec_check_uses_decoded_scalar_reference() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_q6_k_row_major_bytes(1, 256, q6_k_block_with_value(-32))?;

    let output = matrix.mul_vec_checked_against_reference(&[1.0; 256], 0.001)?;

    assert_close(output[0], -8192.0);
    Ok(())
}

#[test]
fn q6_k_matrix_rejects_non_finite_scale_values() -> Result<(), Box<dyn Error>> {
    for scale_bits in [0x7c00, 0xfc00, 0x7e00] {
        let error = match Matrix::from_q6_k_row_major_bytes(
            1,
            256,
            q6_k_block_with_scale_bits(scale_bits, -32),
        ) {
            Ok(_) => {
                return Err(io::Error::other("non-finite Q6_K matrix scale should fail").into());
            }
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("Q6_K matrix scale values must be finite")
        );
    }
    Ok(())
}

#[test]
fn q6_k_matvec_accepts_q8_k_execution_options() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_q6_k_row_major_bytes(1, 256, q6_k_block_with_value(-32))?;

    let output = matrix.mul_vec_with_options(
        &[1.0; 256],
        ScalarExecutionOptions::default().with_q8_k_activation_matvec(true),
    )?;

    assert_eq!(output.len(), 1);
    assert!(output[0].is_finite());
    Ok(())
}

#[test]
#[cfg(target_arch = "aarch64")]
fn q6_k_argmax_honors_q8_k_execution_options() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    bytes.extend(q6_k_block_with_unit_at(0));
    bytes.extend(q6_k_block_with_unit_at(1));
    let matrix = Matrix::from_q6_k_row_major_bytes(2, 256, bytes)?;
    let mut vector = vec![0.0; 256];
    vector[0] = 1.0;
    vector[1] = 1.003;

    assert_eq!(matrix.argmax_mul_vec(&vector)?, 1);
    let q8_k_argmax = matrix.argmax_mul_vec_with_options(
        &vector,
        ScalarExecutionOptions::default().with_q8_k_activation_matvec(true),
    )?;

    assert_eq!(q8_k_argmax, 0);
    Ok(())
}

#[test]
fn matvec_check_rejects_negative_relative_tolerance() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_row_major(1, 1, vec![1.0])?;

    let error = match matrix.mul_vec_checked_against_reference(&[1.0], -0.1) {
        Ok(_) => return Err(io::Error::other("negative tolerance should fail").into()),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("relative error tolerance -0.1 must be non-negative")
    );
    Ok(())
}

fn q8_row(value: i8, count: usize) -> Vec<u8> {
    q8_row_with_scale_bits(0x3c00, value, count)
}

fn q8_row_with_scale_bits(scale_bits: u16, value: i8, count: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&scale_bits.to_le_bytes());
    bytes.extend(std::iter::repeat_n(value as u8, count));
    bytes
}

fn q5_0_block_with_value(value: i32) -> Vec<u8> {
    q5_0_block_with_scale_bits(0x3c00, value)
}

fn q5_0_block_with_scale_bits(scale_bits: u16, value: i32) -> Vec<u8> {
    let quantized = (value + 16) as u8;
    let mut high_bits = 0u32;
    for index in 0..16 {
        if quantized & 0x10 != 0 {
            high_bits |= 1 << index;
            high_bits |= 1 << (index + 16);
        }
    }

    let mut block = Vec::new();
    block.extend_from_slice(&scale_bits.to_le_bytes());
    block.extend_from_slice(&high_bits.to_le_bytes());
    block.extend([(quantized & 0x0f) | ((quantized & 0x0f) << 4); 16]);
    block
}

fn q4_k_block_with_value(value: u8) -> Vec<u8> {
    q4_k_block_with_scale_bits(0x3c00, 0, value)
}

fn q4_k_block_with_scale_bits(scale_bits: u16, min_bits: u16, value: u8) -> Vec<u8> {
    let quantized = value & 0x0f;
    let mut block = Vec::new();
    block.extend_from_slice(&scale_bits.to_le_bytes());
    block.extend_from_slice(&min_bits.to_le_bytes());
    block.extend_from_slice(&[1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]);
    block.extend_from_slice(&[quantized | (quantized << 4); 128]);
    block
}

fn q6_k_block_with_value(value: i32) -> Vec<u8> {
    q6_k_block_with_scale_bits(0x3c00, value)
}

fn q6_k_block_with_scale_bits(scale_bits: u16, value: i32) -> Vec<u8> {
    assert_eq!(
        value, -32,
        "this compact Q6_K fixture helper only encodes -32"
    );

    let mut block = vec![0u8; 128 + 64];
    block.extend(vec![1u8; 16]);
    block.extend_from_slice(&scale_bits.to_le_bytes());
    block
}

#[cfg(target_arch = "aarch64")]
fn q6_k_block_with_unit_at(index: usize) -> Vec<u8> {
    let mut raw_values = [32u8; 256];
    raw_values[index] = 33;
    q6_k_block_from_raw_values(&raw_values)
}

#[cfg(target_arch = "aarch64")]
fn q6_k_block_from_raw_values(raw_values: &[u8; 256]) -> Vec<u8> {
    let mut low_bits = vec![0u8; 128];
    let mut high_bits = vec![0u8; 64];

    for half in 0..2 {
        let value_base = half * 128;
        let low_base = half * 64;
        let high_base = half * 32;
        for index in 0..32 {
            let q1 = raw_values[value_base + index];
            let q2 = raw_values[value_base + index + 32];
            let q3 = raw_values[value_base + index + 64];
            let q4 = raw_values[value_base + index + 96];

            low_bits[low_base + index] = (q1 & 0x0f) | ((q3 & 0x0f) << 4);
            low_bits[low_base + index + 32] = (q2 & 0x0f) | ((q4 & 0x0f) << 4);
            high_bits[high_base + index] = ((q1 >> 4) & 3)
                | (((q2 >> 4) & 3) << 2)
                | (((q3 >> 4) & 3) << 4)
                | (((q4 >> 4) & 3) << 6);
        }
    }

    let mut block = low_bits;
    block.extend(high_bits);
    block.extend(vec![1u8; 16]);
    block.extend_from_slice(&0x3c00u16.to_le_bytes());
    block
}
