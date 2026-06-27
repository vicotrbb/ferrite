use ferrite_model::gguf::{parse_gguf, GgmlType, MetadataValue};
use std::error::Error;
use std::io;

const VALUE_UINT32: u32 = 4;
const VALUE_FLOAT32: u32 = 6;
const VALUE_STRING: u32 = 8;
const VALUE_ARRAY: u32 = 9;
const VALUE_UINT64: u32 = 10;

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

fn push_kv_u32(bytes: &mut Vec<u8>, key: &str, value: u32) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_UINT32);
    push_u32(bytes, value);
}

fn push_kv_f32(bytes: &mut Vec<u8>, key: &str, value: f32) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_FLOAT32);
    bytes.extend_from_slice(&value.to_le_bytes());
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

fn push_tensor_info(
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

fn align_len(bytes: &mut Vec<u8>, alignment: usize) {
    let padding = (alignment - (bytes.len() % alignment)) % alignment;
    bytes.resize(bytes.len() + padding, 0);
}

fn minimal_llama_gguf() -> Vec<u8> {
    minimal_llama_gguf_with_tensor_offset(0)
}

fn minimal_llama_gguf_with_tensor_offset(tensor_offset: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, 1);
    push_u64(&mut bytes, 15);

    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u32(&mut bytes, "general.quantization_version", 2);
    push_kv_u32(&mut bytes, "general.alignment", 64);
    push_kv_u64(&mut bytes, "llama.context_length", 2048);
    push_kv_u64(&mut bytes, "llama.embedding_length", 8);
    push_kv_u64(&mut bytes, "llama.block_count", 2);
    push_kv_u64(&mut bytes, "llama.feed_forward_length", 16);
    push_kv_u64(&mut bytes, "llama.attention.head_count", 2);
    push_kv_u32(&mut bytes, "llama.attention.head_count_kv", 1);
    push_kv_u32(&mut bytes, "llama.attention.key_length", 4);
    push_kv_u32(&mut bytes, "llama.attention.value_length", 4);
    push_kv_f32(
        &mut bytes,
        "llama.attention.layer_norm_rms_epsilon",
        0.00001,
    );
    push_kv_u32(&mut bytes, "llama.rope.dimension_count", 4);
    push_kv_f32(&mut bytes, "llama.rope.freq_base", 10000.0);
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", &["<unk>", "hello"]);

    push_tensor_info(
        &mut bytes,
        "token_embd.weight",
        &[8, 2],
        GgmlType::F32,
        tensor_offset,
    );
    align_len(&mut bytes, 64);

    for value in 0..16u32 {
        bytes.extend_from_slice(&(value as f32).to_le_bytes());
    }

    bytes
}

#[test]
fn parses_gguf_header_metadata_tensor_info_and_data_ranges() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf();

    let file = parse_gguf(&bytes)?;

    assert_eq!(file.version, 3);
    assert_eq!(file.alignment, 64);
    assert_eq!(
        file.metadata.get("general.architecture"),
        Some(&MetadataValue::String("llama".to_owned()))
    );
    assert_eq!(
        file.metadata.get("tokenizer.ggml.tokens"),
        Some(&MetadataValue::Array {
            value_type: ferrite_model::gguf::MetadataValueType::String,
            values: vec![
                MetadataValue::String("<unk>".to_owned()),
                MetadataValue::String("hello".to_owned()),
            ],
        })
    );

    let Some(tensor) = file.tensor("token_embd.weight") else {
        return Err(io::Error::other("token_embd.weight tensor should exist").into());
    };
    assert_eq!(tensor.dimensions, vec![8, 2]);
    assert_eq!(tensor.ty, GgmlType::F32);
    assert_eq!(tensor.relative_offset, 0);
    assert_eq!(tensor.data_range.len(), 64);
    assert_eq!(tensor.data_range.start % 64, 0);
    assert_eq!(
        &bytes[tensor.data_range.start..tensor.data_range.start + 4],
        &0f32.to_le_bytes()
    );
    Ok(())
}

#[test]
fn derives_llama_config_from_uint32_or_uint64_metadata() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf();
    let file = parse_gguf(&bytes)?;

    let config = file.llama_config()?;

    assert_eq!(config.context_length, 2048);
    assert_eq!(config.embedding_length, 8);
    assert_eq!(config.block_count, 2);
    assert_eq!(config.feed_forward_length, 16);
    assert_eq!(config.attention_head_count, 2);
    assert_eq!(config.attention_head_count_kv, 1);
    assert_eq!(config.key_length, 4);
    assert_eq!(config.value_length, 4);
    assert_eq!(config.rope_dimension_count, 4);
    assert_eq!(config.rope_freq_base, Some(10000.0));
    assert_eq!(config.attention_layer_norm_rms_epsilon, Some(0.00001));
    assert_eq!(config.gqa_ratio(), Some(2));
    Ok(())
}

#[test]
fn rejects_tensor_offsets_that_violate_alignment() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_tensor_offset(1);

    let error = match parse_gguf(&bytes) {
        Ok(_) => {
            return Err(io::Error::other("misaligned tensor offset should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("tensor offset 1 is not aligned to 64"));
    Ok(())
}

#[test]
fn rejects_invalid_metadata_keys() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, 0);
    push_u64(&mut bytes, 1);
    push_kv_string(&mut bytes, "General.Architecture", "llama");

    let error = match parse_gguf(&bytes) {
        Ok(_) => return Err(io::Error::other("invalid metadata key should be rejected").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("metadata key is not valid GGUF lower_snake_case hierarchy"));
    Ok(())
}
