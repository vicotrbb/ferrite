# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Stop Gate

## Scope

This run adds x86_64 stop-focused evidence for the larger
`Qwen2.5-1.5B-Instruct-Q8_0` Tier 1 artifact. It exercises Ferrite's
OpenAI-compatible streaming chat path in a bounded amd64 Kubernetes pod with an
explicit OpenAI `stop` sequence and an expected `finish_reason=stop`.

This is one model, one known prompt shape, and one bounded x86_64 pod. It does
not close the stop/EOS portion of the long-chat gate for the full Tier 1 HTTP
model set.

## Environment

- Date: 2026-07-01
- Commit: `ec82d62959659ac1e18f23738d29e752313488ae`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-stop-qwen15-q8`
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
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server PID for RSS sampling: `1657`
- Server port inside pod: `127.0.0.1:18131`
- Release build cgroup memory peak: `2320613376` bytes
- Post-health cgroup memory peak: `4660981760` bytes
- Pod cgroup memory peak after proof: `4660981760` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw probe log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-stop-probe.log`
- Raw server log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-stop.log`

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
  name: ferrite-avx2-stop-qwen15-q8
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
kubectl --context staging exec ferrite-avx2-stop-qwen15-q8 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite; cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 47.93s
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18131 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 256
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

Post-health process and cgroup samples:

```text
PID 1657, RSS 1876220 KiB
memory_current=2753060864
memory_peak=4660981760
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-stop-qwen15-q8 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-1.5B-Instruct-Q8_0 \
    --token-lengths 1 \
    --turns 4 \
    --addr 127.0.0.1:18131 \
    --api-key local-secret \
    --rss-pid 1657 \
    --prompt "hello world" \
    --assistant-context "short context" \
    --follow-up "hello world" \
    --stop "你" \
    --expect-finish-reason stop'
```

The process list rendered the non-ASCII stop sequence as `???` under the pod's
locale, but the gate completed successfully with `finish_reason=stop`. The
gate process wrote `0` to
`target/proof/x86-qwen-1-5b-q8-long-chat-stop-probe.exit`.

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

All four streaming chat scenarios completed with `finish_reason=stop`, one
streaming token event, valid usage accounting, timing summaries, and RSS
samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 1 | stop | 6177 | 1 | 4165 | 4165 | 0.240095 | 4165 | 4165 | 4165 | 4165 | 1931603968 | 1931603968 | 1931603968 |
| 2 | 1 | 1 | stop | 6183 | 1 | 4170 | 4170 | 0.239794 | 4170 | 4170 | 4170 | 4170 | 1931603968 | 1931735040 | 1931735040 |
| 3 | 1 | 1 | stop | 6247 | 1 | 4234 | 4234 | 0.236128 | 4234 | 4234 | 4234 | 4234 | 1931735040 | 1931735040 | 1931735040 |
| 4 | 1 | 1 | stop | 6534 | 1 | 4522 | 4522 | 0.221102 | 4522 | 4522 | 4522 | 4522 | 1931735040 | 1931735040 | 1931735040 |

Usage was stable for every turn:

- prompt tokens: `18`;
- completion tokens: `1`;
- total tokens: `19`.

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
kubectl --context staging delete pod ferrite-avx2-stop-qwen15-q8 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-stop-qwen15-q8 --ignore-not-found
```

The final `get pod` command returned no output. Final node check showed both
`homelab-01` and `homelab-02` as `Ready`.

## Interpretation

Ferrite now has a real x86_64 AVX2 long-chat stop proof for the larger Qwen
1.5B Q8 artifact. The run proves the OpenAI-compatible streaming chat path can
finish through an explicit stop sequence with repeated four-turn shape,
request-error recovery, disconnect/reconnect recovery, timing summaries, RSS
samples, and `long_chat_summary_run_complete=true`.

Remaining proof gaps:

- repeat x86_64 stop-focused long-chat runs for Qwen2.5-1.5B Q6_K and
  SmolLM2-1.7B Q4_K_M;
- add EOS-specific evidence rather than only explicit stop-sequence evidence;
- run longer steady-state serving and memory-focused samples;
- keep the full OpenAI-compatible long-chat gate open until all required
  models have length, stop/EOS, reconnect/error, latency, and RSS evidence.
