# OpenAI Completions Prompt Cache Key

Date: 2026-07-03

## Slice

Extend Ferrite's OpenAI-compatible legacy `/v1/completions` endpoint so it can
use the same `prompt_cache_key` cache namespace mechanism already available on
`/v1/chat/completions`.

This keeps the local OpenAI-compatible HTTP surface closer to the required
service milestone in `documentation/adr/0008-openai-compatible-http-api.md`:
common local clients can target both completion surfaces while Ferrite still
owns the inference runtime and cache behavior.

## Red Tests

```sh
cargo test -p ferrite-server openai::routes_tests::completions_endpoint_reports_cached_tokens_when_experimental_prefix_cache_is_enabled -- --nocapture
```

Initial failure:

```text
unsupported completion field(s): prompt_cache_key
```

```sh
cargo test -p ferrite-server openai::route_streaming_tests::completions_endpoint_stream_reports_cached_tokens_when_experimental_prefix_cache_is_enabled -- --nocapture
```

After fixing an initial test-ordering issue around the single inference permit,
the intended failure was:

```text
assertion `left == right` failed
  left: 0
 right: 1
```

The second streamed completion did not report cached prompt tokens because the
streaming completion path was still using default cache options.

## Implementation

- `CompletionRequest` now accepts `prompt_cache_key` when it is a string or
  `null`, matching the existing chat request validation helper.
- `/v1/completions` maps that key into `GenerationCacheOptions`.
- The route enables the namespace only when the server prefix-cache feature is
  enabled.
- Non-streaming and streaming completion generation now both receive the same
  completion cache options.
- `ferrite-openai-throughput --endpoint completions` can now serialize
  `--prompt-cache-key KEY` into the legacy completion request body. The
  `--prompt-cache-trace` flag remains chat-only because completion requests do
  not carry the `metadata.ferrite_cache_trace` extension.

## Validation

```sh
cargo test -p ferrite-server completions_endpoint_ -- --nocapture
```

Result:

```text
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 374 filtered out
```

The broader filter also walked the integration test binaries with no failures.

```sh
cargo test -p ferrite-server throughput_client::tests -- --nocapture
```

Result:

```text
test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 348 filtered out
```

## Boundaries

This is not a new cache algorithm and it is not a real-model benchmark. It only
closes the endpoint compatibility gap for the existing prompt-cache namespace
mechanism on the local OpenAI-compatible completion surface and proof client.
