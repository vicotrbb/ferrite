pub(crate) fn qwen2_fixture_from_llama_fixture(mut bytes: Vec<u8>) -> Vec<u8> {
    for index in 0..bytes.len().saturating_sub(4) {
        if &bytes[index..index + 5] == b"llama" {
            bytes[index..index + 5].copy_from_slice(b"qwen2");
        }
    }
    bytes
}
