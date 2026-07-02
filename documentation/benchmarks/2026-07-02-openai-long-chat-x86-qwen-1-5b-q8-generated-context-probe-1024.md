# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Generated-Context 1024-Token Probe

## Scope

This run closes the x86_64 generated-context proof set for the larger
`Qwen2.5-1.5B-Instruct-Q8_0` OpenAI-compatible HTTP model artifact at the
1024-token length. It used a bounded amd64 Kubernetes pod with
`--probe-max-tokens 1024`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios used the same
1024-token budget.

Together with the earlier 256-token and 512-token runs, this proves the
generated assistant-context carry-forward shape for this Q8_0 x86_64 slice at
256, 512, and 1024 completion tokens. It does not close x86_64
generated-context coverage for SmolLM2-1.7B, broader EOS-specific behavior,
longer steady-state serving, memory-focused reruns, or 6Gi fit for this Q8_0
pod shape.

## Environment

- Date: 2026-07-02 local time
- Commit: `e5c91853751605a4fad301fc821b59da7a00a791`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-genctx-qwen15-q8-1024`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.246`
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
- Model size on the local host and in the pod: `1.8G`
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server PID for RSS sampling: `2088`
- Server port inside pod: `127.0.0.1:18156`
- Gate process PID: `2135`
- Server RSS after model load: `1875744` KiB
- Pod cgroup memory current after health check: `2028441600` bytes
- Pod cgroup memory peak after build, model load, and proof: `3928395776`
  bytes
- Pod cgroup memory current after server stop: `106860544` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw proof log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-1024.log`
- Raw proof exit file:
  `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-1024.exit`
- Server log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-1024-server.log`

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
  name: ferrite-avx2-genctx-qwen15-q8-1024
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

## Build And Transfer

The first release-build `kubectl exec` stream reset while Cargo was compiling,
but the in-pod build process survived. A follow-up detached build wrapper waited
on the Cargo build directory lock and wrote exit code `0` after the release
build completed:

```text
Blocking waiting for file lock on build directory
Finished `release` profile [optimized] target(s) in 5.94s
```

The Q8_0 model was transferred as 29 local 64M chunks from
`/tmp/ferrite-q8-1024-chunks-64m`. One chunk copy hit
`context deadline exceeded` at `q8.part.as`; the retry loop copied the chunk on
the next attempt. The in-pod reassembled model SHA256 matched the expected Q8_0
hash above, and the chunk directory was removed before the proof run.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18156 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 2048
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen15-q8-1024 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-1.5B-Instruct-Q8_0 \
    --token-lengths 1024 \
    --turns 4 \
    --addr 127.0.0.1:18156 \
    --api-key local-secret \
    --rss-pid 2088 \
    --probe-max-tokens 1024 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-1024.log 2>&1; \
    echo $? > target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-1024.exit'
```

The first background launch wrapper wrote its PID file from the wrong shell
working directory. The gate itself had already started inside `/work/ferrite`;
PID `2135` was recorded afterward from inside the pod. The gate wrote `0` to
`target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-1024.exit`.

During the long run, the staging control plane intermittently returned
connection refusals, an etcd request timeout, an `apiserver not ready` response,
and kubelet proxy `502 Bad Gateway` errors for exec checks. The pod remained
`Running` with zero observed restarts during the run, the detached in-pod gate
continued, and the proof completed. These are recorded as staging control-plane
instability, not as in-pod Ferrite proof failures.

## Probe Results

Both reconnect/error probes completed with the configured 1024-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=1024
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=1024
```

The disconnect reconnect response included generated stream content and started
a fresh generation after reconnect.

## Scenario Results

All four 1024-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 1024 completion tokens,
token-limit status, generated-context status, streaming timing, per-token
latency summaries, and RSS samples.

| Turn | Context | Max tokens | Completed | Finish | Prompt tokens | Completion tokens | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 1024 | 1 | length | 43 | 1024 | 304614 | 1025 | 10037 | 302590 | 3.387412 | 220 | 284 | 343 | 10037 | 1995456512 | 1987674112 | 1987674112 |
| 2 | generated | 1024 | 1 | length | 1080 | 1024 | 738636 | 1025 | 309182 | 736605 | 1.391519 | 323 | 407 | 515 | 309182 | 1987674112 | 2048884736 | 2048884736 |
| 3 | generated | 1024 | 1 | length | 1054 | 1024 | 721161 | 1025 | 305399 | 719134 | 1.425325 | 318 | 399 | 488 | 305399 | 2048884736 | 2060681216 | 2060681216 |
| 4 | generated | 1024 | 1 | length | 1054 | 1024 | 717430 | 1025 | 304311 | 715409 | 1.432747 | 320 | 395 | 489 | 304311 | 2060681216 | 2041733120 | 2041733120 |

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

The prompt-token count increased from `43` on the seed turn to `1080` on the
first generated-context follow-up turn. Turns 3 and 4 reported `1054` prompt
tokens while still using generated assistant context.

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
kubectl --context staging delete pod ferrite-avx2-genctx-qwen15-q8-1024 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-genctx-qwen15-q8-1024 --ignore-not-found
```

The final `get pod` command returned no output, and both staging nodes were
`Ready` after cleanup.

## Interpretation

Qwen2.5-1.5B Q8_0 now has x86_64 AVX2 generated-context long-chat proof at the
256-token, 512-token, and 1024-token budgets. The 1024-token run proves the
same generated follow-up context, token-limit status, request-error recovery,
disconnect/reconnect recovery, per-token latency summaries, RSS sampling, and
integrated completion as the smaller Q8_0 generated-context x86_64 runs.

SmolLM2-1.7B Q4_K_M x86_64 generated-context coverage, broader EOS behavior,
longer steady-state serving, memory-focused reruns, and 6Gi fit for the
current Q8_0 pod shape remain open.
