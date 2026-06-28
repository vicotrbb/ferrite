# 2026-06-28 Tier 1 Third Prompt Reference

## Scope

This slice expands Tier 1 output evidence from two fixed prompts to three fixed
prompts for the largest local Tier 1 profiles currently under active coverage:

- SmolLM2-1.7B-Instruct Q4_K_M
- Qwen2.5-0.5B-Instruct Q4_K_M
- Qwen2.5-1.5B-Instruct Q4_K_M

It is correctness evidence only. It does not change runtime code, prove broad
prompt coverage, prove AVX2 runtime correctness, or prove Tier 1 throughput.

## Prompt

```text
Once upon a time
```

## SmolLM2-1.7B-Instruct Q4_K_M

Prompt tokenization:

```sh
printf 'Once upon a time' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[6403, 1980, 253, 655]
```

Reference command:

```sh
/usr/bin/time -l target/reference/llama.cpp/build/bin/llama-completion -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf -p 'Once upon a time' -n 6 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Reference output:

```text
, in a small village nestled
```

Full prompt plus continuation tokenization:

```text
[6403, 1980, 253, 655, 28, 281, 253, 1165, 6560, 32047]
```

Ferrite gate:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'Once upon a time' --generate-tokens 6 --expect-generated-token-ids 28,281,253,1165,6560,32047
```

Output:

```text
prompt_token_ids=6403,1980,253,655
next_token_id=28
generated_token_ids=28,281,253,1165,6560,32047
generated_text=, in a small village nestled
expected_generated_token_ids=28,281,253,1165,6560,32047
generated_match=true
model_file_bytes=1055609824
scalar_weight_bytes=1053827072
kv_cache_bytes=3932160
```

Memory evidence:

```text
4.52 real
1479524352 maximum resident set size
2123502720 peak memory footprint
```

## Qwen2.5-0.5B-Instruct Q4_K_M

Prompt tokenization:

```sh
printf 'Once upon a time' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[12522, 5193, 264, 882]
```

Reference command:

```sh
/usr/bin/time -l target/reference/llama.cpp/build/bin/llama-completion -m target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf -p 'Once upon a time' -n 3 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Reference output:

```text
, there was
```

Full prompt plus continuation tokenization:

```text
[12522, 5193, 264, 882, 11, 1052, 572]
```

Ferrite gate:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'Once upon a time' --generate-tokens 3 --expect-generated-token-ids 11,1052,572
```

Output:

```text
prompt_token_ids=12522,5193,264,882
next_token_id=11
generated_token_ids=11,1052,572
generated_text=, there was
expected_generated_token_ids=11,1052,572
generated_match=true
model_file_bytes=397808192
scalar_weight_bytes=391859712
kv_cache_bytes=172032
```

Memory evidence:

```text
0.69 real
828178432 maximum resident set size
828230784 peak memory footprint
```

## Qwen2.5-1.5B-Instruct Q4_K_M

Prompt tokenization:

```sh
printf 'Once upon a time' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[12522, 5193, 264, 882]
```

Reference command:

```sh
/usr/bin/time -l target/reference/llama.cpp/build/bin/llama-completion -m target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf -p 'Once upon a time' -n 3 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Reference output:

```text
, there was
```

Full prompt plus continuation tokenization:

```text
[12522, 5193, 264, 882, 11, 1052, 572]
```

Ferrite gate:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'Once upon a time' --generate-tokens 3 --expect-generated-token-ids 11,1052,572
```

Output:

```text
prompt_token_ids=12522,5193,264,882
next_token_id=11
generated_token_ids=11,1052,572
generated_text=, there was
expected_generated_token_ids=11,1052,572
generated_match=true
model_file_bytes=1117320736
scalar_weight_bytes=1111370240
kv_cache_bytes=401408
```

Memory evidence:

```text
4.75 real
2102919168 maximum resident set size
2268141184 peak memory footprint
```

## Result

Ferrite matched deterministic local `llama.cpp` continuations for the third
fixed Tier 1 prompt across SmolLM2-1.7B-Instruct Q4_K_M and both local Qwen2.5
Q4_K_M models.

Tier 1 output coverage is stronger but still partial. Three fixed prompts are
useful regression evidence, not broad prompt coverage, additional quantization
coverage, AVX2 runtime evidence, or a full Tier 1 completion gate.
