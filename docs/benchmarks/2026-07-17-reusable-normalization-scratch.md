# Reusable normalization scratch diagnostic

Date: 2026-07-17

## Scope

This diagnostic evaluates one bounded single-stream allocation change. A token
step previously created a new root-mean-square normalization vector for the
attention input and feed-forward input of every transformer layer, then created
one more for the output projection. The candidate keeps one hidden-size vector
and refills it at each of those non-overlapping stages.

For a model with `L` transformer layers, a token step now makes one
normalization-buffer allocation instead of `2L + 1`. Context-only prompt steps
avoid the final output normalization in both implementations. The arithmetic
order inside normalization is unchanged.

## Fixed inputs

- Host: Apple M5 Pro, 15 logical CPUs
- OS: macOS 26.5.2 arm64
- Toolchain: Rust 1.96.0, LLVM 22.1.2
- Source base: commit `8899f3ea61111597826ae773806073d4b9c8193f`
  on `main`, with the candidate evaluated in a dirty working tree
- Cargo profile: repository `release` profile, no `RUSTFLAGS`
- Baseline CLI SHA-256:
  `e286a8f5d7d3beb5e0660cb4928230b0a8905934a142bae39b69fef4ccc314c5`
- Candidate CLI SHA-256:
  `ab158ca67378a5b73bfad2f4dc3083769b0b8c8525f7efebb5ea66e0eae9d88c`
- Model: Qwen2.5 0.5B Instruct Q4_K_M, 491,400,032 bytes
- Model SHA-256:
  `74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db`
- Prompt: `Write a short story about a rusty robot who learns to sail.`
- Measured decode steps: 128 per run
- Repetitions: five interleaved baseline and candidate pairs
- Workers: seven
- Policy: experimental residual Q8 activation matvec on Arm I8MM

The host had unrelated interactive load. RSS and CPU were not sampled. The
timing result is therefore diagnostic, not clean-host release evidence.

## Command

Each copied release binary was run with the same working directory and model:

```sh
<baseline-or-candidate-binary> \
  --model target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --prompt 'Write a short story about a rusty robot who learns to sail.' \
  --benchmark-runs 128 \
  --threads 7 \
  --experimental-residual-q8-activation-matvec
```

The baseline and candidate were alternated for each pair. The complete
`benchmark_token_ids` line from every run was hashed separately from timing.

## Result

| Variant | Median decode step | Median decode rate | Token trace SHA-256 |
| --- | ---: | ---: | --- |
| Baseline | 10.373 ms | 96.41 tok/s | `ff342fb6b38a5f301d61dab6af424615a33dce9cc75ff1e9c026a2b40aa3674a` |
| Reusable scratch | 10.314 ms | 96.95 tok/s | `ff342fb6b38a5f301d61dab6af424615a33dce9cc75ff1e9c026a2b40aa3674a` |

The candidate median was 0.57% faster. Pair-level timing was noisy, while all
ten runs produced the same complete ordered 128-token trace. The focused unit
test also verifies that repeated normalization writes retain the same vector
allocation and capacity.

## Acceptance

Accepted as an allocation reduction with exact trace parity and no API change.
The modest timing direction supports the change but is not promoted as a
clean-host throughput milestone. CPU, RSS, time to first token, and tail
latency claims require a future full eval on a quiet host.
