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
- Ferrite now also polls the same cancellation signal while evaluating a prompt
  token, at transformer-layer boundaries. This reduces the expected
  post-disconnect CPU window for long prompt prefill without inserting checks
  into every matvec kernel.
- A bounded x86_64 real-model Qwen 0.5B smoke closed a long-prompt streaming
  request before generated content and immediately completed a reconnect
  request with generated content. The reconnect generated event arrived after
  about 8.9 seconds, consistent with the abandoned request not holding the
  single inference permit for the full long-prompt prefill.
- Ferrite now emits one server-side OpenAI streaming lifecycle log line per
  request. The line includes a request id, finish reason, disconnect point,
  prompt tokens started, prompt cancellation polls, generated chunks,
  generated token ids, and elapsed milliseconds. This gives the next real-model
  proof a direct server-side request-lifetime signal instead of relying only on
  reconnect timing and RSS samples.

What is not proven:

- whether Ferrite failed to observe disconnect;
- whether the request would have stopped naturally after a short delay;
- whether Kubernetes port-forward buffering hid the real client state;
- whether the observed CPU belonged to the killed request or another request;
- how quickly a real large-model prompt prefill returns to idle after
  disconnect, because cancellation is cooperative at layer boundaries and does
  not interrupt a single in-progress layer or matvec operation.
- the exact real-model server-side cancellation latency, because the existing
  Qwen 0.5B smoke was run before lifecycle logging existed and therefore used
  external reconnect timing rather than request-lifetime counters.

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

Commit `fb5221c` tightened that seam further:

- `ScalarLlamaSession::accept_prompt_with_control_and_cancellation` combines
  prompt-token context with a no-context cancellation poll.
- `ScalarLlamaSession::accept_token_with_layer_control` checks cancellation
  before each transformer layer.
- The OpenAI streaming worker now uses the cancellation-poll path, so a closed
  stream can stop long prompt prefill before the next layer starts.

Additional validation:

```text
cargo test -p ferrite-inference --test scalar_prompt_cancellation -- --nocapture
cargo test -p ferrite-server generate_with_prompt_cancellation_poll_stops_during_prompt_token_evaluation -- --nocapture
```

This improves the design from "HTTP-layer cancellation only" to cooperative
runtime cancellation during prompt prefill. It still cannot preempt a single
matrix-heavy layer or matvec operation. For real models, the worst-case
cancellation latency is therefore bounded by the time to finish the current
layer plus the transport delay before the SSE receiver is marked closed.

## Real-Model Prefill Disconnect Smoke

Benchmark note:
`documentation/benchmarks/2026-07-03-prefill-cancel-qwen-0-5b.md`

Raw JSON:
`documentation/benchmarks/2026-07-03-prefill-cancel-qwen-0-5b.json`

The 2026-07-03 staging smoke used Qwen2.5-0.5B Q4_K_M in a bounded amd64 pod.
The client sent a streaming chat request with a 140398-character prompt, read
the initial assistant-role SSE event, waited 500 ms, and closed the socket
before any generated content arrived.

The reconnect request started 0.173 ms after closing the first socket. It
returned `HTTP/1.1 200 OK`, emitted generated content after 8904.287 ms, and
finished after 9206.984 ms. Server RSS stayed bounded, from 413924 KiB before
the abort request to 428900 KiB immediately after abort and 426060 KiB after
reconnect.

This weakens the remaining theory for the direct in-pod path: the abandoned
long-prompt request did not appear to hold the single inference permit for the
full long prefill. It does not close the theory for Kubernetes port-forward or
exact server-side cancellation timing, because the smoke did not instrument the
precise disconnect observation point or the number of prompt layers evaluated
after disconnect.

## Local Lifecycle Prefill Cancel Probe

Benchmark note:
`documentation/benchmarks/2026-07-03-local-qwen-0-5b-prefill-cancel-lifecycle.md`

The next direct local probe reran the long-prompt prefill disconnect shape after
`openai_stream_lifecycle` logging existed. The client sent a 155399-character
streaming chat prompt, read the initial assistant-role SSE event, waited about
510 ms, then closed the TCP socket before any generated content arrived. A
short reconnect request started about 3.9 ms later.

Client-side result:

- reconnect returned `HTTP/1.1 200 OK`;
- reconnect generated content after `6387.344` ms;
- reconnect completed after `6421.889` ms.

Server-side lifecycle result:

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=1 prompt_cancellation_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=6419
openai_stream_lifecycle request_id=stream-1 finish_reason=completed disconnect_point=none prompt_tokens_started=8 prompt_cancellation_polls=200 generated_chunks=1 generated_token_ids=1 elapsed_ms=516
```

This narrows the theory again. Ferrite did observe the disconnect, cancelled at
the prompt-evaluation boundary, and did not generate chunks for the abandoned
request. But the cancelled request still occupied the single inference permit
for about 6.4 seconds from request start. The short reconnect request itself
took only 516 ms once it ran, so most client-observed reconnect latency came
from waiting for the cancelled prompt-evaluation path to release the permit.

Commit `fe043a3` documented this proof. The next implementation theory should
not jump straight to matvec-level cancellation. First add finer
prompt-evaluation lifecycle counters:

- prompt token index at cancellation;
- cancellation polls before and after the stream is observed closed;
- current transformer layer when cancellation is observed;
- elapsed time from first observed stream closure to cancellation return.

The first follow-up instrumentation slice added
`prompt_cancellation_closed_polls` and `disconnect_to_finish_ms` to
`openai_stream_lifecycle`; see
`documentation/dev-notes/2026-07-03-openai-stream-lifecycle-cancel-latency.md`.

The counter-enabled real-model rerun is documented in
`documentation/benchmarks/2026-07-03-local-qwen-0-5b-prefill-cancel-lifecycle-counters.md`.
It reported:

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=1 prompt_cancellation_polls=1 prompt_cancellation_closed_polls=1 generated_chunks=0 generated_token_ids=0 elapsed_ms=6455 disconnect_to_finish_ms=0
```

That result shifts the immediate question. Ferrite returned from cancellation
immediately enough to round to `disconnect_to_finish_ms=0` once the closed
stream was observed. The remaining latency is before closure observation, so
the next missing counter is `disconnect_observed_elapsed_ms`, followed by prompt
token index and transformer layer index.

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

The direct in-pod smoke is positive enough that the next code slice should not
jump to lower-level cancellation inside a single layer. Commit `b0ec7d0` added
request-lifetime instrumentation first: request id, finish reason, disconnect
observation point, prompt-token starts, prompt cancellation polls, generated
chunk count, generated token-id count, and elapsed milliseconds. Only consider
attention, FFN, or matvec-level cancellation if those direct counters show
layer-boundary cancellation is still too slow.

Example log shape:

```text
openai_stream_lifecycle request_id=stream-0 finish_reason=cancelled disconnect_point=prompt_evaluation prompt_tokens_started=12 prompt_cancellation_polls=37 generated_chunks=0 generated_token_ids=0 elapsed_ms=1234
```

## Expected Outcomes

The cancellation path is healthy if server CPU returns to idle promptly after a
client disconnect and no additional generated chunks are attempted for that
request.

The theory strengthens if CPU remains active for the cancelled request after
the client connection is closed, especially if RSS continues to grow or the
next request is delayed.

The theory weakens if direct local/in-pod disconnects cancel promptly and the
only reproducible failure is Kubernetes port-forward stream loss.

## Instrumentation Added And Remaining

Commit `b0ec7d0` adds request-lifetime evidence that does not depend on
external inference from `top`:

- request id in server logs;
- stream finish reason;
- client disconnect detection point;
- prompt cancellation polls observed by the inference worker;
- generated chunk and token-id totals;
- prompt-token-start totals;
- per-request elapsed time and final state.

The remaining proof work is to rerun the real-model gate and preserve these
log lines beside the RSS, reconnect, and latency-per-token samples. If that run
shows a high elapsed time after a prompt-evaluation disconnect, add more
granular counters around prompt layers or add an explicit after-disconnect
token counter. If it shows bounded elapsed time and no generated chunks for a
pre-generation disconnect, do not widen cancellation into matvec-level checks
yet.

## Decision Rule

Do not optimize or rewrite cancellation logic from this observation alone.
First, build a focused cancellation gate that proves whether generation stops
when the client disconnects. If the gate reproduces continued generation after
disconnect, fix cancellation before relying on long benchmark automation for
large 1024-token and concurrency runs.
