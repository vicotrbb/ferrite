# 2026-06-28 CLI EOS Generation Stop

## Scope

This slice makes CLI generated-token mode stop after emitting a tokenizer EOS
token when the GGUF metadata provides `tokenizer.ggml.eos_token_id`.

It does not change the lower-level fixed-count `ScalarLlamaSession`
`generate_token_ids` API or benchmark decode loops.

## Motivation

The second SmolLM2-1.7B reference prompt reached EOS after:

```text
 Paris.
```

Before this slice, Ferrite continued to generate post-EOS tokens when
`--generate-tokens` requested a larger count:

```text
generated_token_ids=7042,30,2,198,2,1
generated_text= Paris.<|im_end|>
<|im_end|><|im_start|>
```

The local `llama.cpp` reference stopped at EOS, so the CLI needed EOS-aware
generation behavior for user-facing generation checks.

## Implementation

- `GgufTokenizer` now parses optional `tokenizer.ggml.eos_token_id` metadata
  from `UInt32` or `UInt64` values.
- The CLI generation loop emits the EOS token, sets
  `generated_stopped_on_eos=true`, and stops before accepting that EOS token
  into the session for another decode step.
- Existing fixed-count session and benchmark APIs remain unchanged.

## Verification

Focused tests:

```sh
cargo test -p ferrite-model extracts_eos_token_id_from_gguf_metadata -- --nocapture
cargo test -p ferrite-cli cli_stops_generation_after_eos_token -- --nocapture
cargo test -p ferrite-model
cargo test -p ferrite-cli
```

All passed.

Real SmolLM2-1.7B check:

```sh
cargo build --release -p ferrite-cli
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

Ferrite CLI generated-token mode now matches the reference runtime's EOS stop
behavior for the observed SmolLM2-1.7B prompt while preserving fixed-count
session and benchmark behavior.
