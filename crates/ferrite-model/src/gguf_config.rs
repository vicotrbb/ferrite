#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A model architecture supported by Ferrite's GGUF configuration parser.
pub enum ModelArchitecture {
    /// The Llama transformer family.
    Llama,
    /// The Qwen2 transformer family, including Qwen2.5 models that use this
    /// GGUF architecture identifier.
    Qwen2,
    /// The Microsoft Phi-3 dense decoder family.
    Phi3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// How an architecture stores its attention projection weights in GGUF.
pub enum AttentionProjectionLayout {
    /// Query, key, and value use separate matrices.
    Separate,
    /// Query, key, and value are consecutive row ranges in one matrix.
    FusedQkv,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// How an architecture stores its gated feed-forward input projections.
pub enum FeedForwardProjectionLayout {
    /// Gate and up projections use separate matrices.
    Separate,
    /// Gate and up projections are consecutive row ranges in one matrix.
    FusedGateUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Pairing used by the architecture's rotary position encoding.
pub enum RotaryPairing {
    /// Pair adjacent coordinates after GGUF conversion.
    Adjacent,
    /// Pair matching coordinates from the two halves of the rotary span.
    SplitHalf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Stable execution-facing description of architecture-specific GGUF layout.
pub struct ArchitectureExecution {
    /// Attention projection storage layout.
    pub attention: AttentionProjectionLayout,
    /// Feed-forward projection storage layout.
    pub feed_forward: FeedForwardProjectionLayout,
    /// Rotary coordinate pairing.
    pub rotary_pairing: RotaryPairing,
}

impl ModelArchitecture {
    pub(crate) fn from_metadata(value: &str) -> Option<Self> {
        match value {
            "llama" => Some(Self::Llama),
            "qwen2" => Some(Self::Qwen2),
            "phi3" => Some(Self::Phi3),
            _ => None,
        }
    }

    pub(crate) fn metadata_value(self) -> &'static str {
        match self {
            Self::Llama => "llama",
            Self::Qwen2 => "qwen2",
            Self::Phi3 => "phi3",
        }
    }

    pub(crate) fn metadata_prefix(self) -> &'static str {
        self.metadata_value()
    }

    /// Returns the normalized execution layout for this architecture.
    ///
    /// Loader adapters use this boundary to turn architecture-specific GGUF
    /// tensors into the common transformer weights consumed by inference.
    pub fn execution(self) -> ArchitectureExecution {
        match self {
            Self::Llama => ArchitectureExecution {
                attention: AttentionProjectionLayout::Separate,
                feed_forward: FeedForwardProjectionLayout::Separate,
                rotary_pairing: RotaryPairing::Adjacent,
            },
            Self::Qwen2 => ArchitectureExecution {
                attention: AttentionProjectionLayout::Separate,
                feed_forward: FeedForwardProjectionLayout::Separate,
                rotary_pairing: RotaryPairing::SplitHalf,
            },
            Self::Phi3 => ArchitectureExecution {
                attention: AttentionProjectionLayout::FusedQkv,
                feed_forward: FeedForwardProjectionLayout::FusedGateUp,
                rotary_pairing: RotaryPairing::SplitHalf,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Validated transformer configuration selected by architecture.
pub enum ModelConfig {
    /// A Llama-family model configuration.
    Llama(TransformerConfig),
    /// A Qwen2-family model configuration.
    Qwen2(TransformerConfig),
    /// A Phi-3-family model configuration.
    Phi3(TransformerConfig),
}

impl ModelConfig {
    /// Returns the architecture-independent transformer configuration.
    pub fn transformer(&self) -> &TransformerConfig {
        match self {
            Self::Llama(config) | Self::Qwen2(config) | Self::Phi3(config) => config,
        }
    }

    /// Consumes the architecture wrapper and returns its transformer config.
    pub fn into_transformer(self) -> TransformerConfig {
        match self {
            Self::Llama(config) | Self::Qwen2(config) | Self::Phi3(config) => config,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Architecture-independent transformer dimensions read from GGUF metadata.
pub struct TransformerConfig {
    /// The model architecture that determined the metadata prefix and defaults.
    pub architecture: ModelArchitecture,
    /// The maximum model context length in tokens.
    pub context_length: u64,
    /// The hidden-state width.
    pub embedding_length: u64,
    /// The number of transformer blocks.
    pub block_count: u64,
    /// The feed-forward network width.
    pub feed_forward_length: u64,
    /// The number of query attention heads.
    pub attention_head_count: u64,
    /// The number of key and value attention heads.
    pub attention_head_count_kv: u64,
    /// The key dimension for each attention head.
    pub key_length: u64,
    /// The value dimension for each attention head.
    pub value_length: u64,
    /// The RMS normalization epsilon, when the GGUF metadata specifies one.
    pub attention_layer_norm_rms_epsilon: Option<f32>,
    /// The number of key dimensions transformed by rotary position encoding.
    pub rope_dimension_count: u64,
    /// The rotary position encoding frequency base, when explicitly specified.
    pub rope_freq_base: Option<f32>,
}

/// Backward-compatible name for a Llama transformer configuration.
pub type LlamaConfig = TransformerConfig;

impl TransformerConfig {
    /// Returns the grouped-query attention ratio when the head layout is valid.
    ///
    /// A result of `None` means the key/value head count is zero or does not
    /// divide the query head count evenly.
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
