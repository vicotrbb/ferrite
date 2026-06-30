pub(crate) fn assert_close(actual: f32, expected: f32) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= 0.0001,
        "expected {actual} to be within 0.0001 of {expected}; diff={diff}"
    );
}
