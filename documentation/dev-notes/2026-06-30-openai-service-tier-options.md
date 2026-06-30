# OpenAI Service Tier Options

## Scope

Ferrite's OpenAI-compatible chat completion endpoint now accepts the documented
OpenAI `service_tier` request labels:

- `auto`
- `default`
- `flex`
- `scale`
- `priority`

Ferrite still runs a local single-tier service. When any supported
`service_tier` is explicitly requested, the response reports the actual local
tier as `default`.

Source reference:

- <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create/>

## TDD Evidence

RED command:

```sh
cargo test -p ferrite-server --lib chat_endpoint_accepts_openai_service_tier_options_as_local_default -- --nocapture
```

Observed failure:

```text
assertion `left == right` failed: {"error":{"code":null,"message":"unsupported chat completion field(s): service_tier","param":"service_tier","type":"invalid_request_error"}}
  left: 400
 right: 200
test openai::service_tier_tests::chat_endpoint_accepts_openai_service_tier_options_as_local_default ... FAILED
```

GREEN commands:

```sh
cargo test -p ferrite-server --lib chat_endpoint_accepts_openai_service_tier_options_as_local_default -- --nocapture
cargo test -p ferrite-server --lib openai::schema::service_tier -- --nocapture
cargo test -p ferrite-server --lib chat_endpoint_rejects_unknown_service_tier -- --nocapture
```

Observed result:

```text
test openai::service_tier_tests::chat_endpoint_accepts_openai_service_tier_options_as_local_default ... ok
test openai::schema::service_tier::tests::openai_service_tiers_resolve_to_local_default ... ok
test openai::service_tier_tests::chat_endpoint_rejects_unknown_service_tier ... ok
```

## Boundary

This is request-shape compatibility only. It does not add hosted-service
priority scheduling, flex capacity, scale credits, or multi-tier local
scheduling. Unknown tier strings still return an OpenAI-shaped invalid request.
