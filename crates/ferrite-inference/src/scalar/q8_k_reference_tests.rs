use super::{
    InferenceError,
    float::f16_bits_to_f32,
    q4_k::{Q4_K_BLOCK_BYTES, Q4_K_BLOCK_VALUES},
    q4_k_q8_k::q4_k_q8_k_block_dot,
    q6_k::{Q6_K_BLOCK_BYTES, Q6_K_BLOCK_VALUES},
    q6_k_q8_k::q6_k_q8_k_block_dot,
    q8_k::BlockQ8K,
};

#[test]
fn q4_k_q8_k_matches_llama_integer_identity() -> Result<(), InferenceError> {
    let block = patterned_q4_k_block();
    let activation = BlockQ8K::quantize(&patterned_activation::<Q4_K_BLOCK_VALUES>())?;

    let actual = q4_k_q8_k_block_dot(&block, &activation)?;
    let expected = reference_q4_k_q8_k_dot(&block, &activation)?;

    assert_close(actual, expected);
    Ok(())
}

#[test]
fn q4_k_q8_k_identity_holds_for_signed_q8_k_scales() -> Result<(), InferenceError> {
    let block = patterned_q4_k_block();

    for activation_values in [
        positive_dominant_activation::<Q4_K_BLOCK_VALUES>(),
        negative_dominant_activation::<Q4_K_BLOCK_VALUES>(),
    ] {
        let activation = BlockQ8K::quantize(&activation_values)?;
        let actual = q4_k_q8_k_block_dot(&block, &activation)?;
        let expected = reference_q4_k_q8_k_dot(&block, &activation)?;

        assert_close(actual, expected);
    }

    Ok(())
}

#[test]
fn q6_k_q8_k_matches_llama_arm_split_identity() -> Result<(), InferenceError> {
    let block = patterned_q6_k_block();
    let activation = BlockQ8K::quantize(&patterned_activation::<Q6_K_BLOCK_VALUES>())?;

    let actual = q6_k_q8_k_block_dot(&block, &activation)?;
    let expected = reference_q6_k_q8_k_dot(&block, &activation)?;

    assert_close(actual, expected);
    Ok(())
}

#[test]
fn q6_k_q8_k_identity_holds_for_signed_q8_k_scales() -> Result<(), InferenceError> {
    let block = patterned_q6_k_block();

    for activation_values in [
        positive_dominant_activation::<Q6_K_BLOCK_VALUES>(),
        negative_dominant_activation::<Q6_K_BLOCK_VALUES>(),
    ] {
        let activation = BlockQ8K::quantize(&activation_values)?;
        let actual = q6_k_q8_k_block_dot(&block, &activation)?;
        let expected = reference_q6_k_q8_k_dot(&block, &activation)?;

        assert_close(actual, expected);
    }

    Ok(())
}

#[test]
fn q6_k_q8_k_identity_covers_every_q6_bitplane_lane() -> Result<(), InferenceError> {
    let block = raw_lane_sweep_q6_k_block();
    let activation = BlockQ8K::quantize(&patterned_activation::<Q6_K_BLOCK_VALUES>())?;

    let actual = q6_k_q8_k_block_dot(&block, &activation)?;
    let expected = reference_q6_k_q8_k_dot(&block, &activation)?;

    assert_close(actual, expected);
    Ok(())
}

fn reference_q4_k_q8_k_dot(block: &[u8], activation: &BlockQ8K) -> Result<f32, InferenceError> {
    if block.len() != Q4_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q4_K block byte length {} does not match {Q4_K_BLOCK_BYTES}",
            block.len()
        )));
    }

    let d = f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]]));
    let dmin = f16_bits_to_f32(u16::from_le_bytes([block[2], block[3]]));
    let scales = &block[4..16];
    let quants = &block[16..];
    let mut weighted_sum = 0i32;
    let mut min_sum = 0i32;

    for (chunk_index, quant_chunk) in quants.chunks_exact(32).enumerate() {
        let scale_low_index = chunk_index * 2;
        let scale_high_index = scale_low_index + 1;
        let activation_offset = chunk_index * 64;
        let (scale_low, min_low) = q4_k_scale_min(scale_low_index, scales);
        let (scale_high, min_high) = q4_k_scale_min(scale_high_index, scales);

        weighted_sum += i32::from(scale_low)
            * q4_dot_32(
                quant_chunk,
                &activation.qs[activation_offset..],
                Q4Nibble::Low,
            );
        weighted_sum += i32::from(scale_high)
            * q4_dot_32(
                quant_chunk,
                &activation.qs[activation_offset + 32..],
                Q4Nibble::High,
            );

        min_sum += i32::from(min_low)
            * (i32::from(activation.bsums[scale_low_index * 2])
                + i32::from(activation.bsums[scale_low_index * 2 + 1]));
        min_sum += i32::from(min_high)
            * (i32::from(activation.bsums[scale_high_index * 2])
                + i32::from(activation.bsums[scale_high_index * 2 + 1]));
    }

    Ok(activation.d * (d * weighted_sum as f32 - dmin * min_sum as f32))
}

fn reference_q6_k_q8_k_dot(block: &[u8], activation: &BlockQ8K) -> Result<f32, InferenceError> {
    if block.len() != Q6_K_BLOCK_BYTES {
        return Err(InferenceError::new(format!(
            "Q6_K block byte length {} does not match {Q6_K_BLOCK_BYTES}",
            block.len()
        )));
    }

    let low_bits = &block[0..128];
    let high_bits = &block[128..192];
    let scales = &block[192..208];
    let super_scale = f16_bits_to_f32(u16::from_le_bytes([block[208], block[209]]));
    let mut weighted_sum = 0i32;
    let mut correction_sum = 0i32;

    for (group, scale) in scales.iter().take(16).enumerate() {
        let scale = i32::from(*scale as i8);
        let value_base = group * 16;
        weighted_sum += scale * q6_raw_dot_16(low_bits, high_bits, group, activation);
        correction_sum += scale * i32::from(activation.bsums[value_base / 16]);
    }

    Ok(activation.d * super_scale * (weighted_sum - 32 * correction_sum) as f32)
}

#[derive(Clone, Copy)]
enum Q4Nibble {
    Low,
    High,
}

fn q4_dot_32(quant_chunk: &[u8], q8: &[i8], nibble: Q4Nibble) -> i32 {
    quant_chunk
        .iter()
        .zip(q8.iter().take(32))
        .map(|(quant, activation)| {
            let weight = match nibble {
                Q4Nibble::Low => quant & 0x0f,
                Q4Nibble::High => quant >> 4,
            };
            i32::from(weight) * i32::from(*activation)
        })
        .sum()
}

fn q4_k_scale_min(index: usize, scales: &[u8]) -> (u8, u8) {
    if index < 4 {
        (scales[index] & 63, scales[index + 4] & 63)
    } else {
        (
            (scales[index + 4] & 0x0f) | ((scales[index - 4] >> 6) << 4),
            (scales[index + 4] >> 4) | ((scales[index] >> 6) << 4),
        )
    }
}

fn q6_raw_dot_16(low_bits: &[u8], high_bits: &[u8], group: usize, activation: &BlockQ8K) -> i32 {
    let half = group / 8;
    let group_in_half = group % 8;
    let value_base = half * 128;
    let low_base = half * 64;
    let high_base = half * 32;
    let index_base = group_in_half % 2 * 16;
    let lane_group = match group_in_half / 2 {
        0 => Q6LaneGroup::Q1,
        1 => Q6LaneGroup::Q2,
        2 => Q6LaneGroup::Q3,
        _ => Q6LaneGroup::Q4,
    };
    let activation_base = value_base + index_base + lane_group.activation_offset();

    (0..16)
        .map(|lane| {
            let raw = q6_raw_lane(
                low_bits,
                high_bits,
                low_base,
                high_base,
                index_base + lane,
                lane_group,
            );
            i32::from(raw) * i32::from(activation.qs[activation_base + lane])
        })
        .sum()
}

#[derive(Clone, Copy)]
enum Q6LaneGroup {
    Q1,
    Q2,
    Q3,
    Q4,
}

impl Q6LaneGroup {
    fn activation_offset(self) -> usize {
        match self {
            Q6LaneGroup::Q1 => 0,
            Q6LaneGroup::Q2 => 32,
            Q6LaneGroup::Q3 => 64,
            Q6LaneGroup::Q4 => 96,
        }
    }
}

fn q6_raw_lane(
    low_bits: &[u8],
    high_bits: &[u8],
    low_base: usize,
    high_base: usize,
    offset: usize,
    lane_group: Q6LaneGroup,
) -> u8 {
    let high = high_bits[high_base + offset];
    match lane_group {
        Q6LaneGroup::Q1 => (low_bits[low_base + offset] & 0x0f) | ((high & 3) << 4),
        Q6LaneGroup::Q2 => (low_bits[low_base + offset + 32] & 0x0f) | (((high >> 2) & 3) << 4),
        Q6LaneGroup::Q3 => (low_bits[low_base + offset] >> 4) | (((high >> 4) & 3) << 4),
        Q6LaneGroup::Q4 => (low_bits[low_base + offset + 32] >> 4) | (((high >> 6) & 3) << 4),
    }
}

fn patterned_q4_k_block() -> Vec<u8> {
    let mut block = Vec::new();
    block.extend_from_slice(&0x3c00u16.to_le_bytes());
    block.extend_from_slice(&0x3800u16.to_le_bytes());
    block.extend(1..=12);
    for index in 0..128 {
        let low = index as u8 & 0x0f;
        let high = 15 - low;
        block.push(low | (high << 4));
    }
    block
}

fn patterned_q6_k_block() -> Vec<u8> {
    let mut block = Vec::new();
    block.extend((0..128).map(|index| (index * 37) as u8));
    block.extend((0..64).map(|index| (index * 19) as u8));
    block.extend(
        [-3i8, 2, -5, 4, -7, 6, -9, 8, 9, -8, 7, -6, 5, -4, 3, -2].map(|value| value as u8),
    );
    block.extend_from_slice(&0x3c00u16.to_le_bytes());
    block
}

fn raw_lane_sweep_q6_k_block() -> Vec<u8> {
    let mut block = vec![0u8; Q6_K_BLOCK_BYTES];
    let scales = [-8i8, 7, -6, 5, -4, 3, -2, 1, 2, -3, 4, -5, 6, -7, 8, -9];

    for group in 0..16 {
        for lane in 0..16 {
            let raw = ((group * 16 + lane) % 64) as u8;
            set_q6_raw_lane(&mut block, group, lane, raw);
        }
    }
    for (index, scale) in scales.iter().enumerate() {
        block[192 + index] = *scale as u8;
    }
    block[208..210].copy_from_slice(&0x3c00u16.to_le_bytes());

    block
}

fn set_q6_raw_lane(block: &mut [u8], group: usize, lane: usize, raw: u8) {
    debug_assert!(group < 16);
    debug_assert!(lane < 16);
    debug_assert!(raw < 64);

    let half = group / 8;
    let group_in_half = group % 8;
    let low_base = half * 64;
    let high_base = 128 + half * 32;
    let index_base = group_in_half % 2 * 16;
    let offset = index_base + lane;
    let low = raw & 0x0f;
    let high = raw >> 4;

    match group_in_half / 2 {
        0 => {
            block[low_base + offset] = (block[low_base + offset] & 0xf0) | low;
            block[high_base + offset] = (block[high_base + offset] & !0x03) | high;
        }
        1 => {
            block[low_base + offset + 32] = (block[low_base + offset + 32] & 0xf0) | low;
            block[high_base + offset] = (block[high_base + offset] & !0x0c) | (high << 2);
        }
        2 => {
            block[low_base + offset] = (block[low_base + offset] & 0x0f) | (low << 4);
            block[high_base + offset] = (block[high_base + offset] & !0x30) | (high << 4);
        }
        _ => {
            block[low_base + offset + 32] = (block[low_base + offset + 32] & 0x0f) | (low << 4);
            block[high_base + offset] = (block[high_base + offset] & !0xc0) | (high << 6);
        }
    }
}

fn patterned_activation<const N: usize>() -> [f32; N] {
    let mut values = [0.0; N];
    for (index, value) in values.iter_mut().enumerate() {
        let wave = (index % 31) as f32 - 15.0;
        *value = wave / 9.0;
    }
    values
}

fn positive_dominant_activation<const N: usize>() -> [f32; N] {
    let mut values = patterned_activation::<N>();
    values[0] = 3.25;
    values[1] = -1.5;
    values
}

fn negative_dominant_activation<const N: usize>() -> [f32; N] {
    let mut values = patterned_activation::<N>();
    values[0] = -3.25;
    values[1] = 1.5;
    values
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "actual={actual} expected={expected}"
    );
}
