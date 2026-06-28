# 2026-06-27 Tier 1 Qwen2 0.5B Probe

## Scope

This slice records a Tier 1 model-breadth probe for Qwen2.5-0.5B-Instruct
Q4_K_M.

It is an evidence slice only. It does not change Ferrite runtime code and does
not claim Qwen2 support.

## Model

- Repo: `bartowski/Qwen2.5-0.5B-Instruct-GGUF`
- File: `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Local file size from `ls -lh`: 379M
- Hugging Face repo revision recorded by local cache:
  `41ba88dbac95fed2528c92514c131d73eb5a174b`
- Hugging Face blob id recorded by local cache:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Quantization: Q4_K_M GGUF mixture

Download command:

```sh
huggingface-cli download bartowski/Qwen2.5-0.5B-Instruct-GGUF Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --local-dir target/models --max-workers 1
```

Download output:

```text
Download complete. Moving file to target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf
target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf
```

## Metadata Shape

The current GGUF reader and scalar loader are Llama-specific:

- `GgufFile::llama_config()` accepts only `general.architecture = llama`.
- The config reader expects `llama.*` metadata keys.
- The scalar loader is named and shaped around `ScalarLlamaModel`.

The downloaded Qwen2.5-0.5B artifact exposes Qwen2 metadata keys and attention
bias tensors:

```sh
strings target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf | rg -n "^(general\\.architecture|qwen2\\.|llama\\.|token_embd|output|blk\\.0\\.)" | head -120
```

Relevant output:

```text
general.architecture
qwen2.block_count
qwen2.context_length
qwen2.embedding_length
qwen2.feed_forward_length
qwen2.attention.head_count
qwen2.attention.head_count_kv
qwen2.rope.freq_base
qwen2.attention.layer_norm_rms_epsilon
token_embd.weight
blk.0.attn_norm.weight
blk.0.ffn_down.weight
blk.0.ffn_gate.weight
blk.0.ffn_up.weight
blk.0.ffn_norm.weight
blk.0.attn_k.bias
blk.0.attn_k.weight
blk.0.attn_output.weight
blk.0.attn_q.bias
blk.0.attn_q.weight
blk.0.attn_v.bias
blk.0.attn_v.weight
output_norm.weight
```

## Ferrite Probe

Prompt-token-ID command:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt-token-ids 1
```

Output:

```text
expected llama architecture, found qwen2
```

Text-prompt command:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 3
```

Output:

```text
expected llama architecture, found qwen2
```

## Result

Ferrite does not currently load the Tier 1 Qwen2.5-0.5B-Instruct Q4_K_M GGUF
artifact because the runtime has an explicit Llama-only architecture boundary.

This is useful Tier 1 evidence: the 7:1 GQA model-breadth gap is now blocked on
first-class Qwen2 architecture support rather than model availability. A future
implementation slice should add an architecture-aware config model, explicit
Qwen2 metadata handling, bias-tensor handling or validation, and deterministic
reference-output parity before claiming Qwen2 support.
