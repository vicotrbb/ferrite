use crate::gguf_writer::{
    align_len, align_value, push_kv_string, push_kv_string_array, push_kv_u64,
    push_tensor_info_with_type, push_u32, push_u64, F32TensorFixture, GGML_TYPE_F32,
};

pub fn scalar_llama_chat_f32_gguf_fixture() -> Vec<u8> {
    let alignment = 64u64;
    let tokens = ["<unk>", "hello", "winner", "user: ", "\n", "assistant: "];
    let mut tensors = tensors();

    let mut offset = 0u64;
    for tensor in &mut tensors {
        tensor.offset = align_value(offset, alignment);
        offset = tensor.offset + tensor.values.len() as u64 * 4;
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, tensors.len() as u64);
    push_u64(&mut bytes, 13);
    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u64(&mut bytes, "general.alignment", alignment);
    push_kv_u64(&mut bytes, "llama.context_length", 8);
    push_kv_u64(&mut bytes, "llama.embedding_length", 2);
    push_kv_u64(&mut bytes, "llama.block_count", 1);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 2);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 1);
    push_kv_u64(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "llama.attention.key_length", 2);
    push_kv_u64(&mut bytes, "llama.attention.value_length", 2);
    push_kv_u64(&mut bytes, "llama.rope.dimension_count", 2);
    push_kv_string(&mut bytes, "tokenizer.ggml.model", "llama");
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", &tokens);

    for tensor in &tensors {
        push_tensor_info_with_type(&mut bytes, tensor, GGML_TYPE_F32);
    }
    align_len(&mut bytes, alignment as usize);

    let tensor_data_start = bytes.len();
    for tensor in &tensors {
        let target_len = tensor_data_start + tensor.offset as usize;
        if bytes.len() < target_len {
            bytes.resize(target_len, 0);
        }
        for value in &tensor.values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }

    bytes
}

fn tensors() -> Vec<F32TensorFixture> {
    vec![
        tensor(
            "token_embd.weight",
            matrix_dims(6, 2),
            vec![1.0, 1.0, 0.0, 1.0, 2.0, -1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0],
        ),
        tensor(
            "output.weight",
            matrix_dims(6, 2),
            vec![0.1, 0.1, 0.2, 0.0, 1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        tensor("output_norm.weight", vec![2], vec![1.0, 1.0]),
        tensor("blk.0.attn_norm.weight", vec![2], vec![1.0, 1.0]),
        tensor("blk.0.attn_q.weight", matrix_dims(2, 2), identity()),
        tensor("blk.0.attn_k.weight", matrix_dims(2, 2), identity()),
        tensor("blk.0.attn_v.weight", matrix_dims(2, 2), identity()),
        tensor("blk.0.attn_output.weight", matrix_dims(2, 2), identity()),
        tensor("blk.0.ffn_norm.weight", vec![2], vec![1.0, 1.0]),
        tensor("blk.0.ffn_gate.weight", matrix_dims(2, 2), identity()),
        tensor("blk.0.ffn_up.weight", matrix_dims(2, 2), identity()),
        tensor("blk.0.ffn_down.weight", matrix_dims(2, 2), identity()),
    ]
}

fn tensor(name: &'static str, dimensions: Vec<u64>, values: Vec<f32>) -> F32TensorFixture {
    F32TensorFixture {
        name,
        dimensions,
        values,
        offset: 0,
    }
}

fn matrix_dims(rows: u64, cols: u64) -> Vec<u64> {
    vec![cols, rows]
}

fn identity() -> Vec<f32> {
    vec![1.0, 0.0, 0.0, 1.0]
}
