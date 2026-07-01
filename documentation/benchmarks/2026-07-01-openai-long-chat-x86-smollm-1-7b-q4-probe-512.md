# OpenAI Long-Chat x86_64 SmolLM2 1.7B Q4 512-Token Probe Gate

## Scope

This run extends the x86_64 combined reconnect/error long-chat proof set for
the `SmolLM2-1.7B-Instruct-Q4_K_M` Tier 1 artifact. It exercises the
OpenAI-compatible HTTP server in a bounded amd64 Kubernetes pod with
`--probe-max-tokens 512`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
512-token budget.

This is one model, one token length, and one bounded x86_64 pod. It proves the
512-token x86_64 long-chat budget for SmolLM2-1.7B Q4_K_M. The 1024-token
x86_64 SmolLM2 budget remains unproven, and this run does not close the full
x86_64 Tier 1 HTTP long-chat gate.

## Environment

- Date: 2026-07-01
- Commit: `2fd1c85`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-long-chat-smollm17-q4-512`
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
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path in pod: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`
- Server PID for RSS sampling: `1668`
- Server port inside pod: `127.0.0.1:18128`
- Gate launcher PID in pod: `1696`
- Gate process PID in pod: `1701`
- Gate exit-code file:
  `target/proof/x86-smollm-1-7b-q4-long-chat-probe-512.exit`
- Gate exit code: `0`
- Server initial RSS after model load: `1041724` KiB
- Pod cgroup memory peak after model load: `2713841664` bytes
- Pod cgroup memory peak after build and proof: `2713841664` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `1.2G`
- Raw log: `target/proof/x86-smollm-1-7b-q4-long-chat-probe-512.log`
- Server log: `target/proof/x86-smollm-1-7b-q4-server-512.log`

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
  name: ferrite-avx2-long-chat-smollm17-q4-512
spec:
  restartPolicy: Never
  priorityClassName: homelab-low
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
kubectl --context staging exec ferrite-avx2-long-chat-smollm17-q4-512 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 36.81s
```

The cgroup memory peak immediately after the build and before server startup
was `1707372544` bytes.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18128 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 768
```

Health check response:

```json
{"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

Initial `ps` RSS for the server process was `1041724` KiB. The pod cgroup
memory peak after model load was `2713841664` bytes, below the `6Gi` memory
limit.

## Gate Command

The gate used the known full-length SmolLM2 operational prompt shape from the
earlier local SmolLM2 full matrix. The default short prompt can terminate early
for this model, so this run used explicit prompt, assistant context, and
follow-up text while still expecting `finish_reason=length`.

```sh
kubectl --context staging exec ferrite-avx2-long-chat-smollm17-q4-512 -- sh -lc \
  'cd /work/ferrite && nohup sh -lc '"'"'./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models SmolLM2-1.7B-Instruct-Q4_K_M \
    --token-lengths 512 \
    --turns 4 \
    --addr 127.0.0.1:18128 \
    --api-key local-secret \
    --rss-pid 1668 \
    --probe-max-tokens 512 \
    --expect-finish-reason length \
    --prompt "Write a concise operational note about CPU inference stability." \
    --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
    --follow-up "Continue with reconnect and error-handling risks." \
    > target/proof/x86-smollm-1-7b-q4-long-chat-probe-512.log 2>&1; \
    echo $? > target/proof/x86-smollm-1-7b-q4-long-chat-probe-512.exit'"'"' \
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
| 1 | 512 | 1 | length | 493070 | 513 | 43731 | 491056 | 1.044687 | 791 | 863 | 955 | 43731 | 1269768192 | 1270091776 | 1270091776 |
| 2 | 512 | 1 | length | 492207 | 513 | 44258 | 490192 | 1.046527 | 805 | 867 | 935 | 44258 | 1270091776 | 1270079488 | 1270079488 |
| 3 | 512 | 1 | length | 491774 | 513 | 43817 | 489759 | 1.047453 | 791 | 867 | 933 | 43817 | 1270079488 | 1270099968 | 1270099968 |
| 4 | 512 | 1 | length | 497600 | 513 | 43862 | 495584 | 1.035141 | 797 | 871 | 982 | 43862 | 1270099968 | 1270374400 | 1270374400 |

Usage was stable for every turn:

- prompt tokens: `53`;
- completion tokens: `512`;
- total tokens: `565`.

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

## Staging Notes

The run completed without observed staging node-readiness or exec-stream
failures. The proof ran detached inside the pod, wrote `exit=0`, and reached
`long_chat_summary_run_complete=true`. The pod stayed `Running` with zero
restarts before cleanup.

## Cleanup

The server process was stopped after the run. The raw proof log, server log,
and gate exit-code file were copied back to local `target/proof/`, then the pod
was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-long-chat-smollm17-q4-512 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-long-chat-smollm17-q4-512 --ignore-not-found
kubectl --context staging get nodes -o wide
```

Final node check showed both `homelab-01` and `homelab-02` `Ready`.

## Conclusion

Ferrite now has real x86_64 AVX2 combined long-chat reconnect/error proof for
the OpenAI-compatible server path on `SmolLM2-1.7B-Instruct-Q4_K_M` at the
256-token and 512-token budgets. The 1024-token x86_64 SmolLM2 budget remains
unproven.
