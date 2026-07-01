# OpenAI Long-Chat x86_64 Qwen 0.5B 512-Token Probe Gate

## Scope

This run extends the x86_64 portion of the combined reconnect/error long-chat
proof gate for `Qwen2.5-0.5B-Instruct-Q4_K_M`. It exercises the
OpenAI-compatible HTTP server in a bounded amd64 Kubernetes pod with
`--probe-max-tokens 512`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
512-token budget.

This is one model, one token length, and one bounded x86_64 pod. It does not
close the x86_64 long-chat gate for the full Tier 1 HTTP model set.

## Environment

- Date: 2026-07-01
- Commit: `a7e3d43`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-long-chat-qwen05-512`
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
- Server port inside pod: `127.0.0.1:18119`
- Pod cgroup memory peak after build and proof: `1469911040` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `538M`
- Raw log: `target/proof/x86-qwen-0-5b-q4-long-chat-probe-512.log`
- Server log: `target/proof/x86-qwen-0-5b-server-512.log`

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
  name: ferrite-avx2-long-chat-qwen05-512
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
kubectl --context staging exec ferrite-avx2-long-chat-qwen05-512 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 37.45s
```

The cgroup memory peak immediately after the build and before the proof was
`1053179904` bytes.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18119 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 768
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

Initial `ps` RSS for the server process was `413916` KiB.

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-long-chat-qwen05-512 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-0.5B-Instruct-Q4_K_M \
    --token-lengths 512 \
    --turns 4 \
    --addr 127.0.0.1:18119 \
    --api-key local-secret \
    --rss-pid 1684 \
    --probe-max-tokens 512 \
    --expect-finish-reason length | tee target/proof/x86-qwen-0-5b-q4-long-chat-probe-512.log'
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
| 1 | 512 | 1 | length | 186951 | 513 | 13349 | 184935 | 2.773937 | 297 | 324 | 416 | 13349 | 446918656 | 447574016 | 447574016 |
| 2 | 512 | 1 | length | 188088 | 513 | 13582 | 186074 | 2.756965 | 298 | 330 | 400 | 13582 | 447574016 | 447574016 | 447574016 |
| 3 | 512 | 1 | length | 184438 | 513 | 13420 | 182423 | 2.812137 | 297 | 327 | 360 | 13420 | 447574016 | 447836160 | 447836160 |
| 4 | 512 | 1 | length | 183635 | 513 | 13639 | 181619 | 2.824590 | 295 | 326 | 357 | 13639 | 447836160 | 447836160 | 447836160 |

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

During the run, two `kubectl exec` streams used for interactive observation
reset while the in-pod gate and server processes kept running. Kubernetes node
events also showed transient `NodeNotReady` and `Starting kubelet` events on
the staging control-plane nodes. The proof log itself was written by the
in-pod gate process and reached `long_chat_summary_run_complete=true`.

This evidence is therefore valid for the Ferrite server/gate behavior recorded
in the in-pod log, but it should not be used as a clean staging
control-plane-stability sample.

## Cleanup

The server process was stopped after the run. The raw proof log and server log
were copied back to local `target/proof/`, then the pod was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-long-chat-qwen05-512 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-long-chat-qwen05-512 --ignore-not-found
```

The delete command completed, and the final `get pod` command returned no
output.

## Interpretation

Ferrite now has real x86_64 AVX2 combined long-chat reconnect/error proof for
Qwen2.5-0.5B Q4_K_M at both 256 and 512 completion-token budgets.

Remaining proof gaps:

- repeat this x86_64 shape for the 1024 completion-token budget;
- repeat x86_64 combined runs for Qwen2.5-1.5B Q8_0, Qwen2.5-1.5B Q6_K, and
  SmolLM2-1.7B Q4_K_M;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence.
