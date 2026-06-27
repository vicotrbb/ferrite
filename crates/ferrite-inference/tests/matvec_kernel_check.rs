use ferrite_inference::scalar::Matrix;
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
fn matvec_check_rejects_negative_relative_tolerance() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_row_major(1, 1, vec![1.0])?;

    let error = match matrix.mul_vec_checked_against_reference(&[1.0], -0.1) {
        Ok(_) => return Err(io::Error::other("negative tolerance should fail").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("relative error tolerance -0.1 must be non-negative"));
    Ok(())
}

fn q8_row(value: i8, count: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x3c00u16.to_le_bytes());
    bytes.extend(std::iter::repeat_n(value as u8, count));
    bytes
}
