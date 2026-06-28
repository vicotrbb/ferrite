# Tier 1 Fourth Prompt Reference

Date: 2026-06-28

## Scope

This note expands local Tier 1 model-output coverage beyond the first three
fixed prompts.

The fourth fixed prompt is:

```text
Rust is a systems programming language
```

Models checked:

- SmolLM2-1.7B-Instruct Q4_K_M
- Qwen2.5-0.5B-Instruct Q4_K_M
- Qwen2.5-1.5B-Instruct Q4_K_M

This is local aarch64 evidence against the local `llama.cpp` reference. It does
not add x86_64 prompt coverage, additional quantizations, or throughput claims.

## SmolLM2-1.7B-Instruct Q4_K_M

Prompt tokenization:

```sh
printf 'Rust is a systems programming language' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

returned:

```text
[66, 467, 314, 253, 1734, 6256, 1789]
```

Reference completion:

```sh
target/reference/llama.cpp/build/bin/llama-completion -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf -p 'Rust is a systems programming language' -n 6 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

returned:

```text
 that provides a strong memory model
```

Tokenizing that continuation returned:

```text
[338, 2433, 253, 1837, 3500, 1743]
```

Ferrite check:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'Rust is a systems programming language' --generate-tokens 6 --expect-generated-token-ids 338,2433,253,1837,3500,1743
```

passed:

```text
prompt_token_ids=66,467,314,253,1734,6256,1789
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
next_token_id=338
generated_token_ids=338,2433,253,1837,3500,1743
generated_text= that provides a strong memory model
expected_generated_token_ids=338,2433,253,1837,3500,1743
generated_match=true
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=5111808
```

## Qwen2.5-0.5B-Instruct Q4_K_M

Prompt tokenization:

```sh
printf 'Rust is a systems programming language' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

returned:

```text
[49, 590, 374, 264, 5942, 15473, 4128]
```

Reference completion:

```sh
target/reference/llama.cpp/build/bin/llama-completion -m target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf -p 'Rust is a systems programming language' -n 3 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

returned:

```text
 that is designed
```

Tokenizing that continuation returned:

```text
[429, 374, 6188]
```

Ferrite check:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'Rust is a systems programming language' --generate-tokens 3 --expect-generated-token-ids 429,374,6188
```

passed:

```text
prompt_token_ids=49,590,374,264,5942,15473,4128
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
next_token_id=429
generated_token_ids=429,374,6188
generated_text= that is designed
expected_generated_token_ids=429,374,6188
generated_match=true
model_file_bytes=397808192
model_file_retained_bytes=0
scalar_weight_bytes=391859712
kv_cache_bytes=245760
```

## Qwen2.5-1.5B-Instruct Q4_K_M

Prompt tokenization:

```sh
printf 'Rust is a systems programming language' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --stdin --ids --no-bos --no-escape --log-disable
```

returned:

```text
[49, 590, 374, 264, 5942, 15473, 4128]
```

Reference completion:

```sh
target/reference/llama.cpp/build/bin/llama-completion -m target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf -p 'Rust is a systems programming language' -n 3 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

returned:

```text
 that is designed
```

Tokenizing that continuation returned:

```text
[429, 374, 6188]
```

Ferrite check:

```sh
target/release/ferrite --model target/models/qwen2.5-1.5b-instruct-q4_k_m.gguf --prompt 'Rust is a systems programming language' --generate-tokens 3 --expect-generated-token-ids 429,374,6188
```

passed:

```text
prompt_token_ids=49,590,374,264,5942,15473,4128
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
next_token_id=429
generated_token_ids=429,374,6188
generated_text= that is designed
expected_generated_token_ids=429,374,6188
generated_match=true
model_file_bytes=1117320736
model_file_retained_bytes=0
scalar_weight_bytes=1111370240
kv_cache_bytes=573440
```

## Conclusion

Ferrite matched the local deterministic `llama.cpp` reference continuation for
the fourth fixed Tier 1 prompt across SmolLM2-1.7B, Qwen2.5-0.5B, and
Qwen2.5-1.5B Q4_K_M.

This expands local model-output coverage, but it is not a throughput result and
does not cover additional quantization formats.
