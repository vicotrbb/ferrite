# OpenAI Long-Chat x86_64 Qwen 1.5B Q6 256-Token Probe Gate

## Scope

This run starts the x86_64 combined reconnect/error long-chat proof set for the
larger `Qwen2.5-1.5B-Instruct-Q6_K` Tier 1 artifact. It exercises the
OpenAI-compatible HTTP server in a bounded amd64 Kubernetes pod with
`--probe-max-tokens 256`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
256-token budget.

This is one model, one token length, and one bounded x86_64 pod. It does not
close the x86_64 long-chat gate for Qwen2.5-1.5B Q6_K or for the full Tier 1
HTTP model set.

## Environment

- Date: 2026-07-01
- Commit: `c5187cc`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-long-chat-qwen15-q6-256`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `2Gi`
- Memory limit: `6Gi` (`memory.max=6442450944`)
- Ephemeral-storage request: `6Gi`
- Ephemeral-storage limit: `10Gi`
- Rust toolchain: `rustc 1.96.0`, host `x86_64-unknown-linux-gnu`
- Model: `Qwen2.5-1.5B-Instruct-Q6_K`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Model SHA256:
  `e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7`
- Server PID for RSS sampling: `1661`
- Server port inside pod: `127.0.0.1:18124`
- Gate launcher PID in pod: `1699`
- Gate process PID in pod: `1704`
- Gate exit-code file: `target/proof/x86-qwen-1-5b-q6-long-chat-probe-256.exit`
- Gate exit code: `0`
- Pod cgroup memory peak after build and proof: `4268711936` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `1.6G`
- Raw log: `target/proof/x86-qwen-1-5b-q6-long-chat-probe-256.log`
- Server log: `target/proof/x86-qwen-1-5b-q6-server-256.log`

The pod-side release binaries were built inside the amd64 pod. `file` reported
both `target/release/ferrite-server` and
`target/release/ferrite-openai-long-chat-gate` as `ELF 64-bit LSB pie
executable, x86-64`.

Pod-side release binary hashes:

```text
e5f148ec2c6686d532ac1ea37c48abf6f9ba7c3a4ba46c1ffae28a98ebed261a  target/release/ferrite-server
f613e12832ee0d9ccad126a8ab900e2fe0ee6e8612c181b3d7de137264a8ff24  target/release/ferrite-openai-long-chat-gate
```

## Pod Manifest

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ferrite-avx2-long-chat-qwen15-q6-256
spec:
  restartPolicy: Never
  nodeSelector:
    kubernetes.io/arch: amd64
  containers:
    - name: ferrite
      image: rust:1.96-bookworm
      command: ["/bin/sh", "-lc", "sleep 86400"]
      resources:
        requests:
          cpu: "500m"
          memory: "2Gi"
          ephemeral-storage: "6Gi"
        limits:
          cpu: "2"
          memory: "6Gi"
          ephemeral-storage: "10Gi"
      volumeMounts:
        - name: work
          mountPath: /work
  volumes:
    - name: work
      emptyDir:
        sizeLimit: 10Gi
```

## Build Command

```sh
kubectl --context staging exec ferrite-avx2-long-chat-qwen15-q6-256 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 40.64s
```

The cgroup memory peak immediately after the build and before server startup
was `2118524928` bytes.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18124 \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q6_K \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q6_K"}
```

Initial `ps` RSS for the server process was `1455368` KiB. The pod cgroup
memory peak after model load was `4268711936` bytes, below the `6Gi` memory
limit.

## Gate Command

The gate ran detached inside the pod and wrote its process exit code to a
separate file, so transient `kubectl exec` stream resets could not terminate
the gate process.

```sh
kubectl --context staging exec ferrite-avx2-long-chat-qwen15-q6-256 -- sh -lc \
  'cd /work/ferrite && rm -f \
    target/proof/x86-qwen-1-5b-q6-long-chat-probe-256.log \
    target/proof/x86-qwen-1-5b-q6-long-chat-probe-256.exit && \
    nohup sh -lc '"'"'./target/release/ferrite-openai-long-chat-gate \
      --execute \
      --error-probe \
      --disconnect-probe \
      --models Qwen2.5-1.5B-Instruct-Q6_K \
      --token-lengths 256 \
      --turns 4 \
      --addr 127.0.0.1:18124 \
      --api-key local-secret \
      --rss-pid 1661 \
      --probe-max-tokens 256 \
      --expect-finish-reason length \
      > target/proof/x86-qwen-1-5b-q6-long-chat-probe-256.log 2>&1; \
      echo $? > target/proof/x86-qwen-1-5b-q6-long-chat-probe-256.exit'"'"' \
      >/dev/null 2>&1 & echo $!'
```

The exit-code file contained:

```text
0
```

## Probe Results

Both probes completed and recorded the configured 256-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_max_tokens=256
```

## Scenario Results

All four 256-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 256 completion tokens, streaming
timing, per-token latency summaries, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 346995 | 257 | 48856 | 344980 | 0.744970 | 1086 | 1140 | 1263 | 48856 | 1519206400 | 1519333376 | 1519333376 |
| 2 | 256 | 1 | length | 341704 | 257 | 48122 | 339691 | 0.756569 | 1080 | 1130 | 1203 | 48122 | 1519333376 | 1519362048 | 1519362048 |
| 3 | 256 | 1 | length | 351484 | 257 | 48259 | 349470 | 0.735399 | 1076 | 1143 | 1536 | 48259 | 1519362048 | 1519308800 | 1519308800 |
| 4 | 256 | 1 | length | 344510 | 257 | 49222 | 342497 | 0.750371 | 1084 | 1135 | 1218 | 49222 | 1519308800 | 1519435776 | 1519435776 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `256`;
- total tokens: `299`.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_timing_present=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_run_complete=true
```

## Staging Control-Plane Notes

During setup and observation, the API server intermittently returned
connection-refused, `ServiceUnavailable`, kubelet proxy `502 Bad Gateway`, and
websocket EOF/reset errors for `kubectl exec` streams. The proof itself ran
detached inside the pod, wrote `exit=0`, and reached
`long_chat_summary_run_complete=true`. The pod stayed `Running` with zero
restarts before cleanup.

This evidence is valid for the Ferrite server/gate behavior recorded in the
in-pod log, but it should not be used as a clean staging
control-plane-stability sample.

## Cleanup

The server process was stopped after the run. The raw proof log, server log,
and gate exit-code file were copied back to local `target/proof/`, then the pod
was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-long-chat-qwen15-q6-256 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-long-chat-qwen15-q6-256 --ignore-not-found
```

The delete command completed, and the final `get pod` command returned no
output.

## Interpretation

Ferrite now has real x86_64 AVX2 combined long-chat reconnect/error proof for
Qwen2.5-1.5B Q6_K at the 256 completion-token budget.

Remaining proof gaps:

- repeat this x86_64 shape for Qwen2.5-1.5B Q6_K at 512 and 1024 completion
  tokens;
- repeat x86_64 combined runs for SmolLM2-1.7B Q4_K_M;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence.
