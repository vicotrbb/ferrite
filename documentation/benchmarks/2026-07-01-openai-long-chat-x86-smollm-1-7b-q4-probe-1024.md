# OpenAI Long-Chat x86_64 SmolLM2 1.7B Q4 1024-Token Probe Gate

## Scope

This run completes the x86_64 combined reconnect/error long-chat proof set for
the `SmolLM2-1.7B-Instruct-Q4_K_M` Tier 1 artifact. It exercises the
OpenAI-compatible HTTP server in a bounded amd64 Kubernetes pod with
`--probe-max-tokens 1024`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
1024-token budget.

This is one model, one token length, and one bounded x86_64 pod. It completes
the 256/512/1024 x86_64 long-chat budget set for SmolLM2-1.7B Q4_K_M. Together
with the prior x86 Qwen runs, it closes the required x86_64 combined
reconnect/error long-chat token matrix for the four current Tier 1 HTTP model
artifacts. It is not a release-complete readiness claim.

## Environment

- Date: 2026-07-01
- Commit: `29970ab`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-long-chat-smollm17-q4-1024`
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
- Server PID for RSS sampling: `1658`
- Server port inside pod: `127.0.0.1:18129`
- Gate launcher PID in pod: `1686`
- Gate process PID in pod: `1691`
- Gate exit-code file:
  `target/proof/x86-smollm-1-7b-q4-long-chat-probe-1024.exit`
- Gate exit code: `0`
- Server initial RSS after model load: `1041688` KiB
- Pod cgroup memory peak after model load: `2501103616` bytes
- Pod cgroup memory peak after build and proof: `2501103616` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `1.2G`
- Raw log: `target/proof/x86-smollm-1-7b-q4-long-chat-probe-1024.log`
- Server log: `target/proof/x86-smollm-1-7b-q4-server-1024.log`

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
  name: ferrite-avx2-long-chat-smollm17-q4-1024
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
kubectl --context staging exec ferrite-avx2-long-chat-smollm17-q4-1024 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 41.85s
```

The cgroup memory peak immediately after the build and before server startup
was `1707163648` bytes.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18129 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 1280
```

Health check response:

```json
{"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

Initial `ps` RSS for the server process was `1041688` KiB. The pod cgroup
memory peak after model load was `2501103616` bytes, below the `6Gi` memory
limit.

## Gate Command

The gate used the known full-length SmolLM2 operational prompt shape from the
earlier local SmolLM2 full matrix. The default short prompt can terminate early
for this model, so this run used explicit prompt, assistant context, and
follow-up text while still expecting `finish_reason=length`.

```sh
kubectl --context staging exec ferrite-avx2-long-chat-smollm17-q4-1024 -- sh -lc \
  'cd /work/ferrite && nohup sh -lc '"'"'./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models SmolLM2-1.7B-Instruct-Q4_K_M \
    --token-lengths 1024 \
    --turns 4 \
    --addr 127.0.0.1:18129 \
    --api-key local-secret \
    --rss-pid 1658 \
    --probe-max-tokens 1024 \
    --expect-finish-reason length \
    --prompt "Write a concise operational note about CPU inference stability." \
    --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
    --follow-up "Continue with reconnect and error-handling risks." \
    > target/proof/x86-smollm-1-7b-q4-long-chat-probe-1024.log 2>&1; \
    echo $? > target/proof/x86-smollm-1-7b-q4-long-chat-probe-1024.exit'"'"' \
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
| 1 | 1024 | 1 | length | 990803 | 1025 | 49521 | 988730 | 1.036682 | 806 | 908 | 1030 | 49521 | 1470955520 | 1471049728 | 1471049728 |
| 2 | 1024 | 1 | length | 992512 | 1025 | 44594 | 990491 | 1.034840 | 806 | 918 | 1022 | 44594 | 1471049728 | 1470750720 | 1470750720 |
| 3 | 1024 | 1 | length | 988567 | 1025 | 44298 | 986549 | 1.038975 | 803 | 917 | 1010 | 44298 | 1470750720 | 1471029248 | 1471029248 |
| 4 | 1024 | 1 | length | 999652 | 1025 | 44144 | 997631 | 1.027434 | 804 | 922 | 1051 | 44144 | 1471029248 | 1471127552 | 1471127552 |

Usage was stable for every turn:

- prompt tokens: `53`;
- completion tokens: `1024`;
- total tokens: `1077`.

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
kubectl --context staging delete pod ferrite-avx2-long-chat-smollm17-q4-1024 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-long-chat-smollm17-q4-1024 --ignore-not-found
kubectl --context staging get nodes -o wide
```

Final node check showed both `homelab-01` and `homelab-02` `Ready`.

## Conclusion

Ferrite now has real x86_64 AVX2 combined long-chat reconnect/error proof for
the OpenAI-compatible server path on `SmolLM2-1.7B-Instruct-Q4_K_M` at the
256-token, 512-token, and 1024-token budgets. This completes the x86_64
combined reconnect/error long-chat token matrix for the required Tier 1 HTTP
model artifacts. It does not prove broader EOS-specific long-chat behavior,
longer steady-state serving, high concurrency, or release-grade readiness.
