# OpenAI Long-Chat x86_64 Qwen 0.5B Memory Retention 3x128 Probe

## Scope

This run repeats the local Qwen 0.5B memory-retention probe on a bounded
x86_64 AVX2 `staging` pod. It runs three identical 128-token generated-context
long-chat sessions against one long-lived OpenAI-compatible Ferrite server
process and samples server RSS before, after, and after idle for every request.

This is a small-model warm-retention probe. It does not prove larger-model
memory safety, 512/1024-token steady-state behavior, multi-client memory
safety, or production leak freedom.

## Environment

- Date: 2026-07-02
- Commit: `c14e161`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-memory-qwen05-3x128`
- Node: `homelab-01`
- Host architecture: `x86_64`
- CPU flags observed: `avx`, `avx2`
- Image: `rust:1.96-bookworm`
- CPU request/limit: `500m` / `2`
- Memory request/limit: `1Gi` / `6Gi`
- Ephemeral-storage request/limit: `6Gi` / `10Gi`
- Work volume: `emptyDir`, `10Gi`
- Server port: `127.0.0.1:18184`
- Server PID for RSS sampling: `1668`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path in pod: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Raw proof logs:
  `target/proof/x86-qwen-0-5b-memory-retention-3x128/`

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
kubectl --context staging get pod ferrite-avx2-memory-qwen05-3x128 \
  --ignore-not-found
```

## Server Command

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18184 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 128 \
  --hard-max-tokens 256 \
  --inference-wait-ms 30000
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Session Command

Each session used the same gate shape:

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --addr 127.0.0.1:18184 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 128 \
  --turns 4 \
  --rss-pid "$SERVER_PID" \
  --prompt "Write a concise operational note about CPU inference stability." \
  --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
  --follow-up "Continue with reconnect and error-handling risks."
```

The three sessions ran sequentially in the same server process with a short idle
delay between sessions. All three session exit files contain `0`.

## Results

All three sessions completed four streaming chat turns with generated assistant
context on turns 2-4, token-limit finish status, usage accounting, streaming
token IDs, RSS samples, and `long_chat_summary_run_complete=true`.

| Session | First request RSS before | Final idle RSS | Delta | Max RSS after | Seed TTFT ms | Seed stream tok/s | Generated TTFT avg ms | Generated stream tok/s avg |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 423370752 | 440016896 | +16646144 | 440016896 | 15190 | 2.280782 | 51503.00 | 1.369049 |
| 2 | 440016896 | 440147968 | +131072 | 440147968 | 15124 | 2.272543 | 51759.00 | 1.365232 |
| 3 | 440147968 | 440147968 | 0 | 440147968 | 15134 | 2.274364 | 51631.00 | 1.364975 |

Pod cgroup memory after the run:

```text
cgroup_current_after=1032531968
cgroup_peak=1411563520
```

Each raw log recorded:

```text
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_run_complete=true
```

## Interpretation

For `Qwen2.5-0.5B-Instruct-Q4_K_M` at this 128-token generated-context shape,
server RSS stabilized after the first warm session. Sessions 2 and 3 ended
within `131072` bytes of their first pre-request RSS samples, and session 3 was
flat from first pre-request RSS to final idle RSS.

The latency pressure remains clear. Seed turns used 47 prompt tokens and reached
first token at about 15.1 seconds. Generated-context turns used 158 prompt
tokens and averaged about 51.5 seconds to first token on the bounded 2-CPU x86
pod. This supports continuing context-windowing and prefix/cache work for
latency, while reducing immediate concern that this small 128-token x86 shape
retains a large new RSS chunk on every repeated session.

The broader KV-cache memory-pressure theory remains open for larger models,
512/1024-token generated contexts, explicit cgroup-aware limits, multi-client
overlap, and cache accounting.
