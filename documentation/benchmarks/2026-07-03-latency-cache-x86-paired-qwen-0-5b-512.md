# Benchmark: x86_64 Paired Latency Cache Qwen 0.5B 512

Date: 2026-07-03 UTC, 2026-07-03 local time

## Purpose

Run the second bounded x86_64 paired measurement from the latency/cache
companion protocol:

- Ferrite's long-chat gate runs inside a bounded amd64 Kubernetes pod and
  provides correctness, generated-context identity, cache metadata, reconnect
  probes, and RSS samples.
- `llama-benchy` runs inside the same pod against the local Ferrite OpenAI
  server and provides an external OpenAI-compatible client-side prefix-cache
  latency view without relying on a long Kubernetes port-forward.

This is a 512-token x86 smoke. It does not complete the x86 paired
256/512/1024 ladder.

## Environment

- Ferrite commit: `042dc01`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-paired-qwen05-512`
- Node: `homelab-01`
- Pod IP: `10.42.248.235`
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
- Pod cgroup memory current after proof: `1289072640` bytes
- Pod cgroup memory peak after build and proof: `1820688384` bytes
- Raw pod proof artifacts copied locally:
  `target/proof/x86-paired-qwen05-512-2026-07-03/`

The staging API had transient readiness interruptions during the run. It
returned `ok` before pod creation, later reported startup/readiness errors
during long control-plane streams, and recovered before artifact collection.

The pod was deleted after artifact collection. A final
`kubectl --context staging get pod ferrite-avx2-paired-qwen05-512 --ignore-not-found`
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
Finished `release` profile [optimized] target(s) in 44.26s
```

## Server

```sh
./target/release/ferrite-server \
  --bind 127.0.0.1:18193 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024 \
  --inference-wait-ms 30000 \
  --experimental-prefix-cache
```

Readiness:

```text
GET /health -> {"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

Server PID for the long-chat RSS sampling: `1654`.

## Ferrite Gate

```sh
./target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --addr 127.0.0.1:18193 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 512 \
  --turns 4 \
  --probe-max-tokens 512 \
  --rss-pid 1654 \
  --prompt-cache-key ferrite:paired:x86:qwen05:latency-cache-512 \
  --prompt-cache-trace \
  --require-cached-follow-ups \
  --expect-finish-reason length \
  --proof-log target/proof/x86-paired-qwen05-512-long-chat.log \
  --proof-exit-code target/proof/x86-paired-qwen05-512-long-chat.exit
```

The gate exited `0` and wrote 210 log lines.

### Probe Results

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=512
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=512
```

### Scenario Results

| Turn | Prompt | Cached | Lookup | TTFT ms | Stream tok/s | RSS idle |
| ---: | ---: | ---: | --- | ---: | ---: | ---: |
| 1 | 43 | 0 | `miss` | 13865 | 2.725661 | 444153856 |
| 2 | 542 | 12 | `shared_prefix_hit` | 179888 | 1.378682 | 473120768 |
| 3 | 542 | 306 | `shared_prefix_hit` | 83048 | 1.866903 | 487751680 |
| 4 | 542 | 20 | `shared_prefix_hit` | 178430 | 1.380660 | 503349248 |

Every scenario reported:

```text
long_chat_result_finish_reason=length
long_chat_result_hit_token_limit=true
long_chat_result_streaming_content_chunks=512
long_chat_result_streaming_token_id_chunks=512
long_chat_result_streaming_token_ids=512
long_chat_result_streaming_all_content_chunks_have_token_ids=true
```

Summary fields included:

```text
long_chat_summary_all_generated_context_identities_match_previous_response=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_run_complete=true
```

## llama-benchy Companion

The first two local workstation attempts used a Kubernetes port-forward and
were rejected as benchmark evidence:

- the first `--latency-mode generation --no-adapt-prompt` run was stopped
  after it exceeded the native gate timing without JSON output;
- the adjusted port-forward run wrote null benchmark rows after
  `TransferEncodingError: 400, message='Not enough data to satisfy transfer length header.'`
  and a subsequent `Cannot connect to host 127.0.0.1:18217`.

The accepted companion benchmark ran inside the pod against
`http://127.0.0.1:18193/v1` after installing `uv 0.11.26` in the proof pod and
restarting the Ferrite server to clear the disconnected request.

```sh
/root/.local/bin/uvx llama-benchy \
  --base-url http://127.0.0.1:18193/v1 \
  --api-key local-secret \
  --model Qwen/Qwen2.5-0.5B-Instruct \
  --served-model-name Qwen2.5-0.5B-Instruct-Q4_K_M \
  --tokenizer Qwen/Qwen2.5-0.5B-Instruct \
  --pp 512 \
  --tg 512 \
  --depth 512 \
  --runs 1 \
  --concurrency 1 \
  --latency-mode none \
  --no-warmup \
  --skip-coherence \
  --adapt-prompt \
  --enable-prefix-caching \
  --extra-body prompt_cache_key=ferrite:paired:x86:qwen05:benchy-512-inpod \
  --format json \
  --save-result target/proof/x86-paired-qwen05-512-llama-benchy-inpod.json
```

The command exited `0`.

Raw JSON:
`documentation/benchmarks/2026-07-03-llama-benchy-x86-qwen-0-5b-paired-cache-512.json`

Captured stdout:
`target/proof/x86-paired-qwen05-512-2026-07-03/x86-paired-qwen05-512-llama-benchy-inpod.stdout`

### llama-benchy Results

| Phase | Depth | Prompt | Generated | Concurrency | TG tok/s | TTFR ms | est PPT ms | E2E TTFT ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| context | 512 | 512 | 512 | 1 | 2.673746 | 41.465539 | 41.465539 | 173297.980096 |
| inference | 512 | 512 | 512 | 1 | 2.453705 | 41.247793 | 41.247793 | 192137.629073 |

`llama-benchy` reported version `0.3.8`, timestamp
`2026-07-03 10:09:21Z`, and `latency_mode=none`. It emitted a tokenizer warning
for the full Sherlock Holmes source corpus length (`143278 > 131072`), but the
accepted run completed and produced non-null benchmark rows.

## Interpretation

The x86 512-token paired run proves the OpenAI-compatible long-chat behavior
under a bounded amd64 pod, but it does not show a robust cache-latency win:

- Ferrite's long-chat gate proved generated assistant context was carried
  across turns and that all generated-context identity links matched previous
  responses.
- Every generated follow-up turn was cached, but all were
  `shared_prefix_hit`; none converged to an exact prompt-cache hit.
- The reusable shared prefix varied sharply across follow-up turns: 12, 306,
  then 20 cached prompt tokens out of 542 prompt tokens.
- TTFT stayed high on turn 2 and turn 4, near 179 seconds, while turn 3 dropped
  to 83 seconds when the shared prefix reached 306 tokens.
- Server RSS stayed bounded: gate idle RSS rose from 444 MiB to 503 MiB, and
  cgroup memory peak across build, proof, in-pod Python tooling, and benchmark
  was 1.82 GiB under a 6 GiB pod limit.
- The in-pod `llama-benchy` run exercised the different
  system-context-prefix shape at depth 512, prompt 512, and generation 512. It
  confirmed slow end-to-end first-token behavior from an external
  OpenAI-compatible client, but it does not expose Ferrite's cached-token
  metadata or generated-context identity fields.

## Operational Notes

The initial Kubernetes exec stream used to run the long-chat gate reset while
the gate was running. The in-pod gate process survived, continued running, and
later exited `0`.

The staging API server had multiple transient readiness interruptions:

- one early `/readyz` attempt failed with `etcd-readiness failed`;
- one later attempt returned `ServiceUnavailable: starting`;
- one later readiness check reported bootstrap post-start hooks not ready;
- each recovered to `/readyz -> ok` before dependent cleanup or artifact
  collection proceeded.

Long `llama-benchy` calls through a Kubernetes port-forward were unstable on
this run. The accepted companion benchmark was moved into the pod to remove
port-forward lifetime from the benchmark path.

Killing the local `llama-benchy` client during the first long port-forward
attempt did not immediately return server CPU to idle. The Ferrite server was
restarted before the accepted in-pod benchmark to clear that disconnected
request. This should become a focused reconnect/cancellation theory rather
than being treated as a completed proof.

## Limits

This run does not prove:

- x86_64 paired behavior at 1024 tokens;
- high-concurrency behavior;
- stop/EOS behavior beyond `finish_reason=length`;
- long-running RSS stability beyond this bounded smoke;
- generated-context exact-hit behavior at 512 tokens;
- that Kubernetes port-forward is reliable for long CPU-bound benchmark calls;
- that `llama-benchy` can replace Ferrite's long-chat gate.
