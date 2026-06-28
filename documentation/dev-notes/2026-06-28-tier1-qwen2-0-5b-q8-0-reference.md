# Tier 1 Qwen2.5 0.5B Q8_0 Reference Probe

Date: 2026-06-28

## Scope

This slice expands Tier 1 model-output evidence beyond the existing Q4_K_M
quantization set. It uses `Qwen2.5-0.5B-Instruct-Q8_0.gguf` because Ferrite's
GGUF loader supports Q8_0 tensors and the 0.5B model keeps the local proof
bounded.

This is a local aarch64 proof. It does not prove x86_64 parity for Q8_0 full
model output and does not complete Tier 1.

## Artifact

```text
target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf
size: 506M
sha256: 25130a98aa782284a7dabea0c23245b2fd371ed47244e79d78b8ec23245fdf96
```

Downloaded with:

```sh
huggingface-cli download bartowski/Qwen2.5-0.5B-Instruct-GGUF \
  Qwen2.5-0.5B-Instruct-Q8_0.gguf \
  --local-dir target/models \
  --max-workers 1
```

## Reference Prompts

Reference continuations were generated with local `llama.cpp`:

```sh
target/reference/llama.cpp/build/bin/llama-completion \
  -m target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf \
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
  --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf \
  --prompt 'hello world' \
  --generate-tokens 6 \
  --expect-token-id 198 \
  --expect-generated-token-ids 198,9707,11,4337,0,2585

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf \
  --prompt 'The capital of France is' \
  --generate-tokens 6 \
  --expect-token-id 12095 \
  --expect-generated-token-ids 12095,13,1084,374,279,7772

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf \
  --prompt 'Once upon a time' \
  --generate-tokens 6 \
  --expect-token-id 11 \
  --expect-generated-token-ids 11,1052,572,264,3908,883

target/release/ferrite \
  --model target/models/Qwen2.5-0.5B-Instruct-Q8_0.gguf \
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
benchmark_avg_ns=52794900
maximum resident set size=830095360
peak memory footprint=1095356032
```

This is useful local context for the Q8_0 full-model artifact, but it is not a
full Tier 1 throughput claim. Broader throughput still needs model, prompt,
thread-count, and x86_64 evidence.
