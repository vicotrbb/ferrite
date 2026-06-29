# OpenAI Service Tier

Date: 2026-06-29

## Slice

Ferrite's OpenAI-compatible chat endpoint now accepts `service_tier: "auto"`
and `service_tier: "default"` as local no-op service-tier requests. OpenAI's
Chat Completions API documents `service_tier` as a processing-mode selector
with `auto`, `default`, `flex`, `scale`, and `priority` values. Ferrite maps
the supported local values to a response `service_tier` of `default`.

Source reference:

- https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create

## Implementation

- Added `crates/ferrite-server/src/openai/schema/service_tier.rs` to keep
  service-tier validation and response normalization separate from chat request
  logic.
- Updated chat unsupported-field detection to accept missing `service_tier`,
  `service_tier: "auto"`, or `service_tier: "default"`.
- Updated non-streaming chat responses to include `service_tier: "default"`
  when the request explicitly set a supported local service tier.
- Added a focused `service_tier_tests` module instead of adding more route
  tests to the already-large route test file.

## Red Test

```sh
cargo test -p ferrite-server chat_endpoint_accepts_auto_service_tier -- --nocapture
```

Failed before implementation with:

```text
unsupported chat completion field(s): service_tier
```

## Validation

```sh
cargo fmt --all -- --check
cargo test -p ferrite-server service_tier -- --nocapture
```

All commands passed after implementation.

## Limits

This slice does not implement paid or alternative processing tiers, priority
queues, Flex processing, Scale processing, Project-level tier configuration, or
any scheduling behavior tied to `service_tier`. Values such as `flex`, `scale`,
and `priority` remain unsupported.
