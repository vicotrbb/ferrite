use ferrite_inference::scalar::{
    apply_rope, argmax, rms_norm, Matrix, ScalarLlamaConfig, ScalarLlamaLayerWeights,
    ScalarLlamaModel, ScalarLlamaWeights,
};
use ferrite_model::gguf::parse_gguf;
use std::error::Error;
use std::io;

const VALUE_STRING: u32 = 8;
const VALUE_ARRAY: u32 = 9;
const VALUE_UINT64: u32 = 10;
const GGML_TYPE_F32: u32 = 0;
const GGML_TYPE_F16: u32 = 1;
const GGML_TYPE_BF16: u32 = 30;

struct F32TensorFixture {
    name: &'static str,
    dimensions: Vec<u64>,
    values: Vec<f32>,
    offset: u64,
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    push_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn push_kv_string(bytes: &mut Vec<u8>, key: &str, value: &str) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_STRING);
    push_string(bytes, value);
}

fn push_kv_u64(bytes: &mut Vec<u8>, key: &str, value: u64) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_UINT64);
    push_u64(bytes, value);
}

fn push_kv_string_array(bytes: &mut Vec<u8>, key: &str, values: &[&str]) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_ARRAY);
    push_u32(bytes, VALUE_STRING);
    push_u64(bytes, values.len() as u64);
    for value in values {
        push_string(bytes, value);
    }
}

fn align_value(value: u64, alignment: u64) -> u64 {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
}

fn align_len(bytes: &mut Vec<u8>, alignment: usize) {
    let padding = (alignment - (bytes.len() % alignment)) % alignment;
    bytes.resize(bytes.len() + padding, 0);
}

fn push_tensor_info_with_type(bytes: &mut Vec<u8>, tensor: &F32TensorFixture, tensor_type: u32) {
    push_string(bytes, tensor.name);
    push_u32(bytes, tensor.dimensions.len() as u32);
    for dimension in &tensor.dimensions {
        push_u64(bytes, *dimension);
    }
    push_u32(bytes, tensor_type);
    push_u64(bytes, tensor.offset);
}

fn matrix_dims(cols: u64, rows: u64) -> Vec<u64> {
    vec![cols, rows]
}

fn scalar_llama_f32_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_F32)
}

fn scalar_llama_f16_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_F16)
}

fn scalar_llama_bf16_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_BF16)
}

fn scalar_llama_gguf_fixture(tensor_type: u32) -> Vec<u8> {
    let alignment = 64u64;
    let mut tensors = vec![
        F32TensorFixture {
            name: "token_embd.weight",
            dimensions: matrix_dims(2, 3),
            values: vec![1.0, 1.0, 0.0, 1.0, 2.0, -1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "output_norm.weight",
            dimensions: vec![2],
            values: vec![1.0, 1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "output.weight",
            dimensions: matrix_dims(2, 3),
            values: vec![0.1, 0.1, 0.2, 0.0, 1.0, 0.5],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_norm.weight",
            dimensions: vec![2],
            values: vec![1.0, 1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_q.weight",
            dimensions: matrix_dims(2, 2),
            values: vec![1.0, 0.0, 0.0, 1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_k.weight",
            dimensions: matrix_dims(2, 2),
            values: vec![1.0, 0.0, 0.0, 1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_v.weight",
            dimensions: matrix_dims(2, 2),
            values: vec![1.0, 0.0, 0.0, 1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_output.weight",
            dimensions: matrix_dims(2, 2),
            values: vec![1.0, 0.0, 0.0, 1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_norm.weight",
            dimensions: vec![2],
            values: vec![1.0, 1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_gate.weight",
            dimensions: matrix_dims(2, 2),
            values: vec![1.0, 0.0, 0.0, 1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_up.weight",
            dimensions: matrix_dims(2, 2),
            values: vec![1.0, 0.0, 0.0, 1.0],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_down.weight",
            dimensions: matrix_dims(2, 2),
            values: vec![1.0, 0.0, 0.0, 1.0],
            offset: 0,
        },
    ];

    let mut offset = 0u64;
    let bytes_per_value = if tensor_type == GGML_TYPE_F32 { 4 } else { 2 };
    for tensor in &mut tensors {
        tensor.offset = align_value(offset, alignment);
        offset = tensor.offset + tensor.values.len() as u64 * bytes_per_value;
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, tensors.len() as u64);
    push_u64(&mut bytes, 12);
    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u64(&mut bytes, "general.alignment", alignment);
    push_kv_u64(&mut bytes, "llama.context_length", 1);
    push_kv_u64(&mut bytes, "llama.embedding_length", 2);
    push_kv_u64(&mut bytes, "llama.block_count", 1);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 2);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 1);
    push_kv_u64(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "llama.attention.key_length", 2);
    push_kv_u64(&mut bytes, "llama.attention.value_length", 2);
    push_kv_u64(&mut bytes, "llama.rope.dimension_count", 2);
    push_kv_string_array(
        &mut bytes,
        "tokenizer.ggml.tokens",
        &["<unk>", "hello", "winner"],
    );

    for tensor in &tensors {
        push_tensor_info_with_type(&mut bytes, tensor, tensor_type);
    }
    align_len(&mut bytes, alignment as usize);

    let tensor_data_start = bytes.len();
    for tensor in &tensors {
        let target_len = tensor_data_start + tensor.offset as usize;
        if bytes.len() < target_len {
            bytes.resize(target_len, 0);
        }
        for value in &tensor.values {
            if tensor_type == GGML_TYPE_F16 {
                bytes.extend_from_slice(&f32_to_f16_bits(*value).to_le_bytes());
            } else if tensor_type == GGML_TYPE_BF16 {
                bytes.extend_from_slice(&f32_to_bf16_bits(*value).to_le_bytes());
            } else {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }

    bytes
}

fn f32_to_f16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = bits & 0x7fffff;

    if exponent <= 0 {
        return sign;
    }
    if exponent >= 0x1f {
        return sign | 0x7c00;
    }

    sign | ((exponent as u16) << 10) | ((mantissa >> 13) as u16)
}

fn f32_to_bf16_bits(value: f32) -> u16 {
    (value.to_bits() >> 16) as u16
}

fn assert_close(actual: f32, expected: f32) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= 0.0001,
        "expected {actual} to be within 0.0001 of {expected}; diff={diff}"
    );
}

#[test]
fn rms_norm_uses_scalar_root_mean_square_reference() -> Result<(), Box<dyn Error>> {
    let output = rms_norm(&[3.0, 4.0], &[1.0, 0.5], 0.0)?;
    let rms = 12.5_f32.sqrt();

    assert_close(output[0], 3.0 / rms);
    assert_close(output[1], 2.0 / rms);
    Ok(())
}

#[test]
fn matrix_vector_multiply_rejects_shape_mismatch() -> Result<(), Box<dyn Error>> {
    let matrix = Matrix::from_row_major(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0])?;

    let error = match matrix.mul_vec(&[1.0, 2.0]) {
        Ok(_) => return Err(io::Error::other("shape mismatch should fail").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("matrix columns 3 do not match vector length 2"));
    Ok(())
}

#[test]
fn single_token_llama_reference_path_produces_documented_argmax() -> Result<(), Box<dyn Error>> {
    let identity = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;
    let config = ScalarLlamaConfig {
        vocab_size: 3,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count: 2,
        rope_freq_base: 10_000.0,
        rms_norm_epsilon: 0.0,
    };
    let model = ScalarLlamaModel::new(
        config,
        ScalarLlamaWeights {
            token_embedding: Matrix::from_row_major(
                3,
                2,
                vec![
                    1.0, 1.0, // token 0
                    0.0, 1.0, // token 1
                    2.0, -1.0, // token 2
                ],
            )?,
            output_norm: vec![1.0, 1.0],
            output: Matrix::from_row_major(
                3,
                2,
                vec![
                    0.1, 0.1, // token 0 logit = 0.2 after final norm
                    0.2, 0.0, // token 1 logit = 0.2 after final norm
                    1.0, 0.5, // token 2 logit = 1.5 after final norm
                ],
            )?,
            layers: vec![ScalarLlamaLayerWeights {
                attn_norm: vec![1.0, 1.0],
                q_proj: identity.clone(),
                k_proj: identity.clone(),
                v_proj: identity.clone(),
                o_proj: identity.clone(),
                ffn_norm: vec![1.0, 1.0],
                ffn_gate: identity.clone(),
                ffn_up: identity.clone(),
                ffn_down: identity,
            }],
        },
    )?;

    let next = model.next_token(0)?;

    assert_eq!(next.token_id, 2);
    assert_close(next.logits[0], 0.2);
    assert_close(next.logits[1], 0.2);
    assert_close(next.logits[2], 1.5);
    assert_eq!(argmax(&next.logits)?, 2);
    Ok(())
}

#[test]
fn rope_rotates_even_odd_pairs_by_position_and_frequency() -> Result<(), Box<dyn Error>> {
    let rotated = apply_rope(&[1.0, 0.0, 0.0, 1.0], 1, 4, 1.0)?;

    assert_close(rotated[0], 1.0_f32.cos());
    assert_close(rotated[1], 1.0_f32.sin());
    assert_close(rotated[2], -1.0_f32.sin());
    assert_close(rotated[3], 1.0_f32.cos());
    Ok(())
}

#[test]
fn prompt_path_uses_causal_kv_attention_for_latest_token() -> Result<(), Box<dyn Error>> {
    let identity = Matrix::from_row_major(2, 2, vec![1.0, 0.0, 0.0, 1.0])?;
    let config = ScalarLlamaConfig {
        vocab_size: 4,
        hidden_size: 2,
        intermediate_size: 2,
        attention_head_count: 1,
        attention_head_count_kv: 1,
        head_dim: 2,
        rope_dimension_count: 0,
        rope_freq_base: 10_000.0,
        rms_norm_epsilon: 0.0,
    };
    let model = ScalarLlamaModel::new(
        config,
        ScalarLlamaWeights {
            token_embedding: Matrix::from_row_major(
                4,
                2,
                vec![
                    1.0, 0.0, // token 0
                    0.0, 1.0, // token 1
                    0.0, 0.0, // token 2
                    0.0, 0.0, // token 3
                ],
            )?,
            output_norm: vec![1.0, 1.0],
            output: Matrix::from_row_major(
                4,
                2,
                vec![
                    0.0, 0.0, // token 0
                    1.0, 0.0, // token 1 follows attention toward prior token
                    0.0, 1.0, // token 2 follows current token
                    -1.0, -1.0, // token 3
                ],
            )?,
            layers: vec![ScalarLlamaLayerWeights {
                attn_norm: vec![1.0, 1.0],
                q_proj: identity.clone(),
                k_proj: identity.clone(),
                v_proj: identity.clone(),
                o_proj: identity.clone(),
                ffn_norm: vec![1.0, 1.0],
                ffn_gate: Matrix::from_row_major(2, 2, vec![0.0; 4])?,
                ffn_up: identity.clone(),
                ffn_down: identity,
            }],
        },
    )?;

    let next = model.next_token_for_prompt(&[0, 1])?;

    assert_eq!(next.token_id, 2);
    assert!(next.logits[2] > next.logits[1]);
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_f32_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_f32_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_unquantized(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(next.token_id, 2);
    assert_close(next.logits[0], 0.2);
    assert_close(next.logits[1], 0.2);
    assert_close(next.logits[2], 1.5);
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_f16_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_f16_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_unquantized(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(next.token_id, 2);
    assert!((next.logits[2] - 1.5).abs() < 0.01);
    Ok(())
}

#[test]
fn loads_scalar_llama_reference_weights_from_bf16_gguf_fixture() -> Result<(), Box<dyn Error>> {
    let bytes = scalar_llama_bf16_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;

    let model = ScalarLlamaModel::from_gguf_unquantized(&gguf, &bytes)?;
    let next = model.next_token(0)?;

    assert_eq!(next.token_id, 2);
    assert!((next.logits[2] - 1.5).abs() < 0.01);
    Ok(())
}
