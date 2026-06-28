# 2026-06-28 Token ID Profile API

## Context

Tier 1 throughput work uses token-id-only decode loops to avoid materializing
logits during repeated generation and benchmarks. Ferrite already exposed
`accept_token_profiled` for the logits-returning path, and the CLI could print
next-token profile events, but the token-id-only path did not have a direct
profiling API.

That left the benchmark-oriented decode path harder to inspect without falling
back to logits materialization.

## Change

Added `ScalarLlamaSession::accept_token_id_profiled`, returning a
`ProfiledTokenId` with the selected token id and profile events.

The implementation keeps the token-id-only output path on
`Matrix::argmax_mul_vec` and profiles that optimized argmax operation as the
`output` event. It does not force full logits materialization.

## Validation

Red check:

```text
cargo test -p ferrite-inference --test scalar_profile -- --nocapture
error[E0599]: no method named `accept_token_id_profiled` found for struct `ScalarLlamaSession<'a>` in the current scope
```

Green check:

```text
cargo test -p ferrite-inference --test scalar_profile -- --nocapture
test token_id_only_profile_records_output_argmax_matrix ... ok
```

Broader gates run for this slice:

```text
cargo fmt --all -- --check
git diff --check
cargo test -p ferrite-inference --test scalar_session_cache -- --nocapture
cargo test -p ferrite-inference --test scalar_reference -- --nocapture
cargo test -p ferrite-inference --test matvec_kernel_check -- --nocapture
cargo test -p ferrite-inference quantized_tests -- --nocapture
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
cargo clippy -p ferrite-inference --all-targets -- -D warnings
```
