# Tier 1 OpenAI Long-Chat Gate

Date: 2026-07-01

## Purpose

This gate defines the next OpenAI-compatible proof milestone for Ferrite's Tier
1 local server path. Existing evidence covers short streaming chat shapes and
partial long-chat runs. The next milestone is a dedicated long-chat closure pass
that proves 256, 512, and 1024-token streaming responses, repeated multi-turn
conversations, RSS sampling before and after requests, latency per token,
stop/EOS behavior, and client reconnect/error behavior as one explicit contract.

## Scope

The gate targets `POST /v1/chat/completions` with `stream: true`.

Required local Tier 1 model set:

- `Qwen2.5-0.5B-Instruct-Q4_K_M`
- `Qwen2.5-1.5B-Instruct-Q8_0`
- `Qwen2.5-1.5B-Instruct-Q6_K`
- `SmolLM2-1.7B-Instruct-Q4_K_M`

The first implementation slice may start with Qwen2.5-0.5B Q4_K_M and then
expand through the larger Tier 1 artifacts. The gate is not closed until every
required model above has recorded evidence.

## Next Proof Milestone

The next proof milestone is not another isolated smoke run. It is a dedicated
long-chat gate closure with one reportable matrix:

- all required Tier 1 HTTP model artifacts;
- 256, 512, and 1024 completion-token streaming responses;
- at least four repeated turns per model and token length;
- generated assistant context carried from each completed turn into the next
  follow-up turn;
- RSS samples before and after each measured streaming request, plus idle
  post-run samples;
- latency per generated token, including time to first token, elapsed stream
  time, tokens per second, and min/p50/p95/max inter-token latency;
- terminal `finish_reason` and usage accounting for length, explicit stop, and
  tokenizer EOS cases;
- client disconnect, reconnect, and request-error probes that demonstrate
  bounded behavior and fresh generation after reconnect.

Any run that covers only one model, one token length, no generated follow-up
context, or only one of stop/EOS/reconnect/error behavior remains partial
evidence.

## Current Baseline Before Next Run

The 2026-07-03 active-pair BPE work improves the local Qwen2.5-0.5B
tokenization stage, but it does not close this long-chat gate by itself.

Current proof baseline:

- tokenizer-only parity was preserved for the deterministic same-size prompt:
  `tokenization_benchmark_token_count=29527` and
  `tokenization_benchmark_token_ids_fingerprint=fnv1a64:468c718e7fb1e5a0`;
- tokenizer-only average encode time improved from `6894344416 ns` to
  `4062045166 ns` on that local prompt sample;
- OpenAI server lifecycle `prompt_tokenized_elapsed_ms` improved from `8323`
  to `4331` on the local same-size prompt-stage proof;
- the proof was still a cancelled streaming request, not a completed
  256/512/1024 long-chat run.

The next accepted gate run should therefore report active-pair BPE as the
current tokenizer baseline and then measure end-to-end streaming behavior with
the full matrix below. Do not infer long-chat readiness from tokenizer-stage
improvement alone.

Use these closure flags for dedicated gate attempts:

```text
--require-models Qwen2.5-0.5B-Instruct-Q4_K_M,Qwen2.5-1.5B-Instruct-Q8_0,Qwen2.5-1.5B-Instruct-Q6_K,SmolLM2-1.7B-Instruct-Q4_K_M
--require-token-lengths 256,512,1024
--require-probes error,disconnect,queue
```

These keep partial one-model or one-length runs useful as evidence while
preventing `long_chat_summary_run_complete=true` from passing a closure attempt
that omits part of the required model set, token-length ladder, or operational
probe set. Required probes still need their matching execution flags, such as
`--error-probe`, `--disconnect-probe`, and `--queue-probe`.

Partial local required-gate proofs exist for Qwen2.5-0.5B:

- `documentation/benchmarks/2026-07-03-local-qwen-0-5b-long-chat-required-gates-256.md`
- `documentation/benchmarks/2026-07-03-local-qwen-0-5b-long-chat-required-gates-512.md`
- `documentation/benchmarks/2026-07-03-local-qwen-0-5b-long-chat-required-gates-1024.md`
- `long_chat_summary_run_complete=true`
- required model: `qwen2.5-0.5b-q4_k_m`
- required token lengths covered so far: `256`, `512`, `1024`
- required probes: `error,disconnect`

This completes the local Qwen2.5-0.5B 256/512/1024 ladder, but remains partial
evidence. It does not cover the remaining Tier 1 model artifacts, queue
behavior, or stop/EOS behavior.

An additional bounded local queue-probe slice exists:

- `documentation/benchmarks/2026-07-03-local-qwen-0-5b-queue-probe-128.md`
- required probe: `queue`
- required token length: `128`
- prompt-cache keys: `queue-a`, `queue-b`
- `long_chat_summary_queue_probe_completed=true`
- `long_chat_summary_queue_probe_contender_started_after_holder=true`

This proves the queue probe path can be required and completed locally, but it
does not replace queue coverage in the required 256/512/1024 Tier 1 matrix.

An additional bounded local stop slice exists:

- `documentation/benchmarks/2026-07-03-local-qwen-0-5b-stop-probe-64.md`
- expected finish reason: `stop`
- required token length: `64`
- `long_chat_summary_any_token_limit_hit=false`
- `long_chat_summary_run_complete=true`

This proves explicit stop behavior on a small deterministic local Qwen2.5-0.5B
slice. It does not prove tokenizer EOS behavior or replace stop/EOS coverage in
the required Tier 1 matrix.

## Required Scenarios

For each required model, run three streaming chat lengths:

- 256 completion tokens
- 512 completion tokens
- 1024 completion tokens

For each length, prove all of the following:

- HTTP status is `200`.
- Response content type is SSE-compatible.
- Stream emits at least one JSON data chunk before termination.
- Stream emits exactly one terminal `[DONE]` marker.
- Usage, when requested, reports the requested completion-token count unless
  the model ends earlier with EOS or stop.
- Terminal chunk has `finish_reason: "length"` for full-length runs or
  `finish_reason: "stop"` when stop/EOS terminates generation.
- No generated byte-level BPE fragments are dropped or reordered by streaming
  decode.

## Multi-Turn Conversation Shape

For each model, run a repeated conversation sequence using one server process:

1. User asks a short first prompt and receives a streamed response.
2. The next request includes the original user message, the assistant response,
   and a second user follow-up.
3. Repeat for at least four user turns.

The current harness carries assistant-visible generated text from each completed
scenario into the next turn for the same `(model, token_length)` pair. It
records `long_chat_result_assistant_context_source=seed|generated` and requires
`long_chat_summary_all_follow_up_turns_use_generated_context=true` for
`long_chat_summary_run_complete=true`.

The gate must record:

- prompt-token count for every turn;
- completion-token count for every turn;
- total-token count for every turn;
- whether generated assistant context is carried into every follow-up turn;
- whether any turn hits the configured completion-token limit.

The current harness reports completion-token limit status with
`long_chat_result_hit_token_limit`, plus summary fields
`long_chat_summary_all_token_limit_status_present` and
`long_chat_summary_any_token_limit_hit`.

## Memory Sampling

Each run must sample server RSS at these points:

- before model load when practical for standalone server runs;
- after `/health` reports ready;
- immediately before the long streaming request;
- after first token when the harness can observe it;
- after stream completion;
- after a two-second idle interval;
- after the final multi-turn request.

Evidence must include raw byte values and the sampling command or API used. A
local run may use `ps -o rss= -p <pid>` converted to bytes. Homelab runs must
use the `staging` Kubernetes context only and record pod CPU and memory limits.

## Latency Metrics

Each streaming run must record:

- request start timestamp;
- time to first SSE token event;
- time to final token event;
- total elapsed time through `[DONE]`;
- generated completion tokens;
- average generated tokens per second;
- per-token latency summary: min, p50, p95, max.

The initial implementation may compute latency from SSE event arrival times at
the client. More precise server-side token timing can be added later, but client
arrival timing is required for the first gate.

## Stop And EOS Behavior

For each required model, add one long-chat variant that should stop before the
requested token count:

- explicit OpenAI `stop` sequence for a known prompt; or
- tokenizer EOS if the model naturally emits EOS before the requested length.

The result must show:

- `finish_reason: "stop"`;
- `[DONE]` still emitted exactly once;
- usage reflects generated tokens before stop/EOS;
- no extra content is emitted after the stop/EOS boundary.

## Reconnect And Error Behavior

Ferrite currently does not resume partial SSE generations after a client
disconnect. The gate must make that explicit with bounded behavior:

- close one streaming client connection after at least one token event;
- verify the server releases the single inference permit;
- verify a follow-up request can complete or receive the configured bounded
  queue behavior;
- verify malformed reconnect attempts do not reuse stale generation state;
- record whether the retry starts a new generation rather than resuming.

This is a correctness and operability gate, not a promise of resumable streams.

The current disconnect harness records this with
`long_chat_disconnect_probe_reconnect_generated_event`,
`long_chat_disconnect_probe_reconnect_started_new_generation`, and
`long_chat_summary_disconnect_probe_reconnect_started_new_generation`. A
reconnect response must include generated stream content as well as `[DONE]`;
a done-only SSE response is not accepted as a completed reconnect generation.

## Required Artifacts

Each completed run must add a benchmark note under `documentation/benchmarks/`
with:

- exact commit SHA;
- model path and model id;
- host architecture and CPU;
- build mode;
- server command;
- client command;
- token length;
- prompt and multi-turn transcript shape;
- assistant-context source for each turn;
- RSS samples;
- latency summary;
- token-limit status;
- stop/EOS result;
- reconnect/error result;
- raw pass/fail conclusion and remaining unproven scope.

## Pass Criteria

This gate is complete only when all required models have passing evidence for
256, 512, and 1024-token streaming chat runs, multi-turn conversation runs,
RSS sampling, latency summaries, stop/EOS behavior, and reconnect/error
behavior.

Any one-model or one-length result is partial evidence only.
