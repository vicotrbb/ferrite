mod support;

use ferrite_model::gguf::{parse_gguf, GgmlType, MetadataValue};
use std::error::Error;
use std::io;
use support::gguf::{
    align_len, gguf_with_single_tensor_shape, minimal_llama_gguf,
    minimal_llama_gguf_with_tensor_offset, push_kv_string, push_kv_string_array, push_kv_u32,
    push_tensor_info, push_u32, push_u64,
};

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

#[test]
fn rejects_duplicate_metadata_keys() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, 0);
    push_u64(&mut bytes, 2);
    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_string(&mut bytes, "general.architecture", "qwen2");

    let error = match parse_gguf(&bytes) {
        Ok(_) => return Err(io::Error::other("duplicate metadata key should be rejected").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("duplicate metadata key general.architecture"));
    Ok(())
}

#[test]
fn rejects_duplicate_tensor_names() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, 2);
    push_u64(&mut bytes, 3);

    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_u32(&mut bytes, "general.alignment", 64);
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", &["<unk>", "hello"]);

    push_tensor_info(&mut bytes, "token_embd.weight", &[1], GgmlType::F32, 0);
    push_tensor_info(&mut bytes, "token_embd.weight", &[1], GgmlType::F32, 64);
    align_len(&mut bytes, 64);
    bytes.extend_from_slice(&1.0f32.to_le_bytes());
    bytes.resize(bytes.len() + 60, 0);
    bytes.extend_from_slice(&2.0f32.to_le_bytes());

    let error = match parse_gguf(&bytes) {
        Ok(_) => return Err(io::Error::other("duplicate tensor name should be rejected").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("duplicate tensor name token_embd.weight"));
    Ok(())
}

#[test]
fn rejects_tensors_with_no_dimensions() -> Result<(), Box<dyn Error>> {
    let bytes = gguf_with_single_tensor_shape(&[]);

    let error = match parse_gguf(&bytes) {
        Ok(_) => return Err(io::Error::other("empty tensor shape should be rejected").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("tensor token_embd.weight must have at least one dimension"));
    Ok(())
}

#[test]
fn rejects_tensors_with_zero_dimensions() -> Result<(), Box<dyn Error>> {
    let bytes = gguf_with_single_tensor_shape(&[8, 0]);

    let error = match parse_gguf(&bytes) {
        Ok(_) => return Err(io::Error::other("zero tensor dimension should be rejected").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("tensor token_embd.weight has zero dimension"));
    Ok(())
}

#[test]
fn rejects_tensors_with_too_many_dimensions() -> Result<(), Box<dyn Error>> {
    let bytes = gguf_with_single_tensor_shape(&[1, 1, 1, 1, 1]);

    let error = match parse_gguf(&bytes) {
        Ok(_) => return Err(io::Error::other("over-rank tensor shape should be rejected").into()),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("tensor token_embd.weight has 5 dimensions; maximum supported is 4"));
    Ok(())
}
