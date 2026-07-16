use crate::gguf_writer::{
    F32TensorFixture, GGML_TYPE_BF16, GGML_TYPE_F16, GGML_TYPE_F32, GGML_TYPE_Q8_0, align_len,
    align_value, f32_to_bf16_bits, f32_to_f16_bits, push_kv_string, push_kv_string_array,
    push_kv_u64, push_q8_0_values, push_tensor_info_with_type, push_typed_tensor_info,
    push_typed_tensor_values, push_u32, push_u64, q8_storage_bytes, typed_storage_bytes,
};
use crate::scalar_llama_tensors::{
    matrix_dims, q4_k_scalar_tensors, q5_0_scalar_tensors, q6_k_scalar_tensors, q8_scalar_tensors,
};

/// Builds the minimal dense F32 Llama GGUF fixture.
#[must_use]
pub fn scalar_llama_f32_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_F32, true, None)
}

/// Builds the minimal dense F32 Llama fixture with an explicit context length.
#[must_use]
pub fn scalar_llama_f32_gguf_fixture_with_context_length(context_length: u64) -> Vec<u8> {
    scalar_llama_gguf_fixture_with_context_length(GGML_TYPE_F32, true, None, context_length)
}

/// Builds the dense F32 fixture with an explicit tokenizer EOS token ID.
#[must_use]
pub fn scalar_llama_f32_gguf_fixture_with_eos_token_id(eos_token_id: u64) -> Vec<u8> {
    scalar_llama_gguf_fixture(
        GGML_TYPE_F32,
        true,
        Some(("tokenizer.ggml.eos_token_id", eos_token_id)),
    )
}

/// Builds the dense F32 fixture with an explicit tokenizer EOT token ID.
#[must_use]
pub fn scalar_llama_f32_gguf_fixture_with_eot_token_id(eot_token_id: u64) -> Vec<u8> {
    scalar_llama_gguf_fixture(
        GGML_TYPE_F32,
        true,
        Some(("tokenizer.ggml.eot_token_id", eot_token_id)),
    )
}

/// Builds an F32 fixture whose output projection shares token embeddings.
#[must_use]
pub fn scalar_llama_tied_output_f32_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_F32, false, None)
}

/// Builds the minimal dense F16 Llama GGUF fixture.
#[must_use]
pub fn scalar_llama_f16_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_F16, true, None)
}

/// Builds the minimal dense BF16 Llama GGUF fixture.
#[must_use]
pub fn scalar_llama_bf16_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_BF16, true, None)
}

/// Builds the minimal quantized `Q8_0` Llama GGUF fixture.
#[must_use]
pub fn scalar_llama_q8_0_gguf_fixture() -> Vec<u8> {
    let alignment = 64u64;
    let mut tensors = q8_scalar_tensors();

    let mut offset = 0u64;
    for tensor in &mut tensors {
        tensor.offset = align_value(offset, alignment);
        offset = tensor.offset + q8_storage_bytes(tensor.values.len());
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, tensors.len() as u64);
    push_u64(&mut bytes, 13);
    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u64(&mut bytes, "general.alignment", alignment);
    push_kv_u64(&mut bytes, "llama.context_length", 128);
    push_kv_u64(&mut bytes, "llama.embedding_length", 32);
    push_kv_u64(&mut bytes, "llama.block_count", 1);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 32);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 1);
    push_kv_u64(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "llama.attention.key_length", 32);
    push_kv_u64(&mut bytes, "llama.attention.value_length", 32);
    push_kv_u64(&mut bytes, "llama.rope.dimension_count", 2);
    push_kv_string(&mut bytes, "tokenizer.ggml.model", "llama");
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", &["<unk>", "winner"]);

    for tensor in &tensors {
        push_tensor_info_with_type(&mut bytes, tensor, GGML_TYPE_Q8_0);
    }
    align_len(&mut bytes, alignment as usize);

    let tensor_data_start = bytes.len();
    for tensor in &tensors {
        let target_len = tensor_data_start + tensor.offset as usize;
        if bytes.len() < target_len {
            bytes.resize(target_len, 0);
        }
        push_q8_0_values(&mut bytes, &tensor.values);
    }

    bytes
}

/// Builds the minimal quantized `Q5_0` Llama GGUF fixture.
#[must_use]
pub fn scalar_llama_q5_0_gguf_fixture() -> Vec<u8> {
    let alignment = 64u64;
    let mut tensors = q5_0_scalar_tensors();

    let mut offset = 0u64;
    for tensor in &mut tensors {
        tensor.offset = align_value(offset, alignment);
        offset = tensor.offset + typed_storage_bytes(tensor);
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, tensors.len() as u64);
    push_u64(&mut bytes, 13);
    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u64(&mut bytes, "general.alignment", alignment);
    push_kv_u64(&mut bytes, "llama.context_length", 128);
    push_kv_u64(&mut bytes, "llama.embedding_length", 32);
    push_kv_u64(&mut bytes, "llama.block_count", 1);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 32);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 1);
    push_kv_u64(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "llama.attention.key_length", 32);
    push_kv_u64(&mut bytes, "llama.attention.value_length", 32);
    push_kv_u64(&mut bytes, "llama.rope.dimension_count", 2);
    push_kv_string(&mut bytes, "tokenizer.ggml.model", "llama");
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", &["<unk>", "winner"]);

    for tensor in &tensors {
        push_typed_tensor_info(&mut bytes, tensor);
    }
    align_len(&mut bytes, alignment as usize);

    let tensor_data_start = bytes.len();
    for tensor in &tensors {
        let target_len = tensor_data_start + tensor.offset as usize;
        if bytes.len() < target_len {
            bytes.resize(target_len, 0);
        }
        push_typed_tensor_values(&mut bytes, tensor);
    }

    bytes
}

/// Builds the minimal quantized `Q4_K` Llama GGUF fixture.
#[must_use]
pub fn scalar_llama_q4_k_gguf_fixture() -> Vec<u8> {
    let alignment = 64u64;
    let mut tensors = q4_k_scalar_tensors();

    let mut offset = 0u64;
    for tensor in &mut tensors {
        tensor.offset = align_value(offset, alignment);
        offset = tensor.offset + typed_storage_bytes(tensor);
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, tensors.len() as u64);
    push_u64(&mut bytes, 13);
    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u64(&mut bytes, "general.alignment", alignment);
    push_kv_u64(&mut bytes, "llama.context_length", 128);
    push_kv_u64(&mut bytes, "llama.embedding_length", 64);
    push_kv_u64(&mut bytes, "llama.block_count", 1);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 64);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 1);
    push_kv_u64(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "llama.attention.key_length", 64);
    push_kv_u64(&mut bytes, "llama.attention.value_length", 64);
    push_kv_u64(&mut bytes, "llama.rope.dimension_count", 2);
    push_kv_string(&mut bytes, "tokenizer.ggml.model", "llama");
    push_kv_string_array(
        &mut bytes,
        "tokenizer.ggml.tokens",
        &["<unk>", "winner", "hello", "other"],
    );

    for tensor in &tensors {
        push_typed_tensor_info(&mut bytes, tensor);
    }
    align_len(&mut bytes, alignment as usize);

    let tensor_data_start = bytes.len();
    for tensor in &tensors {
        let target_len = tensor_data_start + tensor.offset as usize;
        if bytes.len() < target_len {
            bytes.resize(target_len, 0);
        }
        push_typed_tensor_values(&mut bytes, tensor);
    }

    bytes
}

/// Builds the minimal quantized `Q6_K` Llama GGUF fixture.
#[must_use]
pub fn scalar_llama_q6_k_gguf_fixture() -> Vec<u8> {
    let alignment = 64u64;
    let mut tensors = q6_k_scalar_tensors();

    let mut offset = 0u64;
    for tensor in &mut tensors {
        tensor.offset = align_value(offset, alignment);
        offset = tensor.offset + typed_storage_bytes(tensor);
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, tensors.len() as u64);
    push_u64(&mut bytes, 13);
    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u64(&mut bytes, "general.alignment", alignment);
    push_kv_u64(&mut bytes, "llama.context_length", 128);
    push_kv_u64(&mut bytes, "llama.embedding_length", 64);
    push_kv_u64(&mut bytes, "llama.block_count", 1);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 64);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 1);
    push_kv_u64(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "llama.attention.key_length", 64);
    push_kv_u64(&mut bytes, "llama.attention.value_length", 64);
    push_kv_u64(&mut bytes, "llama.rope.dimension_count", 2);
    push_kv_string(&mut bytes, "tokenizer.ggml.model", "llama");
    push_kv_string_array(
        &mut bytes,
        "tokenizer.ggml.tokens",
        &["<unk>", "winner", "hello", "other"],
    );

    for tensor in &tensors {
        push_typed_tensor_info(&mut bytes, tensor);
    }
    align_len(&mut bytes, alignment as usize);

    let tensor_data_start = bytes.len();
    for tensor in &tensors {
        let target_len = tensor_data_start + tensor.offset as usize;
        if bytes.len() < target_len {
            bytes.resize(target_len, 0);
        }
        push_typed_tensor_values(&mut bytes, tensor);
    }

    bytes
}

fn scalar_llama_gguf_fixture(
    tensor_type: u32,
    include_output_weight: bool,
    end_token: Option<(&str, u64)>,
) -> Vec<u8> {
    scalar_llama_gguf_fixture_with_context_length(
        tensor_type,
        include_output_weight,
        end_token,
        128,
    )
}

fn scalar_llama_gguf_fixture_with_context_length(
    tensor_type: u32,
    include_output_weight: bool,
    end_token: Option<(&str, u64)>,
    context_length: u64,
) -> Vec<u8> {
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

    if !include_output_weight {
        tensors.retain(|tensor| tensor.name != "output.weight");
    }

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
    push_u64(&mut bytes, if end_token.is_some() { 14 } else { 13 });
    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u64(&mut bytes, "general.alignment", alignment);
    push_kv_u64(&mut bytes, "llama.context_length", context_length);
    push_kv_u64(&mut bytes, "llama.embedding_length", 2);
    push_kv_u64(&mut bytes, "llama.block_count", 1);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 2);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 1);
    push_kv_u64(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "llama.attention.key_length", 2);
    push_kv_u64(&mut bytes, "llama.attention.value_length", 2);
    push_kv_u64(&mut bytes, "llama.rope.dimension_count", 2);
    push_kv_string(&mut bytes, "tokenizer.ggml.model", "llama");
    push_kv_string_array(
        &mut bytes,
        "tokenizer.ggml.tokens",
        &["<unk>", "hello", "winner"],
    );
    if let Some((metadata_key, token_id)) = end_token {
        push_kv_u64(&mut bytes, metadata_key, token_id);
    }

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
