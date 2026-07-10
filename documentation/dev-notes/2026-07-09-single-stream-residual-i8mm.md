# Single-stream residual-I8MM milestone

Date: 2026-07-09

## Outcome

The opt-in activation-matvec path now exceeds the 100 tok/s single-stream
target on the Apple M5 Pro reference machine:

| gate | precise decode | streamed decode | workers |
| --- | ---: | ---: | ---: |
| original baseline (`2026-07-09-191148`) | 31.99 tok/s | 30.90 tok/s | 15 |
| pushed exact stack (`2026-07-09-205246`) | 65.47 tok/s | 65.94 tok/s | 10 |
| current exact gate (`2026-07-09-234613`) | 74.56 tok/s | 74.21 tok/s | 10 |
| residual-I8MM gate (`2026-07-09-235617`) | **106.21 tok/s** | **103.65 tok/s** | 7 |

The accepted fast result is 232.0% above the original 31.99 tok/s baseline,
62.2% above the previously pushed 65.47 tok/s exact stack, and 42.5% above
the current exact path. The result was produced by `scripts/eval.sh`; the JSON
record includes the dirty-tree state, model, host, commands, policy, worker
count, memory, and CPU measurements.

The final combined artifact (`2026-07-09-235740`) independently confirms
103.11 tok/s precise and 106.42 tok/s streamed while also running batch and
HTTP gates.

## Implementation

- Q5_0 and Q8_0 use two 32-value residual-Q8 activation passes.
- Q4_K and Q6_K use two residual-Q8_K passes per 256-value block.
- On Arm FEAT_I8MM, `SMMLA` evaluates two weight rows against both residual
  passes together. Runtime feature detection preserves portable fallback
  dispatch.
- Shared gate/up traversal quantizes the common activation once; Q/K/V can
  share their residual activation when at least two projections use Q5_0.
- NEON quantizers preserve the scalar signed-scale and round-away behavior,
  covered by direct scalar-equality tests.
- The Q8_0 output projection has a fused parallel argmax, avoiding a full
  logits allocation for token-id-only decode.
- The bandwidth-bound pool retains explicit CLI/environment overrides and
  selects seven workers on the 10-worker M5 performance level.
- Attention now reuses one in-place softmax buffer per layer. KV values are
  validated once when inserted instead of once per cached position and query
  head. The optimized attention test is bit-identical to the previous
  reference operation order.
- SwiGLU reuses its gate allocation in both single and batched decode.

## Fidelity gates

- The 128-token benchmark trace hash matches exact mode:
  `f3da26c985011419cf16ce95522e8a951e39e29448c890263c40de05fa7dc30b`.
- Six additional prompts match exact mode for 64 generated tokens each:
  `hello world`, `The capital of France is`, `Once upon a time`,
  `Rust is a systems programming language`, `Machine learning models can`,
  and `The recipe calls for`.
- This is 512 compared generated tokens with no token-id divergence.
- A real-model comparison profile covered 169 candidate matrix calls across
  Q4_K, Q5_0, Q6_K, and Q8_0 with zero per-matrix argmax mismatches. The final
  Q8_0 output projection had maximum absolute drift `0.000607`; its exact and
  candidate top-logit margins were `4.153185` and `4.153200`.
- The current exact gate improved from 65.47 to 74.56 tok/s, so the shared
  attention/allocation work does not trade away default performance.

## User-facing policy

The route remains explicit:

```sh
scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --experimental-residual-q8-activation-matvec
```

The same flag is accepted by `ferrite` and `ferrite-server`. It is deliberately
not combined with experimental continuous batching yet: batched kernels retain
the exact arithmetic contract, and the server rejects that mixed configuration
instead of silently switching policies. Because residual activation
quantization is approximate even though all current output gates match, the
default path remains exact until model-family coverage is broader.
