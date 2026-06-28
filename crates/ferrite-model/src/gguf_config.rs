#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelArchitecture {
    Llama,
    Qwen2,
}

impl ModelArchitecture {
    pub(crate) fn from_metadata(value: &str) -> Option<Self> {
        match value {
            "llama" => Some(Self::Llama),
            "qwen2" => Some(Self::Qwen2),
            _ => None,
        }
    }

    pub(crate) fn metadata_value(self) -> &'static str {
        match self {
            Self::Llama => "llama",
            Self::Qwen2 => "qwen2",
        }
    }

    pub(crate) fn metadata_prefix(self) -> &'static str {
        self.metadata_value()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModelConfig {
    Llama(TransformerConfig),
    Qwen2(TransformerConfig),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransformerConfig {
    pub architecture: ModelArchitecture,
    pub context_length: u64,
    pub embedding_length: u64,
    pub block_count: u64,
    pub feed_forward_length: u64,
    pub attention_head_count: u64,
    pub attention_head_count_kv: u64,
    pub key_length: u64,
    pub value_length: u64,
    pub attention_layer_norm_rms_epsilon: Option<f32>,
    pub rope_dimension_count: u64,
    pub rope_freq_base: Option<f32>,
}

pub type LlamaConfig = TransformerConfig;

impl TransformerConfig {
    pub fn gqa_ratio(&self) -> Option<u64> {
        if self.attention_head_count_kv == 0 {
            return None;
        }

        if self
            .attention_head_count
            .is_multiple_of(self.attention_head_count_kv)
        {
            Some(self.attention_head_count / self.attention_head_count_kv)
        } else {
            None
        }
    }
}
