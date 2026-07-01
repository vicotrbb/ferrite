# OpenAI Long-Chat x86_64 SmolLM 1.7B Q4 Stop Gate

## Scope

This run adds x86_64 stop-focused evidence for
`SmolLM2-1.7B-Instruct-Q4_K_M`. It exercises Ferrite's OpenAI-compatible
streaming chat path in a bounded amd64 Kubernetes pod with an explicit OpenAI
`stop` sequence and an expected `finish_reason=stop`.

This is one model, one known prompt shape, and one bounded x86_64 pod. It
closes the explicit-stop x86_64 coverage gap for the current Tier 1
long-chat model set, but it does not close EOS-specific, steady-state, or
memory-focused proof gaps.

## Environment

- Date: 2026-07-01
- Commit: `fb4f107f8371f852527d78723abe9ccb74c7fe57`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-stop-smollm17-q4`
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
- Rust toolchain: `rustc 1.96.0`, `cargo 1.96.0`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path in pod: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`
- Server PID for RSS sampling: `1658`
- Server port inside pod: `127.0.0.1:18133`
- Release build cgroup memory peak: `2106847232` bytes
- Post-health cgroup memory peak: `3838607360` bytes
- Pod cgroup memory peak after proof: `3838607360` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `1.2G`
- Raw probe log:
  `target/proof/x86-smollm-1-7b-q4-long-chat-stop-probe.log`
- Raw server log:
  `target/proof/x86-smollm-1-7b-q4-long-chat-stop.log`

The pod-side release binaries were built inside the amd64 pod. `file` reported
both `target/release/ferrite-server` and
`target/release/ferrite-openai-long-chat-gate` as `ELF 64-bit LSB pie
executable, x86-64`.

Release binary SHA256 values:

```text
e5f148ec2c6686d532ac1ea37c48abf6f9ba7c3a4ba46c1ffae28a98ebed261a  target/release/ferrite-server
f613e12832ee0d9ccad126a8ab900e2fe0ee6e8612c181b3d7de137264a8ff24  target/release/ferrite-openai-long-chat-gate
```

## Pod Manifest

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ferrite-avx2-stop-smollm17-q4
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
kubectl --context staging exec ferrite-avx2-stop-smollm17-q4 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite; cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 37.67s
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18133 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256
```

Health check response:

```json
{"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

Post-health process and cgroup samples:

```text
PID 1658, RSS 1042004 KiB
memory_current=2777169920
memory_peak=3838607360
```

## Stop Sequence Discovery

The known single-message SmolLM chat stop sequence is `1`. That did not hold
for the long-chat three-message shape used by the gate:

```text
expected finish_reason stop, got length
```

A direct non-streaming chat probe without `stop` for the same message shape and
`max_tokens=4` returned:

```json
{"content":"\nuser: hello","finish_reason":"length"}
```

A follow-up direct probe with `stop: "user"` and `max_tokens=4` returned
`finish_reason: "stop"` with the visible content trimmed to `"\n"`. The
passing gate therefore uses the prompt-specific `user` stop sequence and a
four-token budget.

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-stop-smollm17-q4 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models SmolLM2-1.7B-Instruct-Q4_K_M \
    --token-lengths 4 \
    --turns 4 \
    --addr 127.0.0.1:18133 \
    --api-key local-secret \
    --rss-pid 1658 \
    --prompt "hello world" \
    --assistant-context "short context" \
    --follow-up "hello world" \
    --stop "user" \
    --expect-finish-reason stop'
```

The gate process wrote `0` to
`target/proof/x86-smollm-1-7b-q4-long-chat-stop-probe.exit`.

## Probe Results

Both probes completed:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=1
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_max_tokens=8
```

The disconnect probe starts a new bounded reconnect request rather than
resuming the abandoned stream.

## Scenario Results

All four streaming chat scenarios completed with `finish_reason=stop`, two
streaming token events, valid usage accounting, timing summaries, and RSS
samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 4 | 1 | stop | 19320 | 2 | 16469 | 17308 | 0.115552 | 839 | 839 | 16469 | 16469 | 1073307648 | 1072902144 | 1072902144 |
| 2 | 4 | 1 | stop | 19387 | 2 | 16518 | 17376 | 0.115099 | 858 | 858 | 16518 | 16518 | 1072902144 | 1072791552 | 1072791552 |
| 3 | 4 | 1 | stop | 19336 | 2 | 16517 | 17321 | 0.115460 | 804 | 804 | 16517 | 16517 | 1072791552 | 1073074176 | 1073074176 |
| 4 | 4 | 1 | stop | 19391 | 2 | 16571 | 17378 | 0.115085 | 807 | 807 | 16571 | 16571 | 1073074176 | 1072881664 | 1072881664 |

Usage was stable for every turn:

- prompt tokens: `20`;
- completion tokens: `2`;
- total tokens: `22`.

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
kubectl --context staging delete pod ferrite-avx2-stop-smollm17-q4 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-stop-smollm17-q4 --ignore-not-found
```

The final `get pod` command returned no output. Final node check showed both
`homelab-01` and `homelab-02` as `Ready`.

## Interpretation

Ferrite now has a real x86_64 AVX2 long-chat stop proof for
SmolLM2-1.7B Q4_K_M. Together with the Qwen2.5-0.5B Q4_K_M, Qwen2.5-1.5B
Q8_0, and Qwen2.5-1.5B Q6_K stop runs, the current required Tier 1 HTTP model
set has bounded x86_64 explicit-stop long-chat coverage.

Remaining proof gaps:

- add EOS-specific evidence rather than only explicit stop-sequence evidence;
- run longer steady-state serving and memory-focused samples;
- keep the full OpenAI-compatible long-chat gate open until all required
  models have length, stop/EOS, reconnect/error, latency, and RSS evidence.
