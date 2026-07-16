use crate::gguf_writer::{
    F32TensorFixture, GGML_TYPE_F32, align_len, align_value, push_kv_f32, push_kv_string,
    push_kv_string_array, push_kv_u64, push_tensor_info_with_type, push_u32, push_u64,
};
use crate::scalar_llama_tensors::matrix_dims;

/// Phi-3 chat template used by Microsoft's official Mini 4K GGUF artifact.
pub const PHI3_INSTRUCT_CHAT_TEMPLATE: &str = "{{ bos_token }}{% for message in messages %}{% if (message['role'] == 'user') %}{{'<|user|>' + '\n' + message['content'] + '<|end|>' + '\n' + '<|assistant|>' + '\n'}}{% elif (message['role'] == 'assistant') %}{{message['content'] + '<|end|>' + '\n'}}{% endif %}{% endfor %}";

/// Builds a minimal dense Phi-3 GGUF fixture with fused QKV and gate/up
/// tensors. The tied token embedding selects token ID 1 after token ID 2.
#[must_use]
pub fn scalar_phi3_f32_gguf_fixture() -> Vec<u8> {
    let alignment = 64u64;
    let hidden = 4usize;
    let intermediate = 4usize;
    let tokens = [
        "<unk>",
        "winner",
        "hello",
        "<|user|>",
        "<|assistant|>",
        "<|end|>",
        "<s>",
    ];
    let mut embeddings = vec![0.0; hidden * tokens.len()];
    embeddings[hidden] = 2.0;
    embeddings[hidden * 2] = 1.0;
    let mut tensors = vec![
        F32TensorFixture {
            name: "token_embd.weight",
            dimensions: matrix_dims(hidden as u64, tokens.len() as u64),
            values: embeddings,
            offset: 0,
        },
        F32TensorFixture {
            name: "output_norm.weight",
            dimensions: vec![hidden as u64],
            values: vec![1.0; hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_norm.weight",
            dimensions: vec![hidden as u64],
            values: vec![1.0; hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_qkv.weight",
            dimensions: matrix_dims(hidden as u64, (hidden * 3) as u64),
            values: vec![0.0; hidden * hidden * 3],
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
            name: "blk.0.ffn_up.weight",
            dimensions: matrix_dims(hidden as u64, (intermediate * 2) as u64),
            values: vec![0.0; hidden * intermediate * 2],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_down.weight",
            dimensions: matrix_dims(intermediate as u64, hidden as u64),
            values: vec![0.0; hidden * intermediate],
            offset: 0,
        },
    ];

    let mut offset = 0u64;
    for tensor in &mut tensors {
        tensor.offset = align_value(offset, alignment);
        offset = tensor.offset + (tensor.values.len() * std::mem::size_of::<f32>()) as u64;
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, tensors.len() as u64);
    push_u64(&mut bytes, 15);
    push_kv_string(&mut bytes, "general.architecture", "phi3");
    push_kv_u64(&mut bytes, "general.alignment", alignment);
    push_kv_u64(&mut bytes, "phi3.context_length", 4096);
    push_kv_u64(&mut bytes, "phi3.embedding_length", hidden as u64);
    push_kv_u64(&mut bytes, "phi3.block_count", 1);
    push_kv_u64(&mut bytes, "phi3.feed_forward_length", intermediate as u64);
    push_kv_u64(&mut bytes, "phi3.attention.head_count", 1);
    push_kv_u64(&mut bytes, "phi3.attention.head_count_kv", 1);
    push_kv_u64(&mut bytes, "phi3.rope.dimension_count", hidden as u64);
    push_kv_f32(&mut bytes, "phi3.attention.layer_norm_rms_epsilon", 1.0e-5);
    push_kv_string(&mut bytes, "tokenizer.ggml.model", "llama");
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", &tokens);
    push_kv_u64(&mut bytes, "tokenizer.ggml.bos_token_id", 6);
    push_kv_u64(&mut bytes, "tokenizer.ggml.eos_token_id", 5);
    push_kv_string(
        &mut bytes,
        "tokenizer.chat_template",
        PHI3_INSTRUCT_CHAT_TEMPLATE,
    );

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
