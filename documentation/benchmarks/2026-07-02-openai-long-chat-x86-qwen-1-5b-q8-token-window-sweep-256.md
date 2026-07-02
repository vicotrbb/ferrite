# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Token Window Sweep 256

## Scope

This run completes the first x86_64 generated-context token-window sweep for
`Qwen2.5-1.5B-Instruct-Q8_0` at the 256-token long-chat budget. It adds
32-token, 128-token, and 256-token generated-context windows to the previously
recorded 64-token result.

This is a benchmark and theory-validation slice. It does not change Ferrite's
default OpenAI-compatible serving policy, does not prove conversation quality,
and does not include reconnect/error probes.

## Environment

- Date: 2026-07-02
- Commit: `8845d6a`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-window-sweep-qwen15-q8-256`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Host architecture: `x86_64`
- CPU features observed: `avx`, `avx2`
- CPU request/limit: `500m` / `2`
- Memory request/limit: `1Gi` / `8Gi`
- Ephemeral-storage request/limit: `6Gi` / `10Gi`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw proof directory:
  `target/proof/x86-qwen-1-5b-q8-window-sweep-256/`
- Companion 64-token proof directory:
  `target/proof/x86-qwen-1-5b-q8-window-64tokens-256/`

The staging API was verified before pod creation:

```sh
kubectl config current-context
kubectl --context staging get --raw=/readyz
```

Outputs:

```text
staging
ok
```

The pod was deleted after logs were copied locally, and a final pod check
returned no pod:

```sh
kubectl --context staging get pod ferrite-avx2-window-sweep-qwen15-q8-256 \
  --ignore-not-found
```

Pod-side binary SHA256 values:

```text
7efa78c8b876973d25c2f1c03bf3399f6d1c7aefb1f61773081d6994a4e0e516  target/release/ferrite-server
039c61d988b26bdf829946e84ca6fae7b5398798af06ef979c7a538515ce8487  target/release/ferrite-openai-long-chat-gate
```

## Infrastructure Note

During the 256-token-window run, the local `kubectl exec` stream reset:

```text
error reading from error stream: read tcp ...: read: connection reset by peer
```

The API server `/readyz` still returned `ok`, but `homelab-01` briefly reported
`NotReady` and direct exec calls returned kubelet proxy `502 Bad Gateway`.
Follow-up exec calls recovered. The pod stayed `Running`, the sweep process
continued inside the pod, and all three window runs wrote exit `0`.

## Server Shape

Each window used a fresh server process:

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:${port} \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
  --inference-wait-ms 30000
```

Each server was health-checked before its gate run:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Shape

Each window used the same long-chat gate shape, changing only
`--generated-context-max-tokens`:

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --addr 127.0.0.1:${port} \
  --api-key local-secret \
  --models Qwen2.5-1.5B-Instruct-Q8_0 \
  --token-lengths 256 \
  --turns 4 \
  --rss-pid "$SERVER_PID" \
  --generated-context-max-tokens "$window" \
  --prompt "Write a concise operational note about CPU inference stability." \
  --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
  --follow-up "Continue with reconnect and error-handling risks."
```

The 32, 128, and 256 window runs all wrote exit `0` and recorded:

```text
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_all_rss_present=true
long_chat_summary_run_complete=true
```

## Turn Results

### 32-Token Window

| Turn | Context | Prompt tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 47 | 10998 | 64196 | 3.987765 | 3.417806 | 1940115456 |
| 2 | generated | 60 | 14057 | 64316 | 3.980340 | 3.279184 | 1941032960 |
| 3 | generated | 63 | 14771 | 64581 | 3.963967 | 3.238687 | 1944178688 |
| 4 | generated | 62 | 14400 | 64417 | 3.974069 | 3.260684 | 1944178688 |

### 128-Token Window

| Turn | Context | Prompt tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 47 | 11110 | 64318 | 3.980191 | 3.407186 | 1939877888 |
| 2 | generated | 154 | 37015 | 67789 | 3.776418 | 2.452187 | 1940123648 |
| 3 | generated | 155 | 37344 | 68066 | 3.761035 | 2.438081 | 1946415104 |
| 4 | generated | 156 | 37363 | 68052 | 3.761809 | 2.437953 | 1946415104 |

### 256-Token Window

| Turn | Context | Prompt tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 47 | 11011 | 63960 | 4.002442 | 3.427942 | 1939968000 |
| 2 | generated | 280 | 69057 | 72229 | 3.544277 | 1.818994 | 1954648064 |
| 3 | generated | 286 | 73069 | 71264 | 3.592247 | 1.780595 | 1955172352 |
| 4 | generated | 286 | 70653 | 73973 | 3.460682 | 1.776981 | 1955172352 |

## Sweep Comparison

Generated-turn averages:

| Window | Prompt tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 32 | 61.67 | 14409.33 | 64438.00 | 3.972792 | 3.259518 |
| 64 | 92.33 | 21762.67 | 65377.33 | 3.915744 | 2.949269 |
| 128 | 155.00 | 37240.67 | 67969.00 | 3.766421 | 2.442740 |
| 256 | 284.00 | 70926.33 | 72488.67 | 3.532402 | 1.792190 |
| Unwindowed baseline | 285.33 | 70209.33 | 71463.33 | 3.582265 | 1.814102 |

The unwindowed baseline is
`documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-prefill-decode-theory-256.md`.
The 64-token window row comes from
`documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-token-windowing-256.md`.

## Interpretation

The sweep shows a clear latency curve for generated-context window size:

- 32 generated chunks is the fastest measured window on this prompt, with
  generated-turn TTFT around `14.4s` and stream throughput around `3.26 tok/s`.
- 64 generated chunks remains strong, with TTFT around `21.8s` and stream
  throughput around `2.95 tok/s`.
- 128 generated chunks is a middle point, with TTFT around `37.2s`.
- 256 generated chunks is effectively equivalent to the full generated-context
  baseline on this 256-token completion run.

This supports designing a real policy around small generated-context windows,
probably starting with 32 or 64 generated chunks for performance probes. The
choice cannot be made on latency alone: conversation-continuity checks,
reconnect/error probes, and at least one 512-token budget are still required
before any default serving policy is justified.
