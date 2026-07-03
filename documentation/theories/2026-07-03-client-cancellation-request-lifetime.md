# Theory: Client Cancellation Request Lifetime

Date: 2026-07-03

Status: Candidate, narrowed and partially mitigated by focused cancellation gates

## Hypothesis

Ferrite may continue CPU-bound generation for some time after an
OpenAI-compatible streaming client disconnects, especially when the disconnect
arrives through a Kubernetes port-forward or after a long prompt-prefill phase.

If true, this affects:

- memory and CPU efficiency under flaky clients;
- benchmark reliability when clients are killed or port-forwards reset;
- reconnect semantics for Ollama/OpenAI-compatible clients;
- server shutdown and overload behavior.

## Triggering Observation

During the x86_64 512-token paired latency/cache proof, the first
`llama-benchy` companion attempt ran through a local Kubernetes port-forward
and exceeded the native gate timing without producing JSON. The client process
was killed. Immediately afterward, the Ferrite server still showed active CPU
and elevated RSS inside the pod.

The server was restarted before the accepted in-pod benchmark so that the
accepted result started from a clean request state.

Related proof note:
`documentation/benchmarks/2026-07-03-latency-cache-x86-paired-qwen-0-5b-512.md`

## Evidence Strength

This is not yet a validated bug.

What is proven:

- A long port-forwarded client request became unusable as benchmark evidence.
- Killing the local client did not immediately make the server look idle.
- Restarting the server cleared the condition.
- The same benchmark shape completed when the client ran inside the pod.
- Direct in-process SSE body-drop cancellation releases Ferrite's inference
  permit in the route-level fixture test
  `chat_stream_releases_inference_permit_when_response_body_is_dropped`.
- Direct TCP disconnect after a generated SSE chat event releases Ferrite's
  inference permit in the live HTTP fixture test
  `live_http_server_releases_inference_permit_after_streaming_tcp_disconnect`.
- Direct TCP disconnect after the initial assistant-role SSE event but before
  generated content releases Ferrite's inference permit in the live HTTP
  fixture test
  `live_http_server_releases_inference_permit_after_tcp_disconnect_before_generated_content`.
- Ferrite now checks whether the SSE receiver is already closed after queued
  initial stream chunks and before entering generation. This avoids starting
  generation for the cheap closed-receiver case that is visible at that
  boundary.
- Ferrite now has a cooperative prompt-prefill cancellation seam. The scalar
  session checks a cancellation callback before each prompt token, and the
  OpenAI streaming path maps a closed SSE receiver to prompt cancellation.

What is not proven:

- whether Ferrite failed to observe disconnect;
- whether the request would have stopped naturally after a short delay;
- whether Kubernetes port-forward buffering hid the real client state;
- whether the observed CPU belonged to the killed request or another request;
- how quickly a real large-model prompt prefill returns to idle after
  disconnect, because cancellation is cooperative at prompt-token boundaries
  and does not interrupt a single in-progress token evaluation.

## Direct Route Evidence

Commit `7baf562` added a route-level regression test for the simplest
cancellation path:

```text
cargo test -p ferrite-server chat_stream_releases_inference_permit_when_response_body_is_dropped -- --nocapture
```

The test opens a streaming `/v1/chat/completions` response against the fixture
chat model, verifies the single inference permit is held, drops the response
body without consuming the stream, and waits for the permit to become
available again. It passed on 2026-07-03.

This weakens the broad version of the theory. The remaining risk is not
"Ferrite always ignores dropped streaming clients." The narrower theory is
that cancellation may be delayed or hidden in real TCP, port-forwarded, or
pre-first-token paths where the server has not yet attempted to send a chunk or
where transport buffering delays disconnect observation.

## Direct TCP Evidence

Commit `2c29192` added a live HTTP fixture test for post-token TCP
disconnects:

```text
cargo test -p ferrite-server releases_inference_permit -- --nocapture
```

The test starts a real local TCP listener, sends a streaming
`/v1/chat/completions` request with `max_completion_tokens=4096`, reads until a
generated SSE `delta.content` event is observed, drops the socket, and waits
for the shared inference permit to become available again. It passed on
2026-07-03.

This further narrows the remaining theory. Normal post-token TCP disconnects
release the permit in the fixture path. The still-unproven paths are:

- disconnect before the first generated stream event, especially during long
  prompt prefill;
- disconnect propagation through Kubernetes port-forward under staging control
  plane instability;
- real-model cancellation timing under long CPU-bound Qwen generation, where
  observing cancellation may depend on when the inference loop next emits text.

## Pre-Generated-Content TCP Evidence

Commit `8fc9805` added a live HTTP fixture test for a narrower pre-generated
content path:

```text
cargo test -p ferrite-server live_http_server_releases_inference_permit_after_tcp_disconnect_before_generated_content -- --nocapture
```

The test starts a real local TCP listener, sends a streaming
`/v1/chat/completions` request with `max_completion_tokens=4096`, reads only
until the initial assistant-role SSE event with empty content is observed,
asserts that no generated `delta.content` event has been received, drops the
socket, and waits for the shared inference permit to become available again.
It passed on 2026-07-03.

This proves fixture-path cancellation works after the server has started SSE
streaming but before generated content is delivered. It still does not prove
cancellation during long real-model prompt prefill, because that phase may do
substantial CPU work before any SSE event can be sent and before socket closure
is observed through a failed stream write.

## Pre-Generation Closed-Receiver Guard

Commit `8706e43` exposed `StreamSender::is_closed()` and checks that state
after initial SSE chunks are queued but before the stream worker locks the
inference engine and enters generation.

Validation:

```text
cargo test -p ferrite-server stream_sender_reports_when_receiver_is_closed -- --nocapture
cargo test -p ferrite-server releases_inference_permit -- --nocapture
```

This is a small defensive improvement, not a full cancellation architecture.
It can skip generation when the SSE receiver has already closed at the
pre-generation boundary.

## Cooperative Prompt-Prefill Cancellation

Commits `0bbd2ec` and `39b1d74` added the next cancellation seam:

- `ScalarLlamaSession::accept_prompt_with_control` checks a callback before
  each prompt token is evaluated.
- `InferenceEngine::generate_with_prompt_callback_and_cache_options` exposes
  that seam at the server runtime layer.
- The OpenAI streaming worker maps `StreamSender::is_closed()` to
  `PromptEvaluationControl::Cancel` during prompt evaluation.

Validation:

```text
cargo test -p ferrite-inference --test scalar_prompt_cancellation -- --nocapture
cargo test -p ferrite-server generate_with_prompt_callback_cancels_before_next_prompt_token -- --nocapture
cargo test -p ferrite-server prompt_control_cancels_when_stream_receiver_is_closed -- --nocapture
cargo test -p ferrite-server releases_inference_permit -- --nocapture
cargo fmt -- --check
git diff --check
```

This improves the design from "HTTP-layer cancellation only" to cooperative
runtime cancellation during prompt prefill. It still cannot preempt a single
matrix-heavy token evaluation mid-token. For real models, the worst-case
cancellation latency is therefore bounded by the time to finish the current
prompt token plus the transport delay before the SSE receiver is marked
closed.

## Minimal Experiment

Use a small, repeatable model/server setup and avoid Kubernetes port-forward for
the first pass.

1. Start `ferrite-server` with `--experimental-prefix-cache`, a known API key,
   and a real GGUF model.
2. Start one streaming `/v1/chat/completions` request with a long enough
   `max_tokens` value to keep generation active for at least 30 seconds.
3. Record server PID, RSS, CPU, and active connection state before the request.
4. Disconnect the client deliberately:
   - once before first generated content;
   - once after the first generated content event.
5. Continue sampling server CPU, RSS, and connection state every second for at
   least 30 seconds after disconnect.
6. Repeat through Kubernetes port-forward only after the direct local or
   in-pod path is understood.

The next experiment should use a real model and a long prompt that disconnects
after prompt evaluation starts but before generated content is delivered. It
should measure how many seconds elapse between client disconnect and permit
release with the cooperative prompt-prefill cancellation seam in place.

If that experiment still shows unacceptable post-disconnect CPU, the next
design candidate is lower-level cancellation inside a single token evaluation,
most likely at layer or matvec boundaries. That is a higher-risk change and
should not be attempted without real-model evidence that token-boundary
cancellation is insufficient.

## Expected Outcomes

The cancellation path is healthy if server CPU returns to idle promptly after a
client disconnect and no additional generated chunks are attempted for that
request.

The theory strengthens if CPU remains active for the cancelled request after
the client connection is closed, especially if RSS continues to grow or the
next request is delayed.

The theory weakens if direct local/in-pod disconnects cancel promptly and the
only reproducible failure is Kubernetes port-forward stream loss.

## Instrumentation Needed

The next implementation-quality test should add request-lifetime evidence that
does not depend on external inference from `top`:

- request id in server logs;
- stream start and stream end reason;
- client disconnect detection point;
- generation cancellation signal observed by the inference worker;
- tokens generated after disconnect;
- prompt tokens evaluated after disconnect;
- per-request elapsed time and final state.

## Decision Rule

Do not optimize or rewrite cancellation logic from this observation alone.
First, build a focused cancellation gate that proves whether generation stops
when the client disconnects. If the gate reproduces continued generation after
disconnect, fix cancellation before relying on long benchmark automation for
large 1024-token and concurrency runs.
