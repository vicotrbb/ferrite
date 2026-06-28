# 2026-06-28 Tier 1 SmolLM2 1.7B Second Prompt Reference

## Scope

This slice expands Tier 1 SmolLM2-1.7B-Instruct Q4_K_M output evidence beyond
the original `hello world` prompt. It compares Ferrite against a fixed local
`llama.cpp` deterministic reference for a second prompt.

It is correctness evidence only. It does not change runtime code, prove broad
prompt coverage, prove AVX2 runtime correctness, or prove Tier 1 throughput.

## Model

- Repo: `bartowski/SmolLM2-1.7B-Instruct-GGUF`
- File: `SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Local file size from `ls -lh`: 1.0G

## Prompt

Prompt:

```text
The capital of France is
```

Reference tokenization command:

```sh
printf 'The capital of France is' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[504, 3575, 282, 4649, 314]
```

Ferrite reported the same prompt token IDs in the expectation gate below.

## llama.cpp Reference

Deterministic reference command:

```sh
target/reference/llama.cpp/build/bin/llama-completion -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf -p 'The capital of France is' -n 6 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Output:

```text
 Paris. [end of text]
```

The visible continuation plus EOS tokenizes to Ferrite's expected token IDs:

```sh
printf ' Paris.<|im_end|>' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[7042, 30, 2]
```

## Ferrite Check

Command:

```sh
target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'The capital of France is' --generate-tokens 6 --expect-generated-token-ids 7042,30,2
```

Output:

```text
prompt_token_ids=504,3575,282,4649,314
next_token_id=7042
next_token=ĠParis
generated_cached_tokens=7
generated_token_ids=7042,30,2
generated_stopped_on_eos=true
generated_text= Paris.<|im_end|>
expected_generated_token_ids=7042,30,2
generated_match=true
model_file_bytes=1055609824
model_file_retained_bytes=0
scalar_weight_bytes=1053827072
kv_cache_bytes=2752512
```

## Result

Ferrite matched the fixed local `llama.cpp` deterministic reference for a
second SmolLM2-1.7B-Instruct Q4_K_M prompt. After the CLI EOS-stop slice, the
same check can request six generated tokens and still stop at the three-token
reference continuation. SmolLM2 Tier 1 output coverage now has two fixed prompt
profiles, but remains partial and should not be treated as broad prompt
coverage.
