# 2026-06-28 OpenAI Real Tier 1 Qwen2.5 1.5B Q8_0 Bounded-Wait Proof

## Scope

This slice extends Ferrite's OpenAI-compatible real Tier 1 HTTP evidence for
Qwen2.5-1.5B-Instruct Q8_0 by proving the configured bounded-wait path on an
overlapping request pair.

This does not prove general server throughput, queue fairness, multiple
concurrent successful generations, or long-stream concurrency behavior.

## Change

- Added ignored integration test
  `live_http_server_waits_for_concurrent_qwen_1_5b_q8_request` in
  `crates/ferrite-server/tests/openai_real_tier1_qwen_1_5b_http.rs`.
- The test starts a real Qwen2.5-1.5B Q8_0 live HTTP server with
  `with_inference_wait_timeout(Duration::from_secs(180))`.
- The first request is a one-token streaming chat completion.
- The second request is an overlapping one-token legacy completion sent after a
  50 ms delay.

## Verification

Command:

```sh
cargo test -p ferrite-server --test openai_real_tier1_qwen_1_5b_http live_http_server_waits_for_concurrent_qwen_1_5b_q8_request -- --ignored --nocapture
```

Result:

```text
running 1 test
test live_http_server_waits_for_concurrent_qwen_1_5b_q8_request has been running for over 60 seconds
test live_http_server_waits_for_concurrent_qwen_1_5b_q8_request ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 108.79s
```

Additional checks:

```sh
cargo fmt --all -- --check
git diff --check
```

Both passed before committing the test slice.

## Failed Shape

The first attempted Qwen2.5-1.5B Q8_0 overlap test reused the 0.5B proof shape
with a 16-token first streaming request. That was too long for the debug
integration-test path and exhausted the configured 180 second bounded wait.

Observed failure:

```text
HTTP/1.1 429 Too Many Requests
{"error":{"message":"inference request queue is full; retry later","type":"rate_limit_error","param":null,"code":null}}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 188.36s
```

That failure confirms the bounded wait expires correctly when the in-flight
request does not finish in time. The retained proof uses a one-token first
stream to prove successful waiting on the larger model without converting the
test into a long-stream throughput benchmark.

## Interpretation

Ferrite can now prove a successful overlapping request on the real
Qwen2.5-1.5B Q8_0 OpenAI-compatible HTTP path when operators configure a
bounded wait window. This is stronger than the earlier one-token single-client
latency proof and the default backpressure proof, but it is still not a
throughput claim.

General concurrent serving remains unproven because the runtime still enforces
a single inference permit. Queue fairness, multiple waiting clients,
long-running stream overlap, and production throughput need separate evidence.
