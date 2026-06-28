# Token-ID Generation Path

Date: 2026-06-27

## Scope

`--generate-tokens` now uses a token-id-only session helper for repeated
generated tokens. This avoids returning logits that the generation loop does not
inspect after the first prompt next-token result.

Normal `accept_token`, `--top-logits`, and `--profile-next-token` paths still
return logits where needed.

## Change

Commit `f12b452` added `ScalarLlamaSession::generate_token_ids` and changed the
CLI generation loop to call it. Streaming output is preserved by printing each
returned token ID in order.

## TDD Evidence

Red:

```text
error[E0599]: no method named `generate_token_ids` found for struct `ScalarLlamaSession<'a>`
```

Focused green checks:

```sh
cargo test -p ferrite-inference scalar_session_generates_token_ids_without_returning_logits -- --nocapture
cargo test -p ferrite-cli cli_generates_token_ids_and_decoded_text -- --nocapture
cargo test -p ferrite-cli cli_streams_generated_token_chunks -- --nocapture
```

All focused checks passed.

## Verification

Full verification before commit:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p ferrite-inference --target x86_64-unknown-linux-gnu --tests
git diff --check
```

All commands passed.

## Real Model Evidence

Command:

```sh
/usr/bin/time -l target/release/ferrite --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf --prompt 'hello world' --generate-tokens 6 --expect-token-id 18 --expect-generated-token-ids 18,198,3725,198,198,788
```

Output included:

```text
generated_token_ids=18,198,3725,198,198,788
generated_match=true
expected_token_id=18
match=true
        3.35 real         4.69 user         2.08 sys
          1598685184  maximum resident set size
          2123666624  peak memory footprint
```

The retained benchmark path still emitted the documented token sequence. This
slice does not make a new benchmark throughput claim.

