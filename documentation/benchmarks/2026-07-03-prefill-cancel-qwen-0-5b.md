# Benchmark: Qwen 0.5B Prefill Cancellation Probe

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run a bounded real-model probe for the client-cancellation theory after adding
cooperative prompt-prefill cancellation at transformer-layer boundaries.

The experiment uses Ferrite's OpenAI-compatible streaming chat endpoint with a
long prompt, closes the TCP client after the initial assistant-role SSE event
but before generated content, then immediately sends a short reconnect request.
The main question is whether the reconnect appears stuck behind the abandoned
long-prompt prefill.

## Environment

- Ferrite commit: `8d1830c`
- Kubernetes context: `staging`
- Pod: `ferrite-cancel-qwen05-prefill`
- Node: `homelab-01`
- Pod IP: `10.42.248.197`
- Container image: `rust:1.96-bookworm`
- Architecture: `x86_64`
- CPU: AMD Ryzen 7 5825U with Radeon Graphics
- CPU feature evidence: `/proc/cpuinfo` included `avx2`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `1Gi`
- Memory limit: `6Gi` (`memory.max=6442450944`)
- Ephemeral-storage request: `6Gi`
- Ephemeral-storage limit: `10Gi`
- Workspace size after source copy, model copy, release build, and proof:
  `539M`
- Pod cgroup memory current after proof: `570392576` bytes
- Pod cgroup memory peak after build and proof: `1421094912` bytes

The pod was deleted after artifact collection. A final
`kubectl --context staging get pod ferrite-cancel-qwen05-prefill --ignore-not-found`
returned no pod output.

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Served model id: `qwen2.5-0.5b-q4_k_m`
- Pod path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binary

- `target/release/ferrite-server` SHA256:
  `4003cd5d003115be46e7d717ce9a743af5828cfb6878a01f58fa2c0ca63608ee`

Build result:

```text
Finished `release` profile [optimized] target(s) in 40.29s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18194 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id qwen2.5-0.5b-q4_k_m \
  --api-key local-secret \
  --default-max-tokens 1 \
  --hard-max-tokens 8 \
  --inference-wait-ms 120000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"qwen2.5-0.5b-q4_k_m"}
```

Server PID: `1557`

## Probe

The abort request used:

- endpoint: `POST /v1/chat/completions`
- `stream: true`
- `max_tokens: 1`
- prompt shape: system message plus one user message
- user prompt length: `140398` characters

The client read until the initial assistant-role SSE event, waited `500 ms`,
then closed the socket. The partial response contained no generated content.

Immediately after closing the first socket, the script sent a short streaming
reconnect request:

```json
{"messages":[{"role":"user","content":"Say ok."}],"max_tokens":1,"stream":true}
```

Raw JSON:
`documentation/benchmarks/2026-07-03-prefill-cancel-qwen-0-5b.json`

Raw artifact SHA256:

```text
6579bde7586e127a0e31b0c94a7bf7c1d3dcb7e47c9584166f81374a62764f8d  documentation/benchmarks/2026-07-03-prefill-cancel-qwen-0-5b.json
```

## Results

| Metric | Value |
| --- | ---: |
| Initial role event observed | true |
| Time to initial role marker | `1.095 ms` |
| Delay after role marker before socket close | `500 ms` |
| Generated content before close | false |
| Reconnect started after abort close | `0.173 ms` |
| Reconnect first generated event | `8904.287 ms` |
| Reconnect done | `9206.984 ms` |
| Reconnect status | `HTTP/1.1 200 OK` |
| Reconnect generated event | true |

RSS samples:

| Sample | RSS |
| --- | ---: |
| Before abort request | `413924 KiB` |
| Immediately after abort close | `428900 KiB` |
| Reconnect sample max | `428900 KiB` |
| After reconnect | `426060 KiB` |

The server log artifact was empty.

## Interpretation

This is a positive real-model smoke for the prompt-prefill cancellation work.
The client closed a long-prompt streaming request before generated content, and
an immediate reconnect request completed with generated content in about
`9.207 s`. That is consistent with the abandoned long-prompt request releasing
the single inference permit promptly enough for the reconnect to proceed.

The result does not prove a precise cancellation latency bound. The harness
does not yet record the exact server-side moment when the stream receiver is
marked closed, nor how many prompt layers or prompt tokens were evaluated after
that point. It also lacks a same-run short-prompt baseline, so the reconnect
TTFT should be interpreted as "not stuck behind a long abandoned prefill" rather
than as an optimized latency result.

## Limits

This run does not prove:

- cancellation through Kubernetes port-forward;
- cancellation inside a single layer or matvec;
- exact prompt-token or layer counts after disconnect;
- behavior for Qwen2.5 1.5B, SmolLM2 1.7B, or larger models;
- high-concurrency reconnect behavior.

## Next Step

Add request-lifetime instrumentation if stronger proof is needed: request id,
disconnect observation point, prompt-token count, prompt-layer count, and
permit-release timestamp. That would convert this smoke from external timing
evidence into a direct server-side cancellation-latency measurement.
