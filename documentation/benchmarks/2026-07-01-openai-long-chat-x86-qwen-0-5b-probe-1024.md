# OpenAI Long-Chat x86_64 Qwen 0.5B 1024-Token Probe Gate

## Scope

This run completes the x86_64 token-length slice for the combined
reconnect/error long-chat proof gate for `Qwen2.5-0.5B-Instruct-Q4_K_M`. It
exercises the OpenAI-compatible HTTP server in a bounded amd64 Kubernetes pod
with `--probe-max-tokens 1024`, so the request-error reconnect path,
disconnect reconnect path, and all repeated streaming chat scenarios use the
same 1024-token budget.

This is one model, one token length, and one bounded x86_64 pod. It completes
the 256/512/1024 x86_64 token-budget slice for Qwen2.5-0.5B Q4_K_M, but it
does not close the x86_64 long-chat gate for the full Tier 1 HTTP model set.

## Environment

- Date: 2026-07-01
- Commit: `7ceace2`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-long-chat-qwen05-1024`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `1Gi`
- Memory limit: `6Gi` (`memory.max=6442450944`)
- Ephemeral-storage request: `6Gi`
- Ephemeral-storage limit: `10Gi`
- Rust toolchain: `rustc 1.96.0`, host `x86_64-unknown-linux-gnu`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path in pod: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Server PID for RSS sampling: `1684`
- Server port inside pod: `127.0.0.1:18120`
- Gate launcher PID in pod: `1717`
- Gate process PID in pod: `1722`
- Gate exit-code file: `target/proof/x86-qwen-0-5b-q4-long-chat-probe-1024.exit`
- Gate exit code: `0`
- Pod cgroup memory peak after build and proof: `1451151360` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `538M`
- Raw log: `target/proof/x86-qwen-0-5b-q4-long-chat-probe-1024.log`
- Server log: `target/proof/x86-qwen-0-5b-server-1024.log`

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
  name: ferrite-avx2-long-chat-qwen05-1024
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
          memory: "1Gi"
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
kubectl --context staging exec ferrite-avx2-long-chat-qwen05-1024 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 43.68s
```

The cgroup memory peak immediately after the build and before the proof was
`1104633856` bytes.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18120 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1280
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

Initial `ps` RSS for the server process was `413092` KiB.

## Gate Command

The gate ran detached inside the pod and wrote its process exit code to a
separate file, so transient `kubectl exec` stream resets could not terminate
the gate process.

```sh
kubectl --context staging exec ferrite-avx2-long-chat-qwen05-1024 -- sh -lc \
  'cd /work/ferrite && rm -f \
    target/proof/x86-qwen-0-5b-q4-long-chat-probe-1024.log \
    target/proof/x86-qwen-0-5b-q4-long-chat-probe-1024.exit && \
    nohup sh -lc '"'"'./target/release/ferrite-openai-long-chat-gate \
      --execute \
      --error-probe \
      --disconnect-probe \
      --models Qwen2.5-0.5B-Instruct-Q4_K_M \
      --token-lengths 1024 \
      --turns 4 \
      --addr 127.0.0.1:18120 \
      --api-key local-secret \
      --rss-pid 1684 \
      --probe-max-tokens 1024 \
      --expect-finish-reason length \
      > target/proof/x86-qwen-0-5b-q4-long-chat-probe-1024.log 2>&1; \
      echo $? > target/proof/x86-qwen-0-5b-q4-long-chat-probe-1024.exit'"'"' \
      >/dev/null 2>&1 & echo $!'
```

The exit-code file contained:

```text
0
```

## Probe Results

Both probes completed and recorded the configured 1024-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=1024
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_max_tokens=1024
```

## Scenario Results

All four 1024-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 1024 completion tokens, streaming
timing, per-token latency summaries, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1024 | 1 | length | 369236 | 1025 | 13501 | 367214 | 2.791281 | 297 | 342 | 391 | 13501 | 454332416 | 455249920 | 455249920 |
| 2 | 1024 | 1 | length | 369935 | 1025 | 13716 | 367917 | 2.785947 | 295 | 340 | 396 | 13716 | 455249920 | 455249920 | 455249920 |
| 3 | 1024 | 1 | length | 365607 | 1025 | 13605 | 363588 | 2.819123 | 295 | 340 | 378 | 13605 | 455249920 | 455249920 | 455249920 |
| 4 | 1024 | 1 | length | 370667 | 1025 | 13464 | 368644 | 2.780459 | 297 | 342 | 398 | 13464 | 455249920 | 455249920 | 455249920 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `1024`;
- total tokens: `1067`.

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

During the run, several `kubectl exec` streams used for interactive
observation reset, the API server briefly returned `ServiceUnavailable`, and
staging node readiness temporarily flapped. The gate itself ran detached inside
the pod, wrote `exit=0`, and reached `long_chat_summary_run_complete=true`.
The pod stayed `Running` with zero restarts before cleanup.

This evidence is valid for the Ferrite server/gate behavior recorded in the
in-pod log, but it should not be used as a clean staging
control-plane-stability sample.

## Cleanup

The server process was stopped after the run. The raw proof log, server log,
and gate exit-code file were copied back to local `target/proof/`, then the pod
was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-long-chat-qwen05-1024 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-long-chat-qwen05-1024 --ignore-not-found
```

The delete command completed, and the final `get pod` command returned no
output.

## Interpretation

Ferrite now has real x86_64 AVX2 combined long-chat reconnect/error proof for
Qwen2.5-0.5B Q4_K_M at the 256, 512, and 1024 completion-token budgets.

Remaining proof gaps:

- repeat x86_64 combined runs for Qwen2.5-1.5B Q8_0, Qwen2.5-1.5B Q6_K, and
  SmolLM2-1.7B Q4_K_M;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence.
