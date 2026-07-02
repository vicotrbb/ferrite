# Prefix Cache Key Identity

Date: 2026-07-02

## Slice

This slice aligns `PrefixCacheKey` with ADR 0009's fail-closed cache identity
requirements. It still does not implement cache lookup, K/V state retention,
server wiring, or generation behavior changes.

## Implementation

- Added `PrefixCacheFingerprints` to group:
  - model fingerprint;
  - tokenizer fingerprint;
  - chat-template fingerprint;
  - execution-policy fingerprint;
  - request-shape fingerprint.
- Updated `PrefixCacheKey` to combine those fingerprints with exact
  token-prefix identity.
- Added optional namespace support for future use of OpenAI
  `prompt_cache_key`-style metadata.
- Kept namespace separate from token-prefix equality. A namespace mismatch
  changes the key, but a namespace match alone never proves prompt equivalence.

## Red Test

The first focused test run failed before implementation with missing API and
constructor-shape errors:

```text
error[E0432]: unresolved import `ferrite_inference::prefix_cache::PrefixCacheFingerprints`
error[E0599]: no method named `with_namespace` found for struct `PrefixCacheKey`
```

## Validation

Focused check:

```sh
cargo test -p ferrite-inference --test token_prefix_cache -- --nocapture
```

Result: 3 passed.

Package checks:

```sh
cargo test -p ferrite-inference --tests
cargo fmt --all -- --check
git diff --check
```

Results:

- `cargo test -p ferrite-inference --tests`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

## Limits

The fingerprints are caller-provided strings in this slice. Future server and
runtime slices still need to derive stable model, tokenizer, template,
execution-policy, and request-shape fingerprints from real request state before
cache lookup can be attempted.
