# OpenAI Completion Seed

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible legacy completions endpoint now accepts `seed` as a
deterministic no-op when the value is a JSON int64.

The current local generation path is deterministic under Ferrite's supported
non-sampling request shape. Accepting `seed` improves compatibility with
OpenAI-style clients that send deterministic request parameters while keeping
sampling and backend-fingerprint semantics explicit: Ferrite still returns
`system_fingerprint: null`.

Malformed seed values such as strings, floats, and objects remain unsupported
and produce an OpenAI-shaped `invalid_request_error`.

## Red

The new fixture-backed route test first failed because `seed` was treated as an
unknown completion field:

```sh
cargo test -p ferrite-server completions_endpoint_accepts_seed -- --nocapture
```

Expected failure before implementation:

```text
unsupported completion field(s): seed
```

## Green

Implementation kept seed validation isolated in
`crates/ferrite-server/src/openai/schema/seed.rs` and wired it only into the
legacy completions request schema.

Focused checks:

```sh
cargo test -p ferrite-server completions_endpoint_accepts_seed -- --nocapture
cargo test -p ferrite-server completion_endpoint_rejects_malformed_seed -- --nocapture
cargo test -p ferrite-server openai::schema::seed -- --nocapture
```

## Interpretation

This slice does not implement stochastic sampling, seed-dependent random
number generation, or a Ferrite backend fingerprint. It only prevents a valid
OpenAI completions request parameter from blocking Ferrite's current
deterministic local text-generation subset.
