# Tier 1 Qwen2.5 0.5B Q6_K Reference Probe

Date: 2026-06-28

## Scope

This slice expands Tier 1 model-output evidence beyond Q4_K_M and Q8_0 by
checking `Qwen2.5-0.5B-Instruct-Q6_K.gguf`. Ferrite supports Q6_K tensors, and
the 0.5B model keeps the proof bounded enough for local aarch64 validation.

This is a local aarch64 proof. It does not prove x86_64 parity for Q6_K full
model output and does not complete Tier 1.

## Artifact

```text
target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf
size: 482M
sha256: 32c14c29a44712c02e29d5c2605593ece92ccb7a4358f56016a42b151434c842
```

Downloaded with:

```sh
huggingface-cli download bartowski/Qwen2.5-0.5B-Instruct-GGUF \
  Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  --local-dir target/models \
  --max-workers 1
```

## Reference Prompts

Reference continuations were generated with local `llama.cpp`:

```sh
target/reference/llama.cpp/build/bin/llama-completion \
  -m target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  -p "$prompt" \
  -n 6 \
  --temp 0 \
  --top-k 1 \
  --top-p 1 \
  --repeat-last-n 0 \
  --no-conversation \
  --no-jinja \
  --no-display-prompt \
  --verbosity 1
```

| Prompt | Prompt token IDs | Reference continuation | Reference token IDs |
| --- | --- | --- | --- |
| `hello world` | `[14990,1879]` | `\nHello, World! How` | `[198,9707,11,4337,0,2585]` |
| `The capital of France is` | `[785,6722,315,9625,374]` | ` Paris. It is the largest` | `[12095,13,1084,374,279,7772]` |
| `Once upon a time` | `[12522,5193,264,882]` | `, there was a young man` | `[11,1052,572,264,3908,883]` |
| `Rust is a systems programming language` | `[49,590,374,264,5942,15473,4128]` | ` that is designed to be safe` | `[429,374,6188,311,387,6092]` |

## Ferrite Checks

Ferrite matched all four six-token deterministic continuations:

```sh
target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  --prompt 'hello world' \
  --generate-tokens 6 \
  --expect-token-id 198 \
  --expect-generated-token-ids 198,9707,11,4337,0,2585

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  --prompt 'The capital of France is' \
  --generate-tokens 6 \
  --expect-token-id 12095 \
  --expect-generated-token-ids 12095,13,1084,374,279,7772

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  --prompt 'Once upon a time' \
  --generate-tokens 6 \
  --expect-token-id 11 \
  --expect-generated-token-ids 11,1052,572,264,3908,883

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q6_K.gguf \
  --prompt 'Rust is a systems programming language' \
  --generate-tokens 6 \
  --expect-token-id 429 \
  --expect-generated-token-ids 429,374,6188,311,387,6092
```

All four runs reported `generated_match=true` and `match=true`.

## Benchmark Context

A bounded local benchmark on `hello world` reported:

```text
benchmark_runs=5
benchmark_avg_ns=52152800
maximum resident set size=1024950272
peak memory footprint=1044958656
```

This is useful local context for the Q6_K full-model artifact, but it is not a
full Tier 1 throughput claim. Broader throughput still needs model, prompt,
thread-count, and x86_64 evidence.
