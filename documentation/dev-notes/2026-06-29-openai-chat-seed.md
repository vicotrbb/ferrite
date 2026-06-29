# OpenAI Chat Seed

Date: 2026-06-29

## Scope

Ferrite's OpenAI-compatible chat completions endpoint now accepts `seed` as a
deterministic no-op when the value is a JSON int64.

The current local chat path is deterministic for Ferrite's supported
non-sampling request shape. Accepting `seed` keeps common OpenAI-compatible
client requests from failing when they pass deterministic-generation
parameters. Ferrite still returns `system_fingerprint: null` because it does
not expose an OpenAI backend fingerprint.

Malformed seed values such as strings, floats, and objects remain unsupported
and produce an OpenAI-shaped `invalid_request_error`.

## Red

The new fixture-backed chat route test first failed because `seed` was treated
as an unsupported chat completion field:

```sh
cargo test -p ferrite-server chat_endpoint_accepts_seed -- --nocapture
```

Expected failure before implementation:

```text
unsupported chat completion field(s): seed
```

## Green

The chat request schema now reuses the focused seed validator introduced for
legacy completions:

```sh
cargo test -p ferrite-server chat_endpoint_accepts_seed -- --nocapture
cargo test -p ferrite-server chat_endpoint_rejects_malformed_seed -- --nocapture
cargo test -p ferrite-server openai::schema::seed -- --nocapture
```

## Interpretation

This slice does not implement stochastic sampling, seed-dependent random
number generation, or a Ferrite backend fingerprint. It only accepts a valid
OpenAI chat request parameter that is harmless for Ferrite's current
deterministic local text-generation subset.
