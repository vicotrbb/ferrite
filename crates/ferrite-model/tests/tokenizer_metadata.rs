use ferrite_model::gguf::parse_gguf;
use ferrite_model::tokenizer::{GgufTokenizer, TokenType, TokenizationControl, TokenizerModel};
use std::error::Error;
use std::io;

const VALUE_STRING: u32 = 8;
const VALUE_ARRAY: u32 = 9;
const VALUE_UINT32: u32 = 4;
const VALUE_INT32: u32 = 5;
const VALUE_FLOAT32: u32 = 6;
const VALUE_BOOL: u32 = 7;

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(bytes: &mut Vec<u8>, value: f32) {
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

fn push_kv_bool(bytes: &mut Vec<u8>, key: &str, value: bool) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_BOOL);
    bytes.push(u8::from(value));
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

fn push_kv_f32_array(bytes: &mut Vec<u8>, key: &str, values: &[f32]) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_ARRAY);
    push_u32(bytes, VALUE_FLOAT32);
    push_u64(bytes, values.len() as u64);
    for value in values {
        push_f32(bytes, *value);
    }
}

fn tokenizer_gguf_fixture(tokens: &[&str], token_types: &[i32], merges: &[&str]) -> Vec<u8> {
    tokenizer_gguf_fixture_for_architecture("llama", tokens, token_types, merges)
}

fn tokenizer_gguf_fixture_for_architecture(
    architecture: &str,
    tokens: &[&str],
    token_types: &[i32],
    merges: &[&str],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    push_u32(&mut bytes, 3);
    push_u64(&mut bytes, 0);
    let metadata_count = if merges.is_empty() { 4 } else { 5 };
    push_u64(&mut bytes, metadata_count);
    push_kv_string(&mut bytes, "general.architecture", architecture);
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

fn eot_eom_tokenizer_fixture() -> Vec<u8> {
    let mut bytes = atomic_tokenizer_fixture();
    let metadata_count_offset = 4 + 4 + 8;
    let mut metadata_count_bytes = [0; 8];
    metadata_count_bytes.copy_from_slice(&bytes[metadata_count_offset..metadata_count_offset + 8]);
    let metadata_count = u64::from_le_bytes(metadata_count_bytes);
    bytes[metadata_count_offset..metadata_count_offset + 8]
        .copy_from_slice(&(metadata_count + 2).to_le_bytes());
    push_kv_u32(&mut bytes, "tokenizer.ggml.eot_token_id", 1);
    push_kv_u32(&mut bytes, "tokenizer.ggml.eom_token_id", 3);
    bytes
}

fn out_of_range_eot_tokenizer_fixture() -> Vec<u8> {
    let mut bytes = atomic_tokenizer_fixture();
    let metadata_count_offset = 4 + 4 + 8;
    let mut metadata_count_bytes = [0; 8];
    metadata_count_bytes.copy_from_slice(&bytes[metadata_count_offset..metadata_count_offset + 8]);
    let metadata_count = u64::from_le_bytes(metadata_count_bytes);
    bytes[metadata_count_offset..metadata_count_offset + 8]
        .copy_from_slice(&(metadata_count + 1).to_le_bytes());
    push_kv_u32(&mut bytes, "tokenizer.ggml.eot_token_id", 5);
    bytes
}

fn model_native_end_token_fixture(architecture: &str, token: &str, token_type: i32) -> Vec<u8> {
    tokenizer_gguf_fixture_for_architecture(
        architecture,
        &["<unk>", "hello", token],
        &[2, 1, token_type],
        &[],
    )
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

fn special_bpe_tokenizer_fixture(add_boundaries: bool) -> Vec<u8> {
    let mut bytes = tokenizer_gguf_fixture(
        &[
            "<unk>",
            "h",
            "e",
            "l",
            "o",
            "he",
            "ll",
            "hell",
            "hello",
            "Ġ",
            "<|im_start|>",
            "<|im_end|>",
        ],
        &[2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3],
        &["h e", "l l", "he ll", "hell o"],
    );
    if add_boundaries {
        let metadata_count_offset = 4 + 4 + 8;
        let mut metadata_count_bytes = [0; 8];
        metadata_count_bytes
            .copy_from_slice(&bytes[metadata_count_offset..metadata_count_offset + 8]);
        let metadata_count = u64::from_le_bytes(metadata_count_bytes);
        bytes[metadata_count_offset..metadata_count_offset + 8]
            .copy_from_slice(&(metadata_count + 4).to_le_bytes());
        push_kv_u32(&mut bytes, "tokenizer.ggml.bos_token_id", 10);
        push_kv_u32(&mut bytes, "tokenizer.ggml.eos_token_id", 11);
        push_kv_bool(&mut bytes, "tokenizer.ggml.add_bos_token", true);
        push_kv_bool(&mut bytes, "tokenizer.ggml.add_eos_token", true);
    }
    bytes
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

fn spm_tokenizer_fixture() -> Vec<u8> {
    let mut bytes = tokenizer_gguf_fixture_for_architecture(
        "phi3",
        &[
            "<unk>", "<s>", "</s>", "<0xC3>", "<0xA9>", "▁", "a", "b", "▁a", "ab", "▁ab",
            "<|user|>", "h", "i",
        ],
        &[2, 3, 3, 6, 6, 1, 1, 1, 1, 1, 1, 4, 1, 1],
        &[],
    );
    let metadata_count_offset = 4 + 4 + 8;
    let mut metadata_count_bytes = [0; 8];
    metadata_count_bytes.copy_from_slice(&bytes[metadata_count_offset..metadata_count_offset + 8]);
    let metadata_count = u64::from_le_bytes(metadata_count_bytes);
    bytes[metadata_count_offset..metadata_count_offset + 8]
        .copy_from_slice(&(metadata_count + 3).to_le_bytes());
    push_kv_f32_array(
        &mut bytes,
        "tokenizer.ggml.scores",
        &[
            0.0, 0.0, 0.0, 0.0, 0.0, -10.0, -10.0, -10.0, 10.0, 5.0, 20.0, 0.0, -10.0, -10.0,
        ],
    );
    push_kv_u32(&mut bytes, "tokenizer.ggml.bos_token_id", 1);
    push_kv_bool(&mut bytes, "tokenizer.ggml.add_bos_token", true);
    bytes
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
    assert_eq!(tokenizer.end_of_generation_token_ids(), [4]);
    assert!(tokenizer.is_end_of_generation_token(4));
    assert!(!tokenizer.is_end_of_generation_token(3));
    Ok(())
}

#[test]
fn extracts_explicit_eot_and_eom_token_ids_from_gguf_metadata() -> Result<(), Box<dyn Error>> {
    let bytes = eot_eom_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;

    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.eos_token_id(), None);
    assert_eq!(tokenizer.end_of_generation_token_ids(), [1, 3]);
    assert!(tokenizer.is_end_of_generation_token(1));
    assert!(tokenizer.is_end_of_generation_token(3));
    Ok(())
}

#[test]
fn rejects_end_of_generation_token_id_outside_vocabulary() -> Result<(), Box<dyn Error>> {
    let bytes = out_of_range_eot_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;

    let error = match GgufTokenizer::from_gguf(&file) {
        Ok(_) => return Err(io::Error::other("out-of-range EOT token should fail").into()),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("tokenizer.ggml.eot_token_id value 5 is outside vocabulary size 5")
    );
    Ok(())
}

#[test]
fn recognizes_bounded_model_native_turn_terminators() -> Result<(), Box<dyn Error>> {
    for (architecture, token) in [
        ("llama", "</s>"),
        ("llama", "<|eot_id|>"),
        ("qwen2", "<|im_end|>"),
        ("phi3", "<|end|>"),
    ] {
        let bytes = model_native_end_token_fixture(architecture, token, 4);
        let file = parse_gguf(&bytes)?;
        let tokenizer = GgufTokenizer::from_gguf(&file)?;

        assert_eq!(
            tokenizer.end_of_generation_token_ids(),
            [2],
            "architecture {architecture} token {token}"
        );
        assert!(tokenizer.is_end_of_generation_token(2));
    }
    Ok(())
}

#[test]
fn does_not_infer_normal_text_as_model_native_turn_terminator() -> Result<(), Box<dyn Error>> {
    let bytes = model_native_end_token_fixture("phi3", "<|end|>", 1);
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert!(tokenizer.end_of_generation_token_ids().is_empty());
    assert!(!tokenizer.is_end_of_generation_token(2));
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
fn encodes_scored_llama_vocabularies_with_spm_merges_and_byte_fallback()
-> Result<(), Box<dyn Error>> {
    let bytes = spm_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.encode("ab")?, vec![1, 10]);
    assert_eq!(tokenizer.encode("é")?, vec![1, 5, 3, 4]);
    assert_eq!(tokenizer.decode(&[5, 3, 4])?, " é");
    Ok(())
}

#[test]
fn spm_encoding_preserves_user_defined_tokens_and_restarts_the_space_prefix()
-> Result<(), Box<dyn Error>> {
    let bytes = spm_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.encode("<|user|>\n\thi")?, vec![1, 11, 5, 12, 13]);
    Ok(())
}

#[test]
fn bpe_encoding_preserves_control_tokens_atomically() -> Result<(), Box<dyn Error>> {
    let bytes = special_bpe_tokenizer_fixture(false);
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(
        tokenizer.encode_bpe("<|im_start|>hello<|im_end|>")?,
        vec![10, 8, 11]
    );
    Ok(())
}

#[test]
fn configured_encoding_adds_missing_boundary_tokens_once() -> Result<(), Box<dyn Error>> {
    let bytes = special_bpe_tokenizer_fixture(true);
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    assert_eq!(tokenizer.bos_token_id(), Some(10));
    assert_eq!(tokenizer.eos_token_id(), Some(11));
    assert!(tokenizer.adds_bos_token());
    assert!(tokenizer.adds_eos_token());
    assert_eq!(tokenizer.encode("hello")?, vec![10, 8, 11]);
    assert_eq!(
        tokenizer.encode("<|im_start|>hello<|im_end|>")?,
        vec![10, 8, 11]
    );
    Ok(())
}

#[test]
fn bpe_encoder_does_not_poll_through_irrelevant_merge_rules() -> Result<(), Box<dyn Error>> {
    let mut tokens = ["<unk>", "h", "e", "l", "o", "he", "ll", "hell", "hello"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut merges = ["h e", "l l", "he ll", "hell o"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    for index in 0..64 {
        let left = format!("x{index}");
        let right = format!("y{index}");
        let merged = format!("{left}{right}");
        merges.push(format!("{left} {right}"));
        tokens.push(left);
        tokens.push(right);
        tokens.push(merged);
    }

    let token_types = std::iter::once(2)
        .chain(std::iter::repeat_n(1, tokens.len() - 1))
        .collect::<Vec<_>>();
    let token_refs = tokens.iter().map(String::as_str).collect::<Vec<_>>();
    let merge_refs = merges.iter().map(String::as_str).collect::<Vec<_>>();
    let bytes = tokenizer_gguf_fixture(&token_refs, &token_types, &merge_refs);
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;
    let mut polls = 0usize;

    let token_ids = tokenizer.encode_bpe_with_cancellation("hello", || {
        polls += 1;
        TokenizationControl::Continue
    })?;

    assert_eq!(token_ids, vec![8]);
    assert!(
        polls < 20,
        "encoder should not poll through irrelevant merge rules, got {polls}"
    );
    Ok(())
}

#[test]
fn rejects_invalid_bpe_merge_metadata_at_load_time() -> Result<(), Box<dyn Error>> {
    let bytes = tokenizer_gguf_fixture(&["<unk>", "h", "e", "he"], &[2, 1, 1, 1], &["h e extra"]);
    let file = parse_gguf(&bytes)?;

    let error = match GgufTokenizer::from_gguf(&file) {
        Ok(_) => return Err(io::Error::other("invalid merge should fail tokenizer load").into()),
        Err(error) => error,
    };

    assert!(error.to_string().contains("invalid BPE merge rule"));
    Ok(())
}

#[test]
fn bpe_tokenization_can_be_cancelled() -> Result<(), Box<dyn Error>> {
    let bytes = bpe_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;
    let mut polls = 0;

    let error = match tokenizer.encode_with_cancellation("hello hello", || {
        polls += 1;
        TokenizationControl::Cancel
    }) {
        Ok(_) => return Err(io::Error::other("tokenization should cancel").into()),
        Err(error) => error,
    };

    assert_eq!(error.to_string(), "tokenization cancelled");
    assert_eq!(polls, 1);
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

#[test]
fn bpe_reports_incomplete_utf8_for_partial_byte_token() -> Result<(), Box<dyn Error>> {
    let bytes = byte_bpe_tokenizer_fixture();
    let file = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&file)?;

    let error = match tokenizer.decode(&[13]) {
        Ok(_) => return Err(io::Error::other("partial UTF-8 token should not decode").into()),
        Err(error) => error,
    };
    assert!(error.is_incomplete_utf8());
    assert_eq!(tokenizer.decode_if_complete(&[13])?, None);
    assert_eq!(
        tokenizer.decode_if_complete(&[13, 14])?,
        Some("é".to_owned())
    );
    Ok(())
}
