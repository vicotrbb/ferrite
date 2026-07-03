use crate::diagnostic_hash::{fnv64_bytes, format_fnv64};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LongChatTextIdentity {
    byte_len: usize,
    hash: u64,
}

impl LongChatTextIdentity {
    pub fn from_text(text: &str) -> Self {
        Self {
            byte_len: text.len(),
            hash: fnv64_bytes(text.as_bytes()),
        }
    }

    pub fn byte_len(self) -> usize {
        self.byte_len
    }

    pub fn hash(self) -> u64 {
        self.hash
    }

    pub fn formatted_hash(self) -> String {
        format_fnv64(self.hash)
    }
}
