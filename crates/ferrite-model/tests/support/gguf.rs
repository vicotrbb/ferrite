#![allow(dead_code)]

use ferrite_model::gguf::GgmlType;

const VALUE_UINT32: u32 = 4;
const VALUE_FLOAT32: u32 = 6;
const VALUE_STRING: u32 = 8;
const VALUE_ARRAY: u32 = 9;
const VALUE_UINT64: u32 = 10;

pub(crate) fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn push_string(bytes: &mut Vec<u8>, value: &str) {
    push_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

pub(crate) fn push_kv_string(bytes: &mut Vec<u8>, key: &str, value: &str) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_STRING);
    push_string(bytes, value);
}

pub(crate) fn push_kv_u32(bytes: &mut Vec<u8>, key: &str, value: u32) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_UINT32);
    push_u32(bytes, value);
}

pub(crate) fn push_kv_f32(bytes: &mut Vec<u8>, key: &str, value: f32) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_FLOAT32);
    bytes.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn push_kv_u64(bytes: &mut Vec<u8>, key: &str, value: u64) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_UINT64);
    push_u64(bytes, value);
}

pub(crate) fn push_kv_string_array(bytes: &mut Vec<u8>, key: &str, values: &[&str]) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_ARRAY);
    push_u32(bytes, VALUE_STRING);
    push_u64(bytes, values.len() as u64);
    for value in values {
        push_string(bytes, value);
    }
}

pub(crate) fn push_tensor_info(
    bytes: &mut Vec<u8>,
    name: &str,
    dimensions: &[u64],
    tensor_type: GgmlType,
    offset: u64,
) {
    push_string(bytes, name);
    push_u32(bytes, dimensions.len() as u32);
    for dimension in dimensions {
        push_u64(bytes, *dimension);
    }
    push_u32(bytes, tensor_type as u32);
    push_u64(bytes, offset);
}

pub(crate) fn align_len(bytes: &mut Vec<u8>, alignment: usize) {
    let padding = (alignment - (bytes.len() % alignment)) % alignment;
    bytes.resize(bytes.len() + padding, 0);
}

pub(crate) fn minimal_llama_gguf() -> Vec<u8> {
    minimal_llama_gguf_with_tensor_offset(0)
}

pub(crate) fn minimal_llama_gguf_with_context_length(context_length: u64) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        context_length,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_llama_gguf_with_embedding_length(embedding_length: u64) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        embedding_length,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_llama_gguf_with_block_count(block_count: u64) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        block_count,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_llama_gguf_with_feed_forward_length(feed_forward_length: u64) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        feed_forward_length,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_llama_gguf_with_attention_head_count(attention_head_count: u64) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        attention_head_count,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_llama_gguf_with_attention_head_count_kv(
    attention_head_count_kv: u32,
) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        attention_head_count_kv,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_llama_gguf_with_attention_heads(
    attention_head_count: u64,
    attention_head_count_kv: u32,
) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        embedding_length: attention_head_count * 4,
        attention_head_count,
        attention_head_count_kv,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_llama_gguf_with_key_length(key_length: u32) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        key_length,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_llama_gguf_with_value_length(value_length: u32) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        value_length,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_llama_gguf_with_rope_dimension_count(rope_dimension_count: u32) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        rope_dimension_count,
        ..LlamaGgufOptions::default()
    })
}

pub(crate) fn minimal_qwen2_gguf() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, 1);
    push_u64(&mut bytes, 12);

    push_kv_string(&mut bytes, "general.architecture", "qwen2");
    push_kv_u32(&mut bytes, "general.quantization_version", 2);
    push_kv_u32(&mut bytes, "general.alignment", 64);
    push_kv_u64(&mut bytes, "qwen2.context_length", 32768);
    push_kv_u64(&mut bytes, "qwen2.embedding_length", 896);
    push_kv_u64(&mut bytes, "qwen2.block_count", 24);
    push_kv_u64(&mut bytes, "qwen2.feed_forward_length", 4864);
    push_kv_u64(&mut bytes, "qwen2.attention.head_count", 14);
    push_kv_u64(&mut bytes, "qwen2.attention.head_count_kv", 2);
    push_kv_f32(
        &mut bytes,
        "qwen2.attention.layer_norm_rms_epsilon",
        0.000001,
    );
    push_kv_f32(&mut bytes, "qwen2.rope.freq_base", 1_000_000.0);
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", &["<unk>", "hello"]);

    push_tensor_info(&mut bytes, "token_embd.weight", &[896, 2], GgmlType::F32, 0);
    align_len(&mut bytes, 64);

    for value in 0..1792u32 {
        bytes.extend_from_slice(&(value as f32).to_le_bytes());
    }

    bytes
}

pub(crate) fn minimal_llama_gguf_with_tensor_offset(tensor_offset: u64) -> Vec<u8> {
    minimal_llama_gguf_with_options(LlamaGgufOptions {
        tensor_offset,
        ..LlamaGgufOptions::default()
    })
}

struct LlamaGgufOptions {
    tensor_offset: u64,
    context_length: u64,
    embedding_length: u64,
    block_count: u64,
    feed_forward_length: u64,
    attention_head_count: u64,
    attention_head_count_kv: u32,
    key_length: u32,
    value_length: u32,
    rope_dimension_count: u32,
}

impl Default for LlamaGgufOptions {
    fn default() -> Self {
        Self {
            tensor_offset: 0,
            context_length: 2048,
            embedding_length: 8,
            block_count: 2,
            feed_forward_length: 16,
            attention_head_count: 2,
            attention_head_count_kv: 1,
            key_length: 4,
            value_length: 4,
            rope_dimension_count: 4,
        }
    }
}

fn minimal_llama_gguf_with_options(options: LlamaGgufOptions) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, 1);
    push_u64(&mut bytes, 15);

    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u32(&mut bytes, "general.quantization_version", 2);
    push_kv_u32(&mut bytes, "general.alignment", 64);
    push_kv_u64(&mut bytes, "llama.context_length", options.context_length);
    push_kv_u64(
        &mut bytes,
        "llama.embedding_length",
        options.embedding_length,
    );
    push_kv_u64(&mut bytes, "llama.block_count", options.block_count);
    push_kv_u64(
        &mut bytes,
        "llama.feed_forward_length",
        options.feed_forward_length,
    );
    push_kv_u64(
        &mut bytes,
        "llama.attention.head_count",
        options.attention_head_count,
    );
    push_kv_u32(
        &mut bytes,
        "llama.attention.head_count_kv",
        options.attention_head_count_kv,
    );
    push_kv_u32(&mut bytes, "llama.attention.key_length", options.key_length);
    push_kv_u32(
        &mut bytes,
        "llama.attention.value_length",
        options.value_length,
    );
    push_kv_f32(
        &mut bytes,
        "llama.attention.layer_norm_rms_epsilon",
        0.00001,
    );
    push_kv_u32(
        &mut bytes,
        "llama.rope.dimension_count",
        options.rope_dimension_count,
    );
    push_kv_f32(&mut bytes, "llama.rope.freq_base", 10000.0);
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", &["<unk>", "hello"]);

    push_tensor_info(
        &mut bytes,
        "token_embd.weight",
        &[8, 2],
        GgmlType::F32,
        options.tensor_offset,
    );
    align_len(&mut bytes, 64);

    for value in 0..16u32 {
        bytes.extend_from_slice(&(value as f32).to_le_bytes());
    }

    bytes
}

pub(crate) fn gguf_with_single_tensor_shape(dimensions: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, 1);
    push_u64(&mut bytes, 3);

    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u32(&mut bytes, "general.alignment", 64);
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", &["<unk>", "hello"]);

    push_tensor_info(
        &mut bytes,
        "token_embd.weight",
        dimensions,
        GgmlType::F32,
        0,
    );
    align_len(&mut bytes, 64);

    let element_count = dimensions
        .iter()
        .try_fold(1usize, |accumulator, dimension| {
            usize::try_from(*dimension)
                .ok()
                .and_then(|dimension| accumulator.checked_mul(dimension))
        })
        .unwrap_or(0);
    bytes.resize(bytes.len() + element_count * 4, 0);
    bytes
}
