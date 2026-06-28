# 2026-06-27 Tier 1 Qwen2 1.5B Reference Probe

## Scope

This slice records a real Tier 1 Qwen2.5-1.5B-Instruct Q4_K_M reference probe.

It is an evidence slice only. It does not change Ferrite runtime code and does
not prove throughput.

## Model

- Repo: `Qwen/Qwen2.5-1.5B-Instruct-GGUF`
- File: `qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local path: `target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf`
- Local file size from `ls -lh`: 1.0G
- Local size reported by Ferrite: 1,117,320,736 bytes
- Scalar weight bytes reported by Ferrite: 1,111,370,240 bytes
- Hugging Face repo revision recorded by local cache:
  `91cad51170dc346986eccefdc2dd33a9da36ead9`
- Hugging Face blob id recorded by local cache:
  `6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e`
- Quantization: Q4_K_M GGUF mixture

Download command:

```sh
huggingface-cli download Qwen/Qwen2.5-1.5B-Instruct-GGUF qwen2.5-1.5b-instruct-q4_k_m.gguf --local-dir target/models --max-workers 1
```

Download output:

```text
Download complete. Moving file to target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf
target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf
```

## Reference Profile

Prompt-token command:

```sh
printf 'hello world' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[14990, 1879]
```

Deterministic continuation command:

```sh
target/reference/llama.cpp/build/bin/llama-completion -m target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf -p 'hello world' -n 3 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Output:

```text
Hello,
```

The exact continuation tokenizes to:

```sh
printf '\nHello,' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[198, 9707, 11]
```

## Ferrite Gate

Command:

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'hello world' --generate-tokens 3 --expect-generated-token-ids 198,9707,11
```

Output:

```text
prompt_token_ids=14990,1879
next_token_id=198
next_token=Ċ
generated_cached_tokens=5
generated_token_ids=198,9707,11
generated_text=
Hello,
expected_generated_token_ids=198,9707,11
generated_match=true
model_file_bytes=1117320736
model_file_retained_bytes=0
scalar_weight_bytes=1111370240
kv_cache_bytes=286720
```

## Result

Ferrite loaded the real Tier 1 Qwen2.5-1.5B-Instruct Q4_K_M GGUF artifact and
matched the fixed local `llama.cpp` deterministic reference profile for the
prompt `hello world` over three generated tokens.

This proves a real Qwen2 Tier 1 output path with the 1.5B model and the
head_dim=128 shape from the Tier 1 registry. It does not prove throughput,
broader prompt coverage, AVX2 runtime correctness, or Qwen2 behavior beyond
this fixed reference profile.
