# 2026-06-28 Tier 1 Qwen2 Second Prompt Reference

## Scope

This slice expands Qwen2 Tier 1 correctness evidence beyond the original
`hello world` prompt. It records a second deterministic local `llama.cpp`
reference comparison for:

- Qwen2.5-0.5B-Instruct Q4_K_M
- Qwen2.5-1.5B-Instruct Q4_K_M

This is correctness evidence only. It does not prove broad Qwen2 behavior,
throughput, additional quantizations, or AVX2 runtime correctness.

## Prompt

```text
The capital of France is
```

`llama.cpp` tokenized the prompt identically for both local GGUF artifacts:

```text
[785, 6722, 315, 9625, 374]
```

## Qwen2.5-0.5B-Instruct Q4_K_M

Reference command:

```sh
target/reference/llama.cpp/build/bin/llama-completion -m target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf -p 'The capital of France is' -n 3 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Reference output:

```text
 Paris. It
```

Reference continuation tokenization:

```text
[12095, 13, 1084]
```

Ferrite gate:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'The capital of France is' --generate-tokens 3 --expect-generated-token-ids 12095,13,1084
```

Output:

```text
prompt_token_ids=785,6722,315,9625,374
next_token_id=12095
generated_token_ids=12095,13,1084
generated_text= Paris. It
expected_generated_token_ids=12095,13,1084
generated_match=true
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=196608
```

## Qwen2.5-1.5B-Instruct Q4_K_M

Reference command:

```sh
target/reference/llama.cpp/build/bin/llama-completion -m target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf -p 'The capital of France is' -n 3 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Reference output:

```text
 Paris. The
```

Reference continuation tokenization:

```text
[12095, 13, 576]
```

Ferrite gate:

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'The capital of France is' --generate-tokens 3 --expect-generated-token-ids 12095,13,576
```

Output:

```text
prompt_token_ids=785,6722,315,9625,374
next_token_id=12095
generated_token_ids=12095,13,576
generated_text= Paris. The
expected_generated_token_ids=12095,13,576
generated_match=true
model_file_bytes=1117320736
scalar_weight_bytes=1111370240
kv_cache_bytes=458752
```

## Result

Ferrite matched local deterministic `llama.cpp` reference continuations for a
second Qwen2 prompt on both Tier 1 Qwen2 Q4_K_M models. Qwen2 coverage remains
partial: two fixed prompts are useful regression evidence, not a broad prompt
suite or a full Tier 1 completion gate.
