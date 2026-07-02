# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Token Windowing 256 Probe

## Scope

This run tests the benchmark-only generated-context token window on a larger
x86_64 Tier 1 artifact: `Qwen2.5-1.5B-Instruct-Q8_0`. It uses Ferrite's
OpenAI-compatible HTTP server and the long-chat gate with
`--generated-context-max-tokens 64`.

This is a latency and prompt-size theory probe. It does not change Ferrite's
default serving policy, does not prove conversation quality, and does not cover
reconnect or disconnect behavior in this slice.

## Environment

- Date: 2026-07-02
- Commit: `eb83903`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-window-qwen15-q8-256`
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
- Server PID for RSS sampling: `1670`
- Server port inside pod: `127.0.0.1:18186`
- Server RSS after model load: `1875588` KiB
- Pod cgroup memory current after model load: `3435372544` bytes
- Pod cgroup memory peak after build, model load, and proof:
  `5325168640` bytes
- Pod cgroup memory current after proof before server stop: `3460440064` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw proof directory:
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
kubectl --context staging get pod ferrite-avx2-window-qwen15-q8-256 \
  --ignore-not-found
```

Pod-side binary SHA256 values:

```text
7efa78c8b876973d25c2f1c03bf3399f6d1c7aefb1f61773081d6994a4e0e516  target/release/ferrite-server
039c61d988b26bdf829946e84ca6fae7b5398798af06ef979c7a538515ce8487  target/release/ferrite-openai-long-chat-gate
```

## Server Command

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18186 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
  --inference-wait-ms 30000
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Command

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --addr 127.0.0.1:18186 \
  --api-key local-secret \
  --models Qwen2.5-1.5B-Instruct-Q8_0 \
  --token-lengths 256 \
  --turns 4 \
  --rss-pid "$SERVER_PID" \
  --generated-context-max-tokens 64 \
  --prompt "Write a concise operational note about CPU inference stability." \
  --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
  --follow-up "Continue with reconnect and error-handling risks."
```

The plan output included:

```text
long_chat_generated_context_max_tokens=64
```

The gate wrote `0` to
`target/proof/x86-qwen-1-5b-q8-window-64tokens-256.exit`.

## Results

All four 256-token streaming chat turns completed with `finish_reason=length`,
valid usage accounting, token-limit status, generated-context status,
stream-observed timing, streaming token IDs, and RSS samples.

| Turn | Context | Prompt tokens | Completion tokens | TTFT/prefill ms | Decode ms | Decode tok/s | Stream tok/s | RSS before | RSS after | RSS idle |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 47 | 256 | 11042 | 64532 | 3.966999 | 3.400592 | 1920602112 | 1940000768 | 1940000768 |
| 2 | generated | 92 | 256 | 21752 | 65133 | 3.930389 | 2.957895 | 1940000768 | 1945505792 | 1945505792 |
| 3 | generated | 91 | 256 | 21442 | 65611 | 3.901780 | 2.952194 | 1945505792 | 1945505792 | 1945505792 |
| 4 | generated | 94 | 256 | 22094 | 65388 | 3.915064 | 2.937719 | 1945505792 | 1945505792 | 1945505792 |

The windowed run recorded:

```text
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_run_complete=true
```

## Baseline Comparison

The comparison baseline is
`documentation/benchmarks/2026-07-02-openai-long-chat-x86-qwen-1-5b-q8-prefill-decode-theory-256.md`.
That baseline used the same model, token budget, x86_64 node family, and
OpenAI-compatible long-chat path, but it carried the full generated assistant
context and also enabled reconnect/error probes.

Generated-turn averages:

| Metric | Unwindowed baseline | 64-token window | Change |
| --- | ---: | ---: | ---: |
| Prompt tokens | 285.33 | 92.33 | -67.64% |
| TTFT/prefill ms | 70209.33 | 21762.67 | -69.01% |
| Decode ms | 71463.33 | 65377.33 | -8.52% |
| Decode tok/s | 3.582265 | 3.915744 | +9.31% |
| Stream tok/s | 1.814102 | 2.949269 | +62.57% |

## Interpretation

The larger x86_64 Q8 probe supports the generated-context windowing theory.
Keeping the trailing 64 generated content chunks cut generated-turn prompt
tokens by about two thirds and reduced generated-turn first-token delay by
about 69 percent versus the unwindowed 256-token Q8 baseline.

The decode phase also improved, but much less than TTFT. This is consistent
with the existing prefix-reuse theory: the biggest cost is reprocessing a long
generated-context prompt before the first streamed token.

This result is strong enough to justify a small window-size sweep on the same
model. It is not enough to make token windowing the default public HTTP policy;
that still needs conversation-quality checks, explicit user-facing semantics,
and reconnect/error coverage with the selected window size.
