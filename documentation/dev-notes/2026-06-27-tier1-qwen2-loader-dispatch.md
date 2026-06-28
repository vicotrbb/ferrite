# 2026-06-27 Tier 1 Qwen2 Loader Dispatch

## Scope

This slice routes parsed Qwen2 GGUF configs through the existing scalar
transformer loader.

It proves that the current scalar runtime can load and execute a real
Qwen2.5-0.5B-Instruct Q4_K_M GGUF artifact. It does not prove Qwen2
correctness, because the deterministic reference-token check still mismatches
`llama.cpp`.

## Implementation

- Changed the scalar GGUF loader to consume `GgufFile::model_config()`.
- Accepted both `ModelConfig::Llama` and `ModelConfig::Qwen2` into the existing
  scalar transformer config shape.
- Kept tensor-name expectations unchanged.
- Added a synthetic Qwen2 loader-dispatch test by rewriting the existing tiny
  Llama fixture metadata from `llama` to `qwen2`, preserving the GGUF string
  lengths and tensor payload.

## Validation

Commands:

```sh
cargo test -p ferrite-inference --test scalar_reference loads_scalar_qwen2_reference_weights_from_f32_gguf_fixture -- --nocapture
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture
cargo clippy -p ferrite-inference --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check
```

All commands passed.

## Real Qwen2.5 Probe

Token-ID prompt command:

```sh
cargo run --release -p ferrite-cli -- --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt-token-ids 1 --top-logits 3
```

Output:

```text
prompt_token_ids=1
next_token_id=220
next_token=Ġ
top_logits=220:11.429098,608:11.233154,2038:11.055692
model_file_bytes=397808192
model_file_retained_bytes=0
scalar_weight_bytes=391859712
kv_cache_bytes=24576
```

Text prompt command:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 3
```

Output:

```text
prompt_token_ids=14990,1879
next_token_id=2857
next_token=.js
generated_cached_tokens=5
generated_token_ids=2857,25,16
generated_text=.js:1
model_file_bytes=397808192
model_file_retained_bytes=0
scalar_weight_bytes=391859712
kv_cache_bytes=122880
```

`llama.cpp` prompt-token command:

```sh
printf 'hello world' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[14990, 1879]
```

`llama.cpp` deterministic continuation command:

```sh
target/reference/llama.cpp/build/bin/llama-completion -m target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf -p 'hello world' -n 3 --temp 0 --top-k 1 --top-p 1 --repeat-last-n 0 --no-conversation --no-jinja --no-display-prompt --verbosity 1
```

Output:

```text
Hello,
```

The exact continuation tokenizes to:

```sh
printf '\nHello,' | target/reference/llama.cpp/build/bin/llama-tokenize -m target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --stdin --ids --no-bos --no-escape --log-disable
```

Output:

```text
[198, 9707, 11]
```

Explicit Ferrite expectation command:

```sh
target/release/ferrite --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 3 --expect-generated-token-ids 198,9707,11
```

Output:

```text
generated_token_ids=2857,25,16
expected_generated_token_ids=198,9707,11
generated_match=false
generated token ids 2857,25,16 did not match expected token ids 198,9707,11
```

## Result

Ferrite now gets past the earlier `expected llama architecture, found qwen2`
runtime boundary and can execute the real Tier 1 Qwen2.5-0.5B-Instruct Q4_K_M
artifact.

Qwen2 correctness is not proven. The next Qwen2 slice should diagnose the
reference-token mismatch before making any model-family support claim.
