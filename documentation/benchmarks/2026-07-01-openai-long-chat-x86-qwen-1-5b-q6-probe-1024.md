# OpenAI Long-Chat x86_64 Qwen 1.5B Q6 1024-Token Probe Gate

## Scope

This run completes the x86_64 combined reconnect/error long-chat proof set for
the larger `Qwen2.5-1.5B-Instruct-Q6_K` Tier 1 artifact. It exercises the
OpenAI-compatible HTTP server in a bounded amd64 Kubernetes pod with
`--probe-max-tokens 1024`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
1024-token budget.

This is one model, one token length, and one bounded x86_64 pod. It completes
the 256/512/1024 x86_64 long-chat budget set for Qwen2.5-1.5B Q6_K, but it
does not close the x86_64 long-chat gate for the full Tier 1 HTTP model set.

## Environment

- Date: 2026-07-01
- Commit: `e9f1c89`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-long-chat-qwen15-q6-1024`
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
- Server PID for RSS sampling: `1671`
- Server port inside pod: `127.0.0.1:18126`
- Gate launcher PID in pod: `1710`
- Gate process PID in pod: `1715`
- Gate exit-code file: `target/proof/x86-qwen-1-5b-q6-long-chat-probe-1024.exit`
- Gate exit code: `0`
- Pod cgroup memory peak after build and proof: `3680645120` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `1.6G`
- Raw log: `target/proof/x86-qwen-1-5b-q6-long-chat-probe-1024.log`
- Server log: `target/proof/x86-qwen-1-5b-q6-server-1024.log`

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
  name: ferrite-avx2-long-chat-qwen15-q6-1024
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
kubectl --context staging exec ferrite-avx2-long-chat-qwen15-q6-1024 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 39.78s
```

The cgroup memory peak immediately after the build and before server startup
was `2140860416` bytes.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18126 \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q6_K \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1280
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q6_K"}
```

Initial `ps` RSS for the server process was `1455188` KiB. The pod cgroup
memory peak after model load was `3680645120` bytes, below the `6Gi` memory
limit.

## Gate Command

The gate ran detached inside the pod and wrote its process exit code to a
separate file, so transient `kubectl exec` stream resets could not terminate
the gate process.

```sh
kubectl --context staging exec ferrite-avx2-long-chat-qwen15-q6-1024 -- sh -lc \
  'cd /work/ferrite && rm -f \
    target/proof/x86-qwen-1-5b-q6-long-chat-probe-1024.log \
    target/proof/x86-qwen-1-5b-q6-long-chat-probe-1024.exit && \
    nohup sh -lc '"'"'./target/release/ferrite-openai-long-chat-gate \
      --execute \
      --error-probe \
      --disconnect-probe \
      --models Qwen2.5-1.5B-Instruct-Q6_K \
      --token-lengths 1024 \
      --turns 4 \
      --addr 127.0.0.1:18126 \
      --api-key local-secret \
      --rss-pid 1671 \
      --probe-max-tokens 1024 \
      --expect-finish-reason length \
      > target/proof/x86-qwen-1-5b-q6-long-chat-probe-1024.log 2>&1; \
      echo $? > target/proof/x86-qwen-1-5b-q6-long-chat-probe-1024.exit'"'"' \
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
| 1 | 1024 | 1 | length | 1277993 | 1025 | 49349 | 1275976 | 0.803307 | 1074 | 1191 | 1271 | 49349 | 1574952960 | 1575378944 | 1575378944 |
| 2 | 1024 | 1 | length | 1295941 | 1025 | 48430 | 1293922 | 0.792165 | 1097 | 1198 | 1359 | 48430 | 1575378944 | 1575063552 | 1575063552 |
| 3 | 1024 | 1 | length | 1273656 | 1025 | 48687 | 1271637 | 0.806047 | 1085 | 1173 | 1346 | 48687 | 1575063552 | 1575297024 | 1575297024 |
| 4 | 1024 | 1 | length | 1263484 | 1025 | 48959 | 1261465 | 0.812547 | 1082 | 1177 | 1263 | 48959 | 1575297024 | 1575399424 | 1575399424 |

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

During setup, the initial `git archive | kubectl exec` source copy failed with
`unexpected EOF` while the API server also briefly refused requests. A
follow-up inspection showed the pod still `Running` with an empty workspace,
and the source copy succeeded after the API recovered. During cleanup, the pod
delete completed while the watch reported `apiserver not ready`; a direct retry
confirmed no pod output and both nodes `Ready`.

The proof itself ran detached inside the pod, wrote `exit=0`, and reached
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
kubectl --context staging delete pod ferrite-avx2-long-chat-qwen15-q6-1024 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-long-chat-qwen15-q6-1024 --ignore-not-found
```

The delete command completed, and the final direct `get pod` command returned
no pod output.

## Interpretation

Ferrite now has real x86_64 AVX2 combined long-chat reconnect/error proof for
Qwen2.5-1.5B Q6_K at the 256, 512, and 1024 completion-token budgets.

Remaining proof gaps:

- repeat x86_64 combined runs for SmolLM2-1.7B Q4_K_M;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence.
