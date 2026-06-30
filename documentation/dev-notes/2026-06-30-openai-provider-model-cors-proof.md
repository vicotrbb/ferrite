# OpenAI Provider Model CORS Proof

## Summary

Ferrite now has focused regression coverage that `OPTIONS` preflight requests
work for OpenAI-compatible model retrieval paths whose model IDs contain
provider-style slashes.

The covered paths are:

- `/v1/models/Qwen/Qwen2.5-0.5B-Instruct-Q4_K_M`
- `/v1/models/Qwen%2FQwen2.5-0.5B-Instruct-Q4_K_M`

This matters for browser-based OpenAI-compatible clients that preflight model
catalog calls before sending an authenticated `GET /v1/models/{model}` request.

## Result

No production code change was required. The existing `/v1/models/*model`
route and explicit OpenAI preflight handler already return:

- `204 No Content`;
- `access-control-allow-origin: *`;
- `GET` in `access-control-allow-methods`;
- `authorization` in `access-control-allow-headers`.

## Validation

```text
cargo test -p ferrite-server --lib openai_model_retrieve_preflight_supports_provider_style_model_ids -- --nocapture
```

Result: passed.
