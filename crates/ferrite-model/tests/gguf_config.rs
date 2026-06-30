mod support;

use ferrite_model::gguf::{parse_gguf, ModelArchitecture, ModelConfig};
use std::error::Error;
use std::io;
use support::gguf::{
    minimal_llama_gguf, minimal_llama_gguf_with_attention_head_count,
    minimal_llama_gguf_with_attention_head_count_kv, minimal_llama_gguf_with_attention_heads,
    minimal_llama_gguf_with_block_count, minimal_llama_gguf_with_context_length,
    minimal_llama_gguf_with_embedding_length, minimal_llama_gguf_with_feed_forward_length,
    minimal_llama_gguf_with_key_length, minimal_llama_gguf_with_rope_dimension_count,
    minimal_llama_gguf_with_value_length, minimal_qwen2_gguf,
};

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
fn derives_architecture_aware_llama_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf();
    let file = parse_gguf(&bytes)?;

    let ModelConfig::Llama(config) = file.model_config()? else {
        return Err(io::Error::other("expected llama config").into());
    };

    assert_eq!(config.architecture, ModelArchitecture::Llama);
    assert_eq!(config.context_length, 2048);
    assert_eq!(config.embedding_length, 8);
    assert_eq!(config.attention_head_count, 2);
    assert_eq!(config.attention_head_count_kv, 1);
    assert_eq!(config.key_length, 4);
    assert_eq!(config.value_length, 4);
    assert_eq!(config.rope_dimension_count, 4);
    assert_eq!(config.gqa_ratio(), Some(2));
    Ok(())
}

#[test]
fn rejects_zero_context_length_in_model_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_context_length(0);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other("zero context length should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("llama.context_length must be greater than zero"));
    Ok(())
}

#[test]
fn rejects_zero_embedding_length_in_model_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_embedding_length(0);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other("zero embedding length should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("llama.embedding_length must be greater than zero"));
    Ok(())
}

#[test]
fn rejects_zero_block_count_in_model_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_block_count(0);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other("zero block count should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("llama.block_count must be greater than zero"));
    Ok(())
}

#[test]
fn rejects_zero_feed_forward_length_in_model_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_feed_forward_length(0);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other("zero feed-forward length should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("llama.feed_forward_length must be greater than zero"));
    Ok(())
}

#[test]
fn rejects_zero_attention_key_length_in_model_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_key_length(0);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other("zero attention key length should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("llama.attention.key_length must be greater than zero"));
    Ok(())
}

#[test]
fn rejects_zero_attention_value_length_in_model_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_value_length(0);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other("zero attention value length should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("llama.attention.value_length must be greater than zero"));
    Ok(())
}

#[test]
fn rejects_zero_rope_dimension_count_in_model_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_rope_dimension_count(0);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other("zero rope dimension count should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("llama.rope.dimension_count must be greater than zero"));
    Ok(())
}

#[test]
fn rejects_rope_dimension_count_larger_than_key_length() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_rope_dimension_count(5);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other(
                "rope dimension count larger than key length should be rejected",
            )
            .into());
        }
        Err(error) => error,
    };

    assert!(error.to_string().contains(
        "llama.rope.dimension_count 5 must be less than or equal to llama.attention.key_length 4"
    ));
    Ok(())
}

#[test]
fn rejects_zero_attention_head_count_in_model_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_attention_head_count(0);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other("zero attention head count should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("llama.attention.head_count must be greater than zero"));
    Ok(())
}

#[test]
fn rejects_zero_attention_kv_head_count_in_model_config() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_attention_head_count_kv(0);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other("zero KV attention head count should be rejected").into());
        }
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("llama.attention.head_count_kv must be greater than zero"));
    Ok(())
}

#[test]
fn rejects_kv_head_count_that_does_not_divide_attention_heads() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_llama_gguf_with_attention_heads(3, 2);
    let file = parse_gguf(&bytes)?;

    let error = match file.llama_config() {
        Ok(_) => {
            return Err(io::Error::other(
                "non-divisible KV attention head count should be rejected",
            )
            .into());
        }
        Err(error) => error,
    };

    assert!(error.to_string().contains(
        "llama.attention.head_count 3 must be divisible by llama.attention.head_count_kv 2"
    ));
    Ok(())
}

#[test]
fn derives_qwen2_config_from_qwen2_metadata() -> Result<(), Box<dyn Error>> {
    let bytes = minimal_qwen2_gguf();
    let file = parse_gguf(&bytes)?;

    let ModelConfig::Qwen2(config) = file.model_config()? else {
        return Err(io::Error::other("expected qwen2 config").into());
    };

    assert_eq!(config.architecture, ModelArchitecture::Qwen2);
    assert_eq!(config.context_length, 32768);
    assert_eq!(config.embedding_length, 896);
    assert_eq!(config.block_count, 24);
    assert_eq!(config.feed_forward_length, 4864);
    assert_eq!(config.attention_head_count, 14);
    assert_eq!(config.attention_head_count_kv, 2);
    assert_eq!(config.key_length, 64);
    assert_eq!(config.value_length, 64);
    assert_eq!(config.rope_dimension_count, 64);
    assert_eq!(config.rope_freq_base, Some(1_000_000.0));
    assert_eq!(config.attention_layer_norm_rms_epsilon, Some(0.000001));
    assert_eq!(config.gqa_ratio(), Some(7));
    Ok(())
}
