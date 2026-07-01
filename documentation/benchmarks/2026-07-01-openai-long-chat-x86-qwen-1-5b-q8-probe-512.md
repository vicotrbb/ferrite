# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 512-Token Probe Gate

## Scope

This run extends the x86_64 combined reconnect/error long-chat proof set for
the larger `Qwen2.5-1.5B-Instruct-Q8_0` Tier 1 artifact. It exercises the
OpenAI-compatible HTTP server in a bounded amd64 Kubernetes pod with
`--probe-max-tokens 512`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
512-token budget.

This is one model, one token length, and one bounded x86_64 pod. It does not
close the x86_64 long-chat gate for Qwen2.5-1.5B Q8_0 or for the full Tier 1
HTTP model set.

## Environment

- Date: 2026-07-01
- Commit: `687e091`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-long-chat-qwen15-q8-512`
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
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server PID for RSS sampling: `1684`
- Server port inside pod: `127.0.0.1:18122`
- Gate launcher PID in pod: `1719`
- Gate exit-code file: `target/proof/x86-qwen-1-5b-q8-long-chat-probe-512.exit`
- Gate exit code: `0`
- Pod cgroup memory peak after build and proof: `6335119360` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw log: `target/proof/x86-qwen-1-5b-q8-long-chat-probe-512.log`
- Server log: `target/proof/x86-qwen-1-5b-q8-server-512.log`

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
  name: ferrite-avx2-long-chat-qwen15-q8-512
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
kubectl --context staging exec ferrite-avx2-long-chat-qwen15-q8-512 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 43.67s
```

The cgroup memory peak immediately after the build and before server startup
was `2969403392` bytes.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18122 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 768
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

Initial `ps` RSS for the server process was `1875676` KiB. The pod cgroup
memory peak after model load was `6335119360` bytes, close to but below the
`6Gi` memory limit (`6442450944` bytes).

## Gate Command

The gate ran detached inside the pod and wrote its process exit code to a
separate file, so transient `kubectl exec` stream resets could not terminate
the gate process.

```sh
kubectl --context staging exec ferrite-avx2-long-chat-qwen15-q8-512 -- sh -lc \
  'cd /work/ferrite && rm -f \
    target/proof/x86-qwen-1-5b-q8-long-chat-probe-512.log \
    target/proof/x86-qwen-1-5b-q8-long-chat-probe-512.exit && \
    nohup sh -lc '"'"'./target/release/ferrite-openai-long-chat-gate \
      --execute \
      --error-probe \
      --disconnect-probe \
      --models Qwen2.5-1.5B-Instruct-Q8_0 \
      --token-lengths 512 \
      --turns 4 \
      --addr 127.0.0.1:18122 \
      --api-key local-secret \
      --rss-pid 1684 \
      --probe-max-tokens 512 \
      --expect-finish-reason length \
      > target/proof/x86-qwen-1-5b-q8-long-chat-probe-512.log 2>&1; \
      echo $? > target/proof/x86-qwen-1-5b-q8-long-chat-probe-512.exit'"'"' \
      >/dev/null 2>&1 & echo $!'
```

The exit-code file contained:

```text
0
```

## Probe Results

Both probes completed and recorded the configured 512-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=512
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_max_tokens=512
```

## Scenario Results

All four 512-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 512 completion tokens, streaming
timing, per-token latency summaries, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 512 | 1 | length | 143211 | 513 | 9953 | 141195 | 3.633250 | 218 | 255 | 294 | 9953 | 1955528704 | 1955528704 | 1955528704 |
| 2 | 512 | 1 | length | 143716 | 513 | 9883 | 141700 | 3.620304 | 217 | 257 | 291 | 9883 | 1955528704 | 1955659776 | 1955659776 |
| 3 | 512 | 1 | length | 143531 | 513 | 9949 | 141516 | 3.625018 | 218 | 256 | 291 | 9949 | 1955659776 | 1955921920 | 1955921920 |
| 4 | 512 | 1 | length | 143762 | 513 | 9973 | 141746 | 3.619131 | 218 | 256 | 292 | 9973 | 1955921920 | 1955921920 | 1955921920 |

Usage was stable for every turn:

- prompt tokens: `43`;
- completion tokens: `512`;
- total tokens: `555`.

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

During setup, the API server briefly refused connections and the cluster
reported transient node readiness flaps. The proof itself ran detached inside
the pod, wrote `exit=0`, and reached `long_chat_summary_run_complete=true`.
The pod stayed `Running` with zero restarts before cleanup.

This evidence is valid for the Ferrite server/gate behavior recorded in the
in-pod log, but it should not be used as a clean staging
control-plane-stability sample. It also shows that the Q8_0 512-token x86
server path runs close to the current `6Gi` pod memory limit.

## Cleanup

The server process was stopped after the run. The raw proof log, server log,
and gate exit-code file were copied back to local `target/proof/`, then the pod
was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-long-chat-qwen15-q8-512 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-long-chat-qwen15-q8-512 --ignore-not-found
```

The delete command completed, and the final `get pod` command returned no
output.

## Interpretation

Ferrite now has real x86_64 AVX2 combined long-chat reconnect/error proof for
Qwen2.5-1.5B Q8_0 at the 256 and 512 completion-token budgets.

Remaining proof gaps:

- repeat this x86_64 shape for Qwen2.5-1.5B Q8_0 at 1024 completion tokens;
- repeat x86_64 combined runs for Qwen2.5-1.5B Q6_K and SmolLM2-1.7B Q4_K_M;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence.
