use ferrite_model::gguf::parse_gguf;
use ferrite_model::tokenizer::{GgufTokenizer, TokenType, TokenizerModel};
use std::error::Error;
use std::io;

const VALUE_STRING: u32 = 8;
const VALUE_ARRAY: u32 = 9;
const VALUE_UINT32: u32 = 4;
const VALUE_INT32: u32 = 5;

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
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

fn push_kv_string_array(bytes: &mut Vec<u8>, key: &str, values: &[&str]) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_ARRAY);
    push_u32(bytes, VALUE_STRING);
    push_u64(bytes, values.len() as u64);
    for value in values {
        push_string(bytes, value);
    }
}

fn push_kv_i32_array(bytes: &mut Vec<u8>, key: &str, values: &[i32]) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_ARRAY);
    push_u32(bytes, VALUE_INT32);
    push_u64(bytes, values.len() as u64);
    for value in values {
        push_i32(bytes, *value);
    }
}

fn tokenizer_gguf_fixture(tokens: &[&str], token_types: &[i32], merges: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, 0);
    let metadata_count = if merges.is_empty() { 4 } else { 5 };
    push_u64(&mut bytes, metadata_count);
    push_kv_string(&mut bytes, "general.architecture", "llama");
    push_kv_string(&mut bytes, "tokenizer.ggml.model", "llama");
    push_kv_string_array(&mut bytes, "tokenizer.ggml.tokens", tokens);
    push_kv_i32_array(&mut bytes, "tokenizer.ggml.token_type", token_types);
    if !merges.is_empty() {
        push_kv_string_array(&mut bytes, "tokenizer.ggml.merges", merges);
    }
    bytes
}

fn atomic_tokenizer_fixture() -> Vec<u8> {
    tokenizer_gguf_fixture(
        &["<unk>", "hello", " ", "world", "!"],
        &[2, 1, 1, 1, 1],
        &[],
    )
}

fn eos_tokenizer_fixture() -> Vec<u8> {
    let mut bytes = atomic_tokenizer_fixture();
    let metadata_count_offset = 4 + 4 + 8;
    let mut metadata_count_bytes = [0; 8];
    metadata_count_bytes.copy_from_slice(&bytes[metadata_count_offset..metadata_count_offset + 8]);
    let metadata_count = u64::from_le_bytes(metadata_count_bytes);
    bytes[metadata_count_offset..metadata_count_offset + 8]
        .copy_from_slice(&(metadata_count + 1).to_le_bytes());
    push_kv_u32(&mut bytes, "tokenizer.ggml.eos_token_id", 4);
    bytes
}

fn bpe_tokenizer_fixture() -> Vec<u8> {
    tokenizer_gguf_fixture(
        &[
            "<unk>", "h", "e", "l", "o", "he", "ll", "hell", "hello", "Ġ",
        ],
        &[2, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        &["h e", "l l", "he ll", "hell o"],
    )
}

fn byte_bpe_tokenizer_fixture() -> Vec<u8> {
    tokenizer_gguf_fixture(
        &[
            "<unk>", "h", "e", "l", "o", "he", "ll", "hell", "hello", "Ġ", "c", "a", "f", "Ã", "©",
            "ca", "caf", "Ã©", "cafÃ©", "Ċ",
        ],
        &[2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        &[
            "h e", "l l", "he ll", "hell o", "c a", "ca f", "Ã ©", "caf Ã©",
        ],
    )
}

#[test]
fn extracts_gguf_tokenizer_metadata_and_decodes_tokens() -> Result<(), Box<dyn Error>> {
    let bytes = atomic_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;

    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.model(), TokenizerModel::Llama);
    assert_eq!(tokenizer.len(), 5);
    assert_eq!(tokenizer.token(0), Some("<unk>"));
    assert_eq!(tokenizer.token_type(0), Some(TokenType::Unknown));
    assert_eq!(tokenizer.token_type(1), Some(TokenType::Normal));
    assert_eq!(tokenizer.eos_token_id(), None);
    assert_eq!(tokenizer.decode(&[1, 2, 3, 4])?, "hello world!");
    Ok(())
}

#[test]
fn extracts_eos_token_id_from_gguf_metadata() -> Result<(), Box<dyn Error>> {
    let bytes = eos_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;

    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.eos_token_id(), Some(4));
    Ok(())
}

#[test]
fn atomically_encodes_with_longest_matching_tokens() -> Result<(), Box<dyn Error>> {
    let bytes = atomic_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.encode_atomic("hello world!")?, vec![1, 2, 3, 4]);

    let error = match tokenizer.encode_atomic("hello unknown") {
        Ok(_) => return Err(io::Error::other("unknown text should fail atomic encoding").into()),
        Err(error) => error,
    };
    assert!(error.to_string().contains("no atomic token matches input"));
    Ok(())
}

#[test]
fn encodes_with_ranked_bpe_merges_from_gguf_metadata() -> Result<(), Box<dyn Error>> {
    let bytes = bpe_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.encode_bpe("hello hello")?, vec![8, 9, 8]);
    Ok(())
}

#[test]
fn bpe_seeds_from_gpt2_byte_alphabet_before_merging() -> Result<(), Box<dyn Error>> {
    let bytes = byte_bpe_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.encode_bpe(" hello")?, vec![9, 8]);
    assert_eq!(tokenizer.encode_bpe("café")?, vec![18]);
    Ok(())
}

#[test]
fn bpe_decodes_gpt2_byte_alphabet_tokens() -> Result<(), Box<dyn Error>> {
    let bytes = byte_bpe_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.decode(&[9, 8, 19, 18])?, " hello\ncafé");
    Ok(())
}
