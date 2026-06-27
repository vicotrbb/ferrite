# 2026-06-27 CLI Generated Token Gate

## Scope

This slice adds a CLI correctness gate for multi-token generation:

```sh
--expect-generated-token-ids <id[,id...]>
```

The gate requires `--generate-tokens`, prints the expected sequence, prints
`generated_match=<bool>`, and exits with an error when generated token IDs do
not match. This turns Tier 0 multi-token reference probes into command-level
checks instead of manual output inspection.

## Implementation

- Added parser support for `--expect-generated-token-ids`.
- Reused the existing comma-separated token ID parser.
- Rejected expected generated-token IDs without `--generate-tokens`.
- Compared generated IDs immediately after generation output.
- Added CLI integration tests for success, mismatch failure, and invalid mode
  combinations.

## Evidence

Red test:

```sh
cargo test -p ferrite-cli --test next_token_cli
```

Failed because `--expect-generated-token-ids` was unknown.

Green verification:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
rg -n "TODO|TBD|expect\(|unwrap\(|panic!|unsafe" Cargo.toml crates
```

The hygiene scan only reported the existing workspace lint setting:

```text
Cargo.toml:16:unsafe_code = "forbid"
```

Real Tier 0 135M gate:

```sh
target/release/ferrite --model target/models/SmolLM2-135M-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 30 --expect-generated-token-ids 30,198,198,57,5248,597
```

Relevant output:

```text
generated_token_ids=30,198,198,57,5248,597
expected_generated_token_ids=30,198,198,57,5248,597
generated_match=true
match=true
```

Real Tier 0 360M gate:

```sh
target/release/ferrite --model target/models/SmolLM2-360M-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 18 --expect-generated-token-ids 18,284,476,28120,905,18
```

Relevant output:

```text
generated_token_ids=18,284,476,28120,905,18
expected_generated_token_ids=18,284,476,28120,905,18
generated_match=true
match=true
```

## Boundaries

This is a deterministic greedy-output gate only. It does not add sampling,
stop-token handling, chat-template rendering, or tolerance-based logit
comparison.
