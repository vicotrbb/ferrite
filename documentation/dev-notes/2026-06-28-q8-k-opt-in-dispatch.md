# Q8_K Opt-In Dispatch Slice

Date: 2026-06-28

## Scope

This slice adds an explicit opt-in route for the internal Q4_K/Q6_K x Q8_K
activation matvec path.

It adds:

- `ScalarExecutionOptions`;
- Q4_K and Q6_K opt-in backend selection tests;
- option-aware `Matrix` matvec methods;
- option-aware scalar sessions;
- `--experimental-q8-k-activation-matvec` in the CLI;
- a CLI Q4_K fixture test that exercises the flag.

Default dispatch remains unchanged. Calling `Matrix::mul_vec`,
`ScalarLlamaModel::start_session`, or the CLI without the experimental flag uses
the prior Q4_K/Q6_K dispatch order.

## Red-Green Evidence

The backend-selection tests started red because the requested option surface and
backend variants did not exist:

```text
no `q4_k_mul_vec_with_options` in `scalar::q4_k`
no `q6_k_mul_vec_with_options` in `scalar::q6_k`
no `ScalarExecutionOptions` in `scalar`
variant `Aarch64NeonQ8K` not found
```

The propagation tests then started red because `Matrix` and sessions did not
yet accept execution options:

```text
no method named `mul_vec_with_options` found for struct `Matrix`
no method named `start_session_with_options` found for struct `ScalarLlamaModel`
```

The CLI test started red because the experimental flag was unknown:

```text
unknown argument --experimental-q8-k-activation-matvec
```

Each surface was implemented after its corresponding failing test.

## Verification

Focused checks passed after the implementation slices:

```sh
cargo fmt --all -- --check
git diff --check
cargo test -p ferrite-inference q8_k_backend_when_enabled -- --nocapture
cargo test -p ferrite-inference q4_k_matvec_uses_neon_backend_on_aarch64 -- --nocapture
cargo test -p ferrite-inference q6_k_matvec_uses_neon_backend_on_aarch64 -- --nocapture
cargo test -p ferrite-inference q4_k -- --nocapture
cargo test -p ferrite-inference q6_k -- --nocapture
cargo test -p ferrite-inference q8_k_execution_options -- --nocapture
cargo test -p ferrite-inference q8_k_session_options -- --nocapture
cargo test -p ferrite-cli cli_enables_experimental_q8_k_activation_matvec -- --nocapture
cargo test -p ferrite-cli
cargo clippy -p ferrite-inference --all-targets -- -D warnings
cargo clippy -p ferrite-cli --all-targets -- -D warnings
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
```

The focused output included:

```text
q4_k_matvec_uses_q8_k_backend_when_enabled_on_aarch64 ... ok
q6_k_matvec_uses_q8_k_backend_when_enabled_on_aarch64 ... ok
q4_k_fixture_accepts_q8_k_session_options ... ok
q6_k_fixture_accepts_q8_k_session_options ... ok
cli_enables_experimental_q8_k_activation_matvec ... ok
```

## Current Limitations

- The route is opt-in only.
- Q6_K argmax remains on the existing path.
- No real Tier 1 model-output parity claim is made by this slice.
- No throughput claim is made by this slice.
