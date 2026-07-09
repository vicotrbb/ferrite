# Rejected theory: decode Q4_K / Q6_K blocks once per engine batch

- Date: 2026-07-09
- Status: rejected; experimental Rust changes were discarded
- Model: Qwen2.5-0.5B-Instruct Q4_K_M eval artifact
- Host: Apple M5 Pro, 10 Ferrite inference threads, desktop load present

## Hypothesis

The Q4_K and Q6_K batched matvec kernels still called their single-stream
block-dot routine once per stream. Decoding each quantized block once and
reusing its widened NEON lanes across up to eight activation streams should
reduce repeated integer unpacking and improve batch-8 aggregate throughput.

## Experiment

An experimental implementation kept one NEON accumulator per stream, replayed
the exact single-stream FMA order, and reused decoded quant lanes. Dedicated
unit tests proved every stream's result bit-identical to the single-stream
block-dot path. `cargo test -p ferrite-inference --release` and Clippy with
warnings denied both passed before measurement.

The canonical command was:

```sh
scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --generate-tokens 64 \
  --benchmark-runs 64 \
  --batch-streams 4 \
  --batch-streams 8 \
  --skip-server \
  --tag q4k-q6k-decode-once-preliminary
```

## Result

| artifact | batch 4 | batch 8 | stream-0 parity |
| --- | ---: | ---: | --- |
| `2026-07-09-202640` retained implementation | 72.77 tok/s | 87.70 tok/s | not yet emitted by that harness revision |
| `2026-07-09-203254` decode-once experiment | 71.86 tok/s | 87.73 tok/s | exact |

The batch-8 delta was +0.03 tok/s (+0.034%), far below run-to-run noise, and
batch 4 was slightly lower. Reusing decoded lanes increased live-register
pressure and loop complexity without measurable throughput benefit. The Rust
changes were therefore discarded; the eval artifact remains as negative
evidence so this approach is not repeated without a materially different
kernel layout.
