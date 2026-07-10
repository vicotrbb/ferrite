# OSS Quality Hardening Performance Gate

Date: 2026-07-10

## Scope

Validate that the OSS quality and documentation hardening does not regress the
Ferrite CPU inference path. Runtime edits in this slice are semantic no-ops:
explicit unsafe blocks, documented invariants, allowance reasons, and ignored
error bindings. No measured speedup is attributed to those edits.

## Fixed Inputs

- Host: Apple M5 Pro, 15 cores, 24 GiB RAM
- Toolchain for rebuilt source: Rust 1.96.1
- Model: `target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf`
- Model SHA-256:
  `74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db`
- Prompt: `Write a short story about a rusty robot who learns to sail.`
- Inference workers: 7
- Optimized policy: `experimental_residual_i8mm`

## Paired Release Measurement

The pre-change release binary was built on 2026-07-09 and had SHA-256
`4bc24440b507068521e6d7ff5334c45a14dd28f916df7c8703635f05d8ffc827`.
The rebuilt binary had SHA-256
`fc43b2f12c40ecbcae46d236b4b6ec5708036fb04f866dfa50d2908bec0e728d`.

Each binary ran the same 128-token benchmark five times:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --prompt 'Write a short story about a rusty robot who learns to sail.' \
  --benchmark-runs 128 \
  --experimental-residual-q8-activation-matvec
```

| Binary | Median ns/token | Median tok/s | Token trace |
| --- | ---: | ---: | --- |
| Pre-change | 9,938,302 | 100.6208 | reference |
| Rebuilt | 9,792,499 | 102.1190 | exact match |

The rebuilt median latency is 1.4671% lower and throughput is 1.4889% higher,
which is accepted only as evidence that no regression was observed. The builds
also differ by the Rust 1.96.0 to 1.96.1 patch update and dependency advisory
fix, so this small improvement is not assigned to a source optimization. All
five runs for both binaries produced the same prompt IDs, next token ID, and
complete 128-token benchmark trace.

## Rejected Allocation Experiment

The audit found that rotary position encoding allocated one output vector per
attention head. A candidate changed this to copy the combined projection once
and transform each head in place. It passed focused bit-exact tests and retained
the complete 128-token trace, but failed the performance gate under the same
Rust 1.96.1 build and benchmark command:

| Implementation | Median ns/token | Median tok/s |
| --- | ---: | ---: |
| Existing per-head output | 9,792,499 | 102.1190 |
| Candidate in-place output | 10,218,690 | 97.8599 |

Candidate latency regressed 4.3522% and throughput regressed 4.1707%. The
candidate was reverted. This result demonstrates that allocation count alone
is not a sufficient optimization signal for Ferrite's decode path.

## Long-Context Policy Parity

A 512-token run compared the default exact path with residual I8MM:

| Policy | ns/token | tok/s | Token count | Token SHA-256 |
| --- | ---: | ---: | ---: | --- |
| Default | 14,818,788 | 67.4819 | 512 | `8e3c91f225df00e8292d089ad63a231201b52064ed3486943782eb49d34d5bb3` |
| Residual I8MM | 12,572,863 | 79.5364 | 512 | `8e3c91f225df00e8292d089ad63a231201b52064ed3486943782eb49d34d5bb3` |

The token traces match exactly. Residual I8MM is 17.8633% faster by throughput
for this longer cached sequence.

## Full Evaluation

Command:

```sh
RUSTUP_TOOLCHAIN=1.96.1 scripts/eval.sh \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --experimental-residual-q8-activation-matvec \
  --batch-streams 2 \
  --batch-streams 4 \
  --batch-streams 8 \
  --server-batch-streams 4 \
  --requests 4 \
  --tag oss-quality-hardening
```

Result: `scripts/evals/2026-07-10-143009-qwen2.5-0.5b-instruct-q4_k_m.{json,md}`

| Path | Result |
| --- | ---: |
| Precise single-stream decode | 103.27 tok/s |
| Streamed single-stream decode | 106.14 tok/s |
| Batch 2 aggregate | 108.51 tok/s, parity true |
| Batch 4 aggregate | 134.46 tok/s, parity true |
| Batch 8 aggregate | 159.75 tok/s, parity true |
| Continuous HTTP batch 4 | 91.11 tok/s, parity true |

Relative to the accepted 2026-07-09 `235740` evidence, precise single-stream
throughput increased 0.16%, engine batches increased 5.78%, 4.93%, and 7.01%,
and continuous HTTP batch throughput increased 4.17%. No measured throughput
path regressed.

A later repetition was rejected before handoff because two unrelated Node.js
builds overlapped it. Ferrite's generation CPU mean fell from 552% to 228%, and
all CLI and server paths slowed together while parity remained true. The
contaminated outputs were removed rather than presented as comparable evidence.
No production inference source changed after the accepted run. Subsequent Rust
changes were limited to test-only lint metadata and standalone help dispatch.

## Acceptance

Accepted. The full eval reports `ok`, every available parity check passes, the
512-token exact and optimized traces are identical, and the paired median shows
no performance regression. The one isolated candidate that regressed was
removed from the final source.
