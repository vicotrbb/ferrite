# OpenAI Long-Chat x86_64 Qwen 0.5B 256-Token Probe Gate

## Scope

This run starts the x86_64 portion of the combined reconnect/error long-chat
proof gate. It exercises `Qwen2.5-0.5B-Instruct-Q4_K_M` in a bounded amd64
Kubernetes pod with `--probe-max-tokens 256`, so the request-error reconnect
path, disconnect reconnect path, and all repeated streaming chat scenarios use
the same 256-token budget.

This is one model, one token length, and one bounded x86_64 pod. It does not
close the x86_64 long-chat gate for the full Tier 1 HTTP model set.

## Environment

- Date: 2026-07-01
- Commit: `40750b8`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-long-chat-qwen05-256`
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
- Server PID for RSS sampling: `1718`
- Server port inside pod: `127.0.0.1:18118`
- Pod cgroup memory peak after build and proof: `1964257280` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `538M`
- Raw log: `target/proof/x86-qwen-0-5b-q4-long-chat-probe-256.log`

The pod-side release binaries were built inside the amd64 pod. `file` reported
both `target/release/ferrite-server` and
`target/release/ferrite-openai-long-chat-gate` as `ELF 64-bit LSB pie
executable, x86-64`.

## Pod Manifest

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ferrite-avx2-long-chat-qwen05-256
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
kubectl --context staging exec ferrite-avx2-long-chat-qwen05-256 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 42.61s
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18118 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-long-chat-qwen05-256 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-0.5B-Instruct-Q4_K_M \
    --token-lengths 256 \
    --turns 4 \
    --addr 127.0.0.1:18118 \
    --api-key local-secret \
    --rss-pid 1718 \
    --probe-max-tokens 256 \
    --expect-finish-reason length | tee target/proof/x86-qwen-0-5b-q4-long-chat-probe-256.log'
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
| 1 | 256 | 1 | length | 97193 | 257 | 13472 | 95178 | 2.700184 | 295 | 317 | 343 | 13472 | 439906304 | 440037376 | 440037376 |
| 2 | 256 | 1 | length | 97519 | 257 | 13440 | 95506 | 2.690906 | 295 | 317 | 344 | 13440 | 440037376 | 440168448 | 440168448 |
| 3 | 256 | 1 | length | 97564 | 257 | 13651 | 95550 | 2.689685 | 298 | 318 | 344 | 13651 | 440168448 | 440430592 | 440430592 |
| 4 | 256 | 1 | length | 97808 | 257 | 13888 | 95795 | 2.682796 | 297 | 317 | 345 | 13888 | 440430592 | 440430592 | 440430592 |

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

## Cleanup

The server process was stopped after the run. The raw proof log and server log
were copied back to local `target/proof/`, then the pod was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-long-chat-qwen05-256 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-long-chat-qwen05-256 --ignore-not-found
```

The final `get pod` command returned no output.

## Interpretation

Ferrite now has one real x86_64 AVX2 combined long-chat reconnect/error proof
for the OpenAI-compatible server path: Qwen2.5-0.5B Q4_K_M at the 256
completion-token budget.

Remaining proof gaps:

- repeat this x86_64 shape for 512 and 1024 completion-token budgets;
- repeat x86_64 combined runs for Qwen2.5-1.5B Q8_0, Qwen2.5-1.5B Q6_K, and
  SmolLM2-1.7B Q4_K_M;
- run longer steady-state serving and memory-focused samples;
- broaden EOS-specific evidence.
