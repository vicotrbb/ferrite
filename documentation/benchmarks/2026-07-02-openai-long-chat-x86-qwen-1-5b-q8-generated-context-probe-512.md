# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Generated-Context 512-Token Probe

## Scope

This run extends the x86_64 generated-context proof set for the larger
`Qwen2.5-1.5B-Instruct-Q8_0` OpenAI-compatible HTTP model artifact to
`--probe-max-tokens 512`. The request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios used the same
512-token budget.

Together with the matching 256-token run, this proves the Q8_0 x86_64
generated assistant-context carry-forward shape at 256 and 512 completion
tokens. It does not close the 1024-token Q8_0 length, SmolLM2-1.7B x86_64
generated-context coverage, EOS-specific behavior, longer steady-state serving,
memory-focused reruns, or 6Gi fit for this Q8_0 path.

## Environment

- Date: 2026-07-02 local time
- Commit: `3a8b4d8429c3e643ae8858532159a66435a77f28`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-genctx-qwen15-q8-512`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.255`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `1Gi`
- Memory limit: `8Gi` (`memory.max=8589934592`)
- Ephemeral-storage request: `8Gi`
- Ephemeral-storage limit: `12Gi`
- EmptyDir size limit: `12Gi`
- Priority class: `homelab-low`
- Rust toolchain: `rustc 1.96.0`, host `x86_64-unknown-linux-gnu`, LLVM
  `22.1.2`
- Model: `Qwen2.5-1.5B-Instruct-Q8_0`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`
- Model size on the local host: `1.8G`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server PID for RSS sampling: `2148`
- Server port inside pod: `127.0.0.1:18155`
- Gate process PID: `2188`
- Server RSS after model load: `1875696` KiB
- Pod cgroup memory current after health check: `3240497152` bytes
- Pod cgroup memory peak after build, model load, and proof: `5252100096`
  bytes
- Pod cgroup memory current after server stop: `1319354368` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw proof log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-512.log`
- Raw proof exit file:
  `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-512.exit`
- Server log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-512-server.log`

The pod-side release binaries were built inside the amd64 pod. `file` reported
both `target/release/ferrite-server` and
`target/release/ferrite-openai-long-chat-gate` as `ELF 64-bit LSB pie
executable, x86-64`.

Binary SHA256 values:

```text
8c199d1728a4a662d237cd7818a7722b8605c28b75dcd26cb116419dee69fac8  target/release/ferrite-server
698f895e374a19fc7acbbc246f5a03d6ee4b7cf4e23ddc4ee8ff825c14b3132d  target/release/ferrite-openai-long-chat-gate
```

Build IDs:

```text
target/release/ferrite-server: BuildID[sha1]=22037f79e16befb073c9c052b67a5162844ecbf2
target/release/ferrite-openai-long-chat-gate: BuildID[sha1]=5d463743e8f56c2d5e351520c2c2cefbf6d7a156
```

## Pod Manifest

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ferrite-avx2-genctx-qwen15-q8-512
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
          memory: "1Gi"
          ephemeral-storage: "8Gi"
        limits:
          cpu: "2"
          memory: "8Gi"
          ephemeral-storage: "12Gi"
      volumeMounts:
        - name: work
          mountPath: /work
  volumes:
    - name: work
      emptyDir:
        sizeLimit: 12Gi
```

The pod used an 8Gi memory limit because prior Q8_0 x86 long-chat work ran
close to the earlier 6Gi pod memory limit. This run is therefore not evidence
that this Q8_0 generated-context path fits a 6Gi limit.

## Build Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen15-q8-512 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

The release build completed successfully:

```text
Finished `release` profile [optimized] target(s) in 38.01s
```

The first pod creation attempt failed because the staging API refused config
lookup; retrying the pod creation succeeded. Direct `kubectl cp` model
transfers were not stable for this 1.8G artifact. A single-file transfer left a
partial 900M model file with SHA256
`064ad694e0dfa5ea7ee38e4fac980e0ce48ec9779faee573c1e853d831ef2b0f`, a second
single-file retry failed, and a 256M chunked transfer failed after the first
chunk. The transfer was completed with 64M chunks from
`/tmp/ferrite-q8-512-chunks-64m`, reassembled in the pod, and verified against
the expected Q8_0 SHA256 above before the chunk directory was removed.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18155 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen15-q8-512 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-1.5B-Instruct-Q8_0 \
    --token-lengths 512 \
    --turns 4 \
    --addr 127.0.0.1:18155 \
    --api-key local-secret \
    --rss-pid 2148 \
    --probe-max-tokens 512 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-512.log 2>&1; \
    echo $? > target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-512.exit'
```

The first background launch wrapper wrote its PID file from the wrong shell
working directory. The gate itself had already started inside `/work/ferrite`;
PID `2188` was recorded afterward from inside the pod. The gate wrote `0` to
`target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-512.exit`.

## Probe Results

Both reconnect/error probes completed with the configured 512-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=512
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=512
```

The disconnect reconnect response included generated stream content and started
a fresh generation after reconnect.

## Scenario Results

All four 512-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 512 completion tokens, token-limit
status, generated-context status, streaming timing, per-token latency summaries,
and RSS samples.

| Turn | Context | Max tokens | Completed | Finish | Prompt tokens | Completion tokens | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 512 | 1 | length | 43 | 512 | 143687 | 513 | 9936 | 141669 | 3.621099 | 220 | 255 | 292 | 9936 | 1956052992 | 1956052992 | 1956052992 |
| 2 | generated | 512 | 1 | length | 553 | 512 | 304416 | 513 | 142022 | 302400 | 1.696428 | 272 | 308 | 361 | 142022 | 1956052992 | 1986985984 | 1986985984 |
| 3 | generated | 512 | 1 | length | 543 | 512 | 303111 | 513 | 139712 | 301095 | 1.703776 | 270 | 307 | 369 | 139712 | 1986985984 | 1986985984 | 1986985984 |
| 4 | generated | 512 | 1 | length | 533 | 512 | 300110 | 513 | 139110 | 298094 | 1.720930 | 268 | 308 | 355 | 139110 | 1986985984 | 1987117056 | 1987117056 |

Each turn reported:

```text
long_chat_result_hit_token_limit=true
```

The generated-context status progressed as intended:

```text
long_chat_result_assistant_context_source=seed
long_chat_result_assistant_context_source=generated
long_chat_result_assistant_context_source=generated
long_chat_result_assistant_context_source=generated
```

The prompt-token count increased from `43` on the seed turn to `553` on the
first generated-context follow-up turn. Turns 3 and 4 reported `543` and `533`
prompt tokens while still using generated assistant context.

## Integrated Summary

```text
long_chat_summary_planned_scenarios=4
long_chat_summary_completed_scenarios=4
long_chat_summary_all_finish_reasons_present=true
long_chat_summary_all_usage_accounting_valid=true
long_chat_summary_all_token_limit_status_present=true
long_chat_summary_any_token_limit_hit=true
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_timing_present=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_error_probe_required=true
long_chat_summary_error_probe_completed=true
long_chat_summary_disconnect_probe_required=true
long_chat_summary_disconnect_probe_completed=true
long_chat_summary_disconnect_probe_reconnect_started_new_generation=true
long_chat_summary_run_complete=true
```

## Cleanup

The server process was stopped after the run. The raw proof log, exit file, and
server log were copied back to local `target/proof/`, then the pod was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-genctx-qwen15-q8-512 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-genctx-qwen15-q8-512 --ignore-not-found
```

The final `get pod` command returned no output, and both staging nodes were
`Ready` after cleanup.

## Interpretation

Qwen2.5-1.5B Q8_0 now has x86_64 AVX2 generated-context long-chat proof at the
256-token and 512-token budgets. The 512-token run proves the same generated
follow-up context, token-limit status, request-error recovery,
disconnect/reconnect recovery, per-token latency summaries, RSS sampling, and
integrated completion as the 256-token run.

The remaining Q8_0 x86_64 generated-context length is 1024 tokens. SmolLM2-1.7B
Q4_K_M x86_64 generated-context coverage, broader EOS behavior, longer
steady-state serving, memory-focused reruns, and 6Gi fit for the current Q8_0
pod shape remain open.
