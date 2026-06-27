use crate::gguf_writer::{
    align_len, align_value, f32_to_bf16_bits, f32_to_f16_bits, push_kv_string,
    push_kv_string_array, push_kv_u64, push_q8_0_values, push_tensor_info_with_type,
    push_typed_tensor_info, push_typed_tensor_values, push_u32, push_u64, q8_storage_bytes,
    typed_storage_bytes, F32TensorFixture, TypedTensorFixture, GGML_TYPE_BF16, GGML_TYPE_F16,
    GGML_TYPE_F32, GGML_TYPE_Q4_K, GGML_TYPE_Q8_0,
};

pub fn scalar_llama_f32_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_F32)
}

pub fn scalar_llama_f16_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_F16)
}

pub fn scalar_llama_bf16_gguf_fixture() -> Vec<u8> {
    scalar_llama_gguf_fixture(GGML_TYPE_BF16)
}

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
    push_kv_u64(&mut bytes, "llama.context_length", 1);
    push_kv_u64(&mut bytes, "llama.embedding_length", 32);
    push_kv_u64(&mut bytes, "llama.block_count", 1);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 32);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 1);
    push_kv_u64(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "llama.attention.key_length", 32);
    push_kv_u64(&mut bytes, "llama.attention.value_length", 32);
    push_kv_u64(&mut bytes, "llama.rope.dimension_count", 0);
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
    push_kv_u64(&mut bytes, "llama.context_length", 1);
    push_kv_u64(&mut bytes, "llama.embedding_length", 64);
    push_kv_u64(&mut bytes, "llama.block_count", 1);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 64);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 1);
    push_kv_u64(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "llama.attention.key_length", 64);
    push_kv_u64(&mut bytes, "llama.attention.value_length", 64);
    push_kv_u64(&mut bytes, "llama.rope.dimension_count", 0);
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
    push_u64(&mut bytes, 13);
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
    push_kv_string(&mut bytes, "tokenizer.ggml.model", "llama");
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

fn q8_scalar_tensors() -> Vec<F32TensorFixture> {
    let hidden = 32usize;
    let intermediate = 32usize;
    vec![
        F32TensorFixture {
            name: "token_embd.weight",
            dimensions: matrix_dims(hidden as u64, 2),
            values: two_row_values(hidden, 1.0, 0.0),
            offset: 0,
        },
        F32TensorFixture {
            name: "output_norm.weight",
            dimensions: vec![hidden as u64],
            values: vec![1.0; hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "output.weight",
            dimensions: matrix_dims(hidden as u64, 2),
            values: two_row_values(hidden, 0.0, 1.0),
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_norm.weight",
            dimensions: vec![hidden as u64],
            values: vec![1.0; hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_q.weight",
            dimensions: matrix_dims(hidden as u64, hidden as u64),
            values: vec![0.0; hidden * hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_k.weight",
            dimensions: matrix_dims(hidden as u64, hidden as u64),
            values: vec![0.0; hidden * hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_v.weight",
            dimensions: matrix_dims(hidden as u64, hidden as u64),
            values: vec![0.0; hidden * hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_output.weight",
            dimensions: matrix_dims(hidden as u64, hidden as u64),
            values: vec![0.0; hidden * hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_norm.weight",
            dimensions: vec![hidden as u64],
            values: vec![1.0; hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_gate.weight",
            dimensions: matrix_dims(hidden as u64, intermediate as u64),
            values: vec![0.0; hidden * intermediate],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_up.weight",
            dimensions: matrix_dims(hidden as u64, intermediate as u64),
            values: vec![0.0; hidden * intermediate],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_down.weight",
            dimensions: matrix_dims(intermediate as u64, hidden as u64),
            values: vec![0.0; hidden * intermediate],
            offset: 0,
        },
    ]
}

fn q4_k_scalar_tensors() -> Vec<TypedTensorFixture> {
    let hidden = 64usize;
    let intermediate = 64usize;
    vec![
        q4_k_tensor(
            "token_embd.weight",
            matrix_dims(hidden as u64, 4),
            four_row_values(hidden, &[1.0, 0.0, 0.0, 0.0]),
        ),
        f32_tensor("output_norm.weight", vec![hidden as u64], vec![1.0; hidden]),
        q4_k_tensor(
            "output.weight",
            matrix_dims(hidden as u64, 4),
            four_row_values(hidden, &[0.0, 1.0, 0.0, 0.0]),
        ),
        f32_tensor(
            "blk.0.attn_norm.weight",
            vec![hidden as u64],
            vec![1.0; hidden],
        ),
        q4_k_tensor(
            "blk.0.attn_q.weight",
            matrix_dims(hidden as u64, hidden as u64),
            vec![0.0; hidden * hidden],
        ),
        q4_k_tensor(
            "blk.0.attn_k.weight",
            matrix_dims(hidden as u64, hidden as u64),
            vec![0.0; hidden * hidden],
        ),
        q4_k_tensor(
            "blk.0.attn_v.weight",
            matrix_dims(hidden as u64, hidden as u64),
            vec![0.0; hidden * hidden],
        ),
        q4_k_tensor(
            "blk.0.attn_output.weight",
            matrix_dims(hidden as u64, hidden as u64),
            vec![0.0; hidden * hidden],
        ),
        f32_tensor(
            "blk.0.ffn_norm.weight",
            vec![hidden as u64],
            vec![1.0; hidden],
        ),
        q4_k_tensor(
            "blk.0.ffn_gate.weight",
            matrix_dims(hidden as u64, intermediate as u64),
            vec![0.0; hidden * intermediate],
        ),
        q4_k_tensor(
            "blk.0.ffn_up.weight",
            matrix_dims(hidden as u64, intermediate as u64),
            vec![0.0; hidden * intermediate],
        ),
        q4_k_tensor(
            "blk.0.ffn_down.weight",
            matrix_dims(intermediate as u64, hidden as u64),
            vec![0.0; hidden * intermediate],
        ),
    ]
}

fn q4_k_tensor(name: &'static str, dimensions: Vec<u64>, values: Vec<f32>) -> TypedTensorFixture {
    TypedTensorFixture {
        name,
        dimensions,
        values,
        tensor_type: GGML_TYPE_Q4_K,
        offset: 0,
    }
}

fn f32_tensor(name: &'static str, dimensions: Vec<u64>, values: Vec<f32>) -> TypedTensorFixture {
    TypedTensorFixture {
        name,
        dimensions,
        values,
        tensor_type: GGML_TYPE_F32,
        offset: 0,
    }
}

fn two_row_values(cols: usize, first: f32, second: f32) -> Vec<f32> {
    let mut values = vec![first; cols];
    values.extend(vec![second; cols]);
    values
}

fn four_row_values(cols: usize, row_values: &[f32; 4]) -> Vec<f32> {
    let mut values = Vec::with_capacity(cols * row_values.len());
    for row_value in row_values {
        values.extend(vec![*row_value; cols]);
    }
    values
}

fn matrix_dims(cols: u64, rows: u64) -> Vec<u64> {
    vec![cols, rows]
}
