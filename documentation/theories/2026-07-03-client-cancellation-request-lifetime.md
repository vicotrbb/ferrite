# Theory: Client Cancellation Request Lifetime

Date: 2026-07-03

Status: Candidate, narrowed by direct-route test

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

What is not proven:

- whether Ferrite failed to observe disconnect;
- whether the request would have stopped naturally after a short delay;
- whether Kubernetes port-forward buffering hid the real client state;
- whether the observed CPU belonged to the killed request or another request;
- whether cancellation behavior differs before first token versus after first
  streamed token.

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

The next experiment should use a real model or a deliberately slow fixture path
that disconnects before first generated content, so it can distinguish prompt
prefill cancellation from post-token stream cancellation.

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
- per-request elapsed time and final state.

## Decision Rule

Do not optimize or rewrite cancellation logic from this observation alone.
First, build a focused cancellation gate that proves whether generation stops
when the client disconnects. If the gate reproduces continued generation after
disconnect, fix cancellation before relying on long benchmark automation for
large 1024-token and concurrency runs.
