#![allow(
    unsafe_code,
    reason = "audited aarch64 SIMD intrinsics are isolated in this kernel module"
)]

use super::{
    neon_util::{native_f16_bits_to_f32, widen_s8_lanes},
    q5_0::{Q5_0MatVecBackend, Q5_0MatVecOutput, Q5_0_BLOCK_BYTES, Q5_0_BLOCK_VALUES},
};
use rayon::prelude::*;
use std::arch::aarch64::{
    uint8x16_t, vaddq_s8, vaddvq_f32, vandq_u8, vcombine_u8, vdupq_n_f32, vdupq_n_u8, vfmaq_f32,
    vld1_u8, vld1q_f32, vld1q_u8, vreinterpretq_s8_u8, vshrq_n_u8,
};

const ROW_PARALLEL_MIN_ROWS: usize = 512;
/// Minimum rows per rayon task: keeps per-task work in the tens of
/// microseconds so fork-join overhead and straggler tails stay small
/// relative to the streamed weight bytes.
const ROW_PARALLEL_MIN_ROWS_PER_TASK: usize = 128;
/// Batched rows perform several dot products after each weight decode, so a
/// smaller chunk still amortizes scheduling while exposing enough tasks for
/// 896-row attention projections to occupy a 10-worker pool.
const BATCH_ROWS_PER_TASK: usize = 64;

pub(super) fn neon_q5_0_mul_vec(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> Q5_0MatVecOutput {
    let row_bytes = (cols / Q5_0_BLOCK_VALUES) * Q5_0_BLOCK_BYTES;
    if uses_row_parallel(rows, cols) {
        return neon_q5_0_mul_vec_row_parallel(bytes, rows, row_bytes, vector);
    }

    let mut values = vec![0.0; rows];
    for (row, row_chunk) in bytes.chunks_exact(row_bytes).enumerate() {
        values[row] = neon_q5_0_row_dot(row_chunk, vector);
    }

    Q5_0MatVecOutput {
        values,
        backend: Q5_0MatVecBackend::Aarch64Neon,
    }
}

pub(super) fn neon_q5_0_mul_vec_pair(
    left_bytes: &[u8],
    right_bytes: &[u8],
    rows: usize,
    cols: usize,
    vector: &[f32],
) -> (Vec<f32>, Vec<f32>) {
    let row_bytes = (cols / Q5_0_BLOCK_VALUES) * Q5_0_BLOCK_BYTES;
    let mut left_values = vec![0.0; rows];
    let mut right_values = vec![0.0; rows];

    if uses_row_parallel(rows, cols) {
        left_values
            .par_iter_mut()
            .zip(right_values.par_iter_mut())
            .zip(left_bytes.par_chunks_exact(row_bytes))
            .zip(right_bytes.par_chunks_exact(row_bytes))
            .with_min_len(ROW_PARALLEL_MIN_ROWS_PER_TASK)
            .for_each(|(((left_out, right_out), left_row), right_row)| {
                (*left_out, *right_out) = neon_q5_0_row_dot_pair(left_row, right_row, vector);
            });
    } else {
        for (((left_out, right_out), left_row), right_row) in left_values
            .iter_mut()
            .zip(&mut right_values)
            .zip(left_bytes.chunks_exact(row_bytes))
            .zip(right_bytes.chunks_exact(row_bytes))
        {
            (*left_out, *right_out) = neon_q5_0_row_dot_pair(left_row, right_row, vector);
        }
    }

    (left_values, right_values)
}

fn neon_q5_0_mul_vec_row_parallel(
    bytes: &[u8],
    rows: usize,
    row_bytes: usize,
    vector: &[f32],
) -> Q5_0MatVecOutput {
    let values = bytes
        .par_chunks_exact(row_bytes)
        .with_min_len(ROW_PARALLEL_MIN_ROWS_PER_TASK)
        .map(|row_chunk| neon_q5_0_row_dot(row_chunk, vector))
        .collect::<Vec<_>>();
    debug_assert_eq!(values.len(), rows);

    Q5_0MatVecOutput {
        values,
        backend: Q5_0MatVecBackend::Aarch64NeonRowParallel,
    }
}

/// Batched matvec: streams each weight row once and dots it against every
/// activation vector, so DRAM traffic and weight decode are amortized over
/// the batch. Per-stream block/FMA order matches `neon_q5_0_mul_vec`
/// exactly, so each stream's output is bit-identical to a single-vector
/// call. Callers validate shapes and cap the batch at
/// [`super::q5_0::Q5_0_MAX_BATCH`].
pub(super) fn neon_q5_0_mul_vec_batch(
    bytes: &[u8],
    rows: usize,
    cols: usize,
    vectors: &[&[f32]],
) -> Vec<Vec<f32>> {
    let batch = vectors.len();
    let row_bytes = (cols / Q5_0_BLOCK_VALUES) * Q5_0_BLOCK_BYTES;
    let mut flat = vec![0.0f32; rows * batch];
    bytes
        .par_chunks_exact(row_bytes)
        .zip(flat.par_chunks_exact_mut(batch))
        .with_min_len(BATCH_ROWS_PER_TASK)
        .for_each(|(row_chunk, row_out)| neon_q5_0_row_dot_batch(row_chunk, vectors, row_out));

    (0..batch)
        .map(|stream| (0..rows).map(|row| flat[row * batch + stream]).collect())
        .collect()
}

fn neon_q5_0_row_dot_batch(row_chunk: &[u8], vectors: &[&[f32]], row_out: &mut [f32]) {
    for (block_index, block) in row_chunk.chunks_exact(Q5_0_BLOCK_BYTES).enumerate() {
        let col_base = block_index * Q5_0_BLOCK_VALUES;
        // SAFETY: the dispatch path checks NEON support, `block` has exactly
        // one Q5_0 block, and the caller validated every vector to `cols`
        // (a multiple of 32), so the per-block slices are in bounds.
        unsafe {
            neon_q5_0_block_dot_batch(block, vectors, col_base, row_out);
        }
    }
}

/// Decodes one Q5_0 block once and dots it against every stream's
/// activation window. Each stream's FMA sequence and per-block
/// `horizontal-add × scale` shape match `neon_q5_0_block_dot` exactly, so
/// per-stream sums stay bit-identical to the single-vector kernel.
#[target_feature(enable = "neon")]
unsafe fn neon_q5_0_block_dot_batch(
    block: &[u8],
    vectors: &[&[f32]],
    col_base: usize,
    row_out: &mut [f32],
) {
    // SAFETY: NEON is enabled on this function and callers provide a complete
    // Q5_0 block whose first two bytes hold the half-precision scale.
    let scale = unsafe { native_f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]])) };
    let quants = &block[6..];

    // SAFETY: same bounds argument as `neon_q5_0_block_dot`; every load is
    // within the 16 quant bytes or a validated 32-element vector window.
    unsafe {
        let quant_bytes = vld1q_u8(quants.as_ptr());
        let low_nibbles = vandq_u8(quant_bytes, vdupq_n_u8(0x0f));
        let high_nibbles = vshrq_n_u8(quant_bytes, 4);
        let (low_offsets, high_offsets) = q5_signed_offsets(block);
        let low_quads = widen_s8_lanes(vaddq_s8(
            vreinterpretq_s8_u8(low_nibbles),
            vreinterpretq_s8_u8(low_offsets),
        ));
        let high_quads = widen_s8_lanes(vaddq_s8(
            vreinterpretq_s8_u8(high_nibbles),
            vreinterpretq_s8_u8(high_offsets),
        ));

        let mut stream = 0usize;
        while stream + 4 <= vectors.len() {
            let windows = [
                vectors[stream].as_ptr().add(col_base),
                vectors[stream + 1].as_ptr().add(col_base),
                vectors[stream + 2].as_ptr().add(col_base),
                vectors[stream + 3].as_ptr().add(col_base),
            ];
            let mut lanes_0 = vdupq_n_f32(0.0);
            let mut lanes_1 = vdupq_n_f32(0.0);
            let mut lanes_2 = vdupq_n_f32(0.0);
            let mut lanes_3 = vdupq_n_f32(0.0);
            for step in 0..4 {
                let low_offset = step * 4;
                lanes_0 = vfmaq_f32(
                    lanes_0,
                    low_quads[step],
                    vld1q_f32(windows[0].add(low_offset)),
                );
                lanes_1 = vfmaq_f32(
                    lanes_1,
                    low_quads[step],
                    vld1q_f32(windows[1].add(low_offset)),
                );
                lanes_2 = vfmaq_f32(
                    lanes_2,
                    low_quads[step],
                    vld1q_f32(windows[2].add(low_offset)),
                );
                lanes_3 = vfmaq_f32(
                    lanes_3,
                    low_quads[step],
                    vld1q_f32(windows[3].add(low_offset)),
                );

                let high_offset = low_offset + 16;
                lanes_0 = vfmaq_f32(
                    lanes_0,
                    high_quads[step],
                    vld1q_f32(windows[0].add(high_offset)),
                );
                lanes_1 = vfmaq_f32(
                    lanes_1,
                    high_quads[step],
                    vld1q_f32(windows[1].add(high_offset)),
                );
                lanes_2 = vfmaq_f32(
                    lanes_2,
                    high_quads[step],
                    vld1q_f32(windows[2].add(high_offset)),
                );
                lanes_3 = vfmaq_f32(
                    lanes_3,
                    high_quads[step],
                    vld1q_f32(windows[3].add(high_offset)),
                );
            }
            row_out[stream] += vaddvq_f32(lanes_0) * scale;
            row_out[stream + 1] += vaddvq_f32(lanes_1) * scale;
            row_out[stream + 2] += vaddvq_f32(lanes_2) * scale;
            row_out[stream + 3] += vaddvq_f32(lanes_3) * scale;
            stream += 4;
        }

        for (out, vector) in row_out[stream..].iter_mut().zip(&vectors[stream..]) {
            let window = vector.as_ptr().add(col_base);
            let mut lanes = vdupq_n_f32(0.0);
            for step in 0..4 {
                lanes = vfmaq_f32(lanes, low_quads[step], vld1q_f32(window.add(step * 4)));
                lanes = vfmaq_f32(
                    lanes,
                    high_quads[step],
                    vld1q_f32(window.add(step * 4 + 16)),
                );
            }
            *out += vaddvq_f32(lanes) * scale;
        }
    }
}

fn neon_q5_0_row_dot(row_chunk: &[u8], vector: &[f32]) -> f32 {
    let mut sum = 0.0;
    for (block_index, block) in row_chunk.chunks_exact(Q5_0_BLOCK_BYTES).enumerate() {
        let col_base = block_index * Q5_0_BLOCK_VALUES;
        // SAFETY: the dispatch path checks NEON support, `block` has exactly
        // one Q5_0 block, and the matrix constructor validates that columns
        // are a multiple of 32 so the per-block vector slice is in bounds.
        sum +=
            unsafe { neon_q5_0_block_dot(block, &vector[col_base..col_base + Q5_0_BLOCK_VALUES]) };
    }
    sum
}

fn neon_q5_0_row_dot_pair(left_row: &[u8], right_row: &[u8], vector: &[f32]) -> (f32, f32) {
    let mut left_sum = 0.0;
    let mut right_sum = 0.0;
    for (block_index, (left_block, right_block)) in left_row
        .chunks_exact(Q5_0_BLOCK_BYTES)
        .zip(right_row.chunks_exact(Q5_0_BLOCK_BYTES))
        .enumerate()
    {
        let col_base = block_index * Q5_0_BLOCK_VALUES;
        let activation = &vector[col_base..col_base + Q5_0_BLOCK_VALUES];
        // SAFETY: both blocks and the activation window have the validated
        // Q5_0 sizes. Each matrix keeps its original block accumulation order.
        unsafe {
            left_sum += neon_q5_0_block_dot(left_block, activation);
            right_sum += neon_q5_0_block_dot(right_block, activation);
        }
    }
    (left_sum, right_sum)
}

fn uses_row_parallel(rows: usize, _cols: usize) -> bool {
    rows >= ROW_PARALLEL_MIN_ROWS
}

const Q5_SIGN_OFFSETS: [[u8; 8]; 256] = build_q5_sign_offsets();

const fn build_q5_sign_offsets() -> [[u8; 8]; 256] {
    let mut table = [[0xf0u8; 8]; 256];
    let mut byte = 0usize;
    while byte < table.len() {
        let mut bit = 0usize;
        while bit < 8 {
            if byte & (1 << bit) != 0 {
                table[byte][bit] = 0;
            }
            bit += 1;
        }
        byte += 1;
    }
    table
}

/// Expands the Q5 high-bit word into signed offsets for the two nibble
/// vectors. A set bit contributes zero; a clear bit contributes -16. The
/// 2 KiB table remains L1-resident and lets one signed add finish each half.
#[inline(always)]
pub(super) unsafe fn q5_signed_offsets(block: &[u8]) -> (uint8x16_t, uint8x16_t) {
    // SAFETY: callers provide a complete Q5_0 block, so bytes 2 through 5
    // select four valid 8-lane lookup rows from the fixed table.
    unsafe {
        let low = vcombine_u8(
            vld1_u8(Q5_SIGN_OFFSETS[block[2] as usize].as_ptr()),
            vld1_u8(Q5_SIGN_OFFSETS[block[3] as usize].as_ptr()),
        );
        let high = vcombine_u8(
            vld1_u8(Q5_SIGN_OFFSETS[block[4] as usize].as_ptr()),
            vld1_u8(Q5_SIGN_OFFSETS[block[5] as usize].as_ptr()),
        );
        (low, high)
    }
}

#[target_feature(enable = "neon")]
pub(super) unsafe fn neon_q5_0_block_dot(block: &[u8], vector: &[f32]) -> f32 {
    // SAFETY: NEON is enabled and callers provide a complete Q5_0 block.
    let scale = unsafe { native_f16_bits_to_f32(u16::from_le_bytes([block[0], block[1]])) };
    let quants = &block[6..];

    // SAFETY: callers pass a length-checked 22-byte Q5_0 block (16 quant
    // bytes after the scale and high-bit words) and a 32-element vector
    // slice, so the 16-byte quant load, the mask-table load, and the eight
    // 4-lane vector loads below are all in bounds.
    unsafe {
        let quant_bytes = vld1q_u8(quants.as_ptr());

        // Values 0..16 come from the low nibbles plus high bits 0..16;
        // values 16..32 come from the high nibbles plus high bits 16..32.
        let low_nibbles = vandq_u8(quant_bytes, vdupq_n_u8(0x0f));
        let high_nibbles = vshrq_n_u8(quant_bytes, 4);

        let (low_offsets, high_offsets) = q5_signed_offsets(block);
        let low_signed = vaddq_s8(
            vreinterpretq_s8_u8(low_nibbles),
            vreinterpretq_s8_u8(low_offsets),
        );
        let high_signed = vaddq_s8(
            vreinterpretq_s8_u8(high_nibbles),
            vreinterpretq_s8_u8(high_offsets),
        );

        // Keep the exact FMA accumulation order of the previous kernel
        // (low group then high group per 4-lane step) so the result stays
        // bit-identical: integer-to-f32 conversion of values in -16..16 is
        // exact, so only the accumulation order could change the sum.
        let low_quads = widen_s8_lanes(low_signed);
        let high_quads = widen_s8_lanes(high_signed);
        let mut lanes = vdupq_n_f32(0.0);
        for step in 0..4 {
            let low_vector = vld1q_f32(vector.as_ptr().add(step * 4));
            lanes = vfmaq_f32(lanes, low_quads[step], low_vector);

            let high_vector = vld1q_f32(vector.as_ptr().add(step * 4 + 16));
            lanes = vfmaq_f32(lanes, high_quads[step], high_vector);
        }

        vaddvq_f32(lanes) * scale
    }
}

#[cfg(test)]
mod tests {
    use super::{neon_q5_0_block_dot, neon_q5_0_block_dot_batch};
    use crate::scalar::{
        q5_0::{decode_q5_0_row, Q5_0_BLOCK_BYTES, Q5_0_BLOCK_VALUES},
        InferenceError,
    };

    #[test]
    fn neon_q5_0_block_dot_matches_decoded_values() -> Result<(), InferenceError> {
        let block = patterned_q5_0_block();
        let vector = (0..Q5_0_BLOCK_VALUES)
            .map(|index| (index % 9) as f32 - 4.0)
            .collect::<Vec<_>>();

        // SAFETY: the test provides a complete Q5_0 block and matching
        // 32-value activation on an aarch64 target with baseline NEON.
        let actual = unsafe { neon_q5_0_block_dot(&block, &vector) };
        let expected = decode_q5_0_row(&block, Q5_0_BLOCK_VALUES)?
            .iter()
            .zip(&vector)
            .map(|(left, right)| left * right)
            .sum::<f32>();

        assert!(
            (actual - expected).abs() < 0.001,
            "actual={actual} expected={expected}"
        );
        Ok(())
    }

    #[test]
    fn batched_block_dot_is_bit_identical_per_stream() -> Result<(), InferenceError> {
        let block = patterned_q5_0_block();
        let vectors = (0..8)
            .map(|stream| {
                (0..Q5_0_BLOCK_VALUES)
                    .map(|index| ((index + stream * 3) % 17) as f32 - 8.0)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let references = vectors.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let mut actual = vec![0.0; references.len()];

        // SAFETY: every test vector has one complete Q5_0 activation window,
        // and the output has one slot per input stream.
        unsafe { neon_q5_0_block_dot_batch(&block, &references, 0, &mut actual) };
        for (index, vector) in references.iter().enumerate() {
            // SAFETY: each reference is a complete 32-value activation.
            let expected = unsafe { neon_q5_0_block_dot(&block, vector) };
            assert_eq!(actual[index].to_bits(), expected.to_bits());
        }
        Ok(())
    }

    #[test]
    fn neon_q5_0_block_dot_matches_decoded_values_across_high_bit_patterns(
    ) -> Result<(), InferenceError> {
        let vector = (0..Q5_0_BLOCK_VALUES)
            .map(|index| (index % 7) as f32 - 3.0)
            .collect::<Vec<_>>();

        for high_bits in [
            0u32,
            u32::MAX,
            0xa5c3_3c5a,
            0x0f0f_f0f0,
            0x8000_0001,
            0x0001_8000,
        ] {
            let mut block = patterned_q5_0_block();
            block[2..6].copy_from_slice(&high_bits.to_le_bytes());

            // SAFETY: each modified input remains a complete Q5_0 block and
            // the activation retains its validated 32-value length.
            let actual = unsafe { neon_q5_0_block_dot(&block, &vector) };
            let expected = decode_q5_0_row(&block, Q5_0_BLOCK_VALUES)?
                .iter()
                .zip(&vector)
                .map(|(left, right)| left * right)
                .sum::<f32>();

            assert!(
                (actual - expected).abs() < 0.001,
                "high_bits={high_bits:#010x} actual={actual} expected={expected}"
            );
        }
        Ok(())
    }

    fn patterned_q5_0_block() -> Vec<u8> {
        let mut block = Vec::with_capacity(Q5_0_BLOCK_BYTES);
        block.extend_from_slice(&0x3c00u16.to_le_bytes());
        block.extend_from_slice(&0xa5c33c5au32.to_le_bytes());
        for index in 0..16 {
            let low = (index * 3) as u8 & 0x0f;
            let high = (15 - index) as u8 & 0x0f;
            block.push(low | (high << 4));
        }
        block
    }
}
