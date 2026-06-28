# 2026-06-27 Tier 1 SmolLM2 1.7B Reference Probe

## Scope

This slice records Ferrite's first real Tier 1 model output probe against a
fixed local `llama.cpp` reference profile.

It is an evidence slice only. It does not change Ferrite runtime code and does
not prove the Tier 1 throughput target.

## Model

- Repo: `bartowski/SmolLM2-1.7B-Instruct-GGUF`
- File: `SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local file size from `ls -lh`: 1.0G
- Local size reported by Ferrite: 1,055,609,824 bytes
- Scalar weight bytes reported by Ferrite: 1,053,827,072 bytes
- Hugging Face repo revision recorded by local cache:
  `1f03464768bfcc0319fc50da8ff5fb20b6417ba2`
- Hugging Face blob id recorded by local cache:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`
- Quantization: Q4_K_M GGUF mixture

Download command:

```sh
huggingface-cli download bartowski/SmolLM2-1.7B-Instruct-GGUF SmolLM2-1.7B-Instruct-Q4_K_M.gguf --local-dir target/models --max-workers 1
```

Download output:

```text
Download complete. Moving file to target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf
target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf
```

## Prompt Tokenization

Reference command:

```sh
printf 'hello world' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[28120, 905]
```

Ferrite reported the same prompt token IDs in both probes below.

## Single-Token Ferrite Probe

Command:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --top-logits 8
```

Output:

```text
prompt_token_ids=28120,905
next_token_id=18
next_token="
top_logits=18:20.126324,6653:19.568890,21646:18.931122,1715:18.610294,1002:18.605652,17:18.602179,76:18.517265,23:18.436216
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=786432
       14.87 real        12.40 user         1.13 sys
          1754808320  maximum resident set size
          2124027072  peak memory footprint
```

## llama.cpp Reference

One-token command:

```sh
/usr/bin/time -l target/reference/llama.cpp/build/bin/llama-completion -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf -p 'hello world' -n 1 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Output:

```text
"
```

The generated text tokenizes to Ferrite's `next_token_id`:

```sh
printf '"' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[18]
```

Six-token deterministic reference command:

```sh
/usr/bin/time -l target/reference/llama.cpp/build/bin/llama-completion -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf -p 'hello world' -n 6 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Output:

````text
"
```

In
````

The exact continuation tokenizes to:

````sh
printf '"\n```\n\nIn' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
````

Output:

```text
[18, 198, 3725, 198, 198, 788]
```

## Ferrite Six-Token Gate

Command:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 18 --expect-generated-token-ids 18,198,3725,198,198,788
```

Output:

````text
prompt_token_ids=28120,905
next_token_id=18
next_token="
generated_cached_tokens=8
generated_token_ids=18,198,3725,198,198,788
generated_text="
```

In
expected_generated_token_ids=18,198,3725,198,198,788
generated_match=true
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=3145728
expected_token_id=18
match=true
       52.35 real        49.10 user         1.24 sys
          1466826752  maximum resident set size
          2123814144  peak memory footprint
````

## Result

Ferrite loaded the real Tier 1 SmolLM2-1.7B-Instruct Q4_K_M GGUF artifact and
matched the fixed local `llama.cpp` deterministic reference profile for the
prompt `hello world` over six generated tokens.

This proves one real 1.7B Llama-family output path for the current scalar
runtime. It does not prove x86_64 AVX2 runtime correctness, the Tier 1
throughput target, or broad Tier 1 model-family coverage.
