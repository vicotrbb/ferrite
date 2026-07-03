# Benchmark: x86_64 Paired Latency Cache Qwen 0.5B 256

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the first bounded x86_64 paired measurement from the latency/cache
companion protocol:

- Ferrite's long-chat gate runs inside a bounded amd64 Kubernetes pod and
  provides correctness, generated-context identity, cache metadata, reconnect
  probes, and RSS samples.
- `llama-benchy` runs from the local workstation through a Kubernetes
  port-forward and provides an external OpenAI-compatible client-side
  prefix-cache latency view against the x86 server.

This is a 256-token x86 smoke. It does not complete the x86 paired
256/512/1024 ladder.

## Environment

- Ferrite commit: `eba93a8`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-paired-qwen05-256`
- Node: `homelab-01`
- Pod IP: `10.42.248.195`
- Container image: `rust:1.96-bookworm`
- Architecture: `x86_64`
- CPU feature evidence: `/proc/cpuinfo` included `avx` and `avx2`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `1Gi`
- Memory limit: `6Gi` (`memory.max=6442450944`)
- Ephemeral-storage request: `6Gi`
- Ephemeral-storage limit: `10Gi`
- Workspace size after source copy, model copy, release build, and proof:
  `541M`
- Pod cgroup memory current after proof: `1160372224` bytes
- Pod cgroup memory peak after build and proof: `1481572352` bytes
- Raw pod proof artifacts copied locally:
  `target/proof/x86-paired-qwen05-256-2026-07-03/`

The staging API was checked before pod creation:

```text
kubectl config current-context -> staging
kubectl --context staging get --raw=/readyz -> ok
```

The pod was deleted after artifact collection. A final
`kubectl --context staging get pod ferrite-avx2-paired-qwen05-256 --ignore-not-found`
returned no pod output.

## Model

- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Pod path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`

## Binaries

The binaries were built inside the amd64 pod. `file` reported both as
`ELF 64-bit LSB pie executable, x86-64`.

- `target/release/ferrite-server` SHA256:
  `fa71c1ced92ef06697c1220e313dfdab6faf08da6d8851301e3013793ccb2727`
- `target/release/ferrite-openai-long-chat-gate` SHA256:
  `d1e9fbed42b9e65f950738d38e20a57162dec2a2c8078137dcaecfd598d1ba62`

Build result:

```text
Finished `release` profile [optimized] target(s) in 47.92s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18192 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

Server PID for RSS sampling: `1652`.

## Ferrite Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18192 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 256 \
  --turns 4 \
  --probe-max-tokens 256 \
  --rss-pid 1652 \
  --prompt-cache-key ferrite:paired:x86:qwen05:latency-cache-256 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/x86-paired-qwen05-256-long-chat.log \
  --proof-exit-code target/proof/x86-paired-qwen05-256-long-chat.exit
```

The gate exited `0` and wrote 210 log lines.

### Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=256
```

### Scenario Results

| Turn | Prompt | Cached | Lookup | TTFT ms | Stream tok/s | RSS idle |
| ---: | ---: | ---: | --- | ---: | ---: | ---: |
| 1 | 43 | 0 | `miss` | 13806 | 2.614425 | 442167296 |
| 2 | 286 | 12 | `shared_prefix_hit` | 90550 | 1.433338 | 451080192 |
| 3 | 286 | 14 | `shared_prefix_hit` | 89956 | 1.434757 | 460517376 |
| 4 | 286 | 14 | `shared_prefix_hit` | 89993 | 1.435260 | 468201472 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=256
long_chat_result_streaming_token_id_chunks=256
long_chat_result_streaming_token_ids=256
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

Summary fields included:

```text
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_run_complete=true
```

## llama-benchy Companion

The companion benchmark ran from the local workstation through a Kubernetes
port-forward to the x86 pod.

```sh
uvx llama-benchy \
  --base-url http://127.0.0.1:18216/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 256 \
  --tg 256 \
  --depth 256 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode generation \
  --no-warmup \
  --skip-coherence \
  --no-adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:paired:x86:qwen05:benchy-256-rerun \
  --format json \
  --save-result documentation/benchmarks/2026-07-03-llama-benchy-x86-qwen-0-5b-paired-cache-256.json
```

The command exited `0`.

Raw JSON:
`documentation/benchmarks/2026-07-03-llama-benchy-x86-qwen-0-5b-paired-cache-256.json`

Captured stdout:
`target/proof/x86-paired-qwen05-256-llama-benchy-rerun.stdout`

### llama-benchy Results

| Phase | Depth | Prompt | Generated | Concurrency | TG tok/s | TTFR ms | est PPT ms | E2E TTFT ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| context | 256 | 256 | 256 | 1 | 2.882023 | 46.157000 | 22.449389 | 87046.129750 |
| inference | 256 | 256 | 256 | 1 | 2.756126 | 7.736167 | 0.0 | 90266.089625 |

## Interpretation

The first x86 paired run preserves the local 256-token qualitative result while
showing the expected x86 latency difference:

- Ferrite's long-chat gate proved generated assistant context was carried
  across turns and that all generated-context identity links matched previous
  responses.
- The generated-context lane did not converge to an exact prompt fixed point.
  Follow-up turns reused only 12 to 14 prompt tokens and remained
  `shared_prefix_hit`.
- TTFT for generated follow-up turns stayed near 90 seconds on the bounded x86
  pod, much slower than the local macOS 256-token paired smoke.
- `llama-benchy` successfully exercised the different OpenAI-compatible
  system-context prefix-cache shape at depth 256, prompt 256, and generation
  256 through a port-forward to the x86 server.
- The external companion run produced portable JSON, but it did not expose
  Ferrite's cached-token metadata or generated-context identity fields.

## Operational Notes

The first server launch command started the server but failed to write the PID
file because shell backgrounding moved the working directory context away from
`/work/ferrite`. The PID was recovered with `pgrep` and recorded manually. This
was an operator command issue, not a Ferrite runtime failure.

The first `llama-benchy` attempt through port `18215` produced a valid JSON
file with a complete context row but null inference metrics after
`TransferEncodingError: 400, message='Not enough data to satisfy transfer length header.'`.
At the same time, the port-forward reported `lost connection to pod`, and a
nearby Kubernetes exec hit `etcdserver: leader changed`. The pod had zero
restarts, the Ferrite server remained healthy inside the pod, and a second
port-forward on `18216` completed the companion benchmark. The partial JSON was
not committed; the successful rerun is the archived benchmark artifact.

## Limits

This run does not prove:

- x86_64 paired behavior at 512 or 1024 tokens;
- high-concurrency behavior;
- stop/EOS behavior;
- long-running RSS stability;
- generated-context exact-hit behavior at 256 tokens;
- that `llama-benchy` can replace Ferrite's long-chat gate.
