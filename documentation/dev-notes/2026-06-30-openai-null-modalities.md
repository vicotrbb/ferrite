# OpenAI Null Modalities

## Scope

Ferrite's OpenAI-compatible chat completion endpoint now treats
`"modalities": null` as equivalent to omitting `modalities`.

The endpoint still only supports text output. Non-text or malformed modalities
remain unsupported.

## TDD Evidence

RED command:

```sh
cargo test -p ferrite-server --lib null_modalities_are_text_only -- --nocapture
```

Observed failure:

```text
assertion failed: is_text_only_modalities(&Some(Value::Null))
test openai::schema::modalities::tests::null_modalities_are_text_only ... FAILED
```

GREEN commands:

```sh
cargo test -p ferrite-server --lib null_modalities_are_text_only -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_accepts_null_modalities -- --nocapture
```

Observed result:

```text
test openai::schema::modalities::tests::null_modalities_are_text_only ... ok
test openai::chat_option_tests::chat_endpoint_accepts_null_modalities ... ok
```

## Boundary

This is a request-shape compatibility slice only. It does not add audio output,
multi-modal output, or additional model-serving behavior.
