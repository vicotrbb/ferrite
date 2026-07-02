# OpenAI Long-Chat x86_64 Qwen 0.5B Generated-Context 1024-Token Probe

## Scope

This run completes the x86_64 generated-context token-length slice for
`Qwen2.5-0.5B-Instruct-Q4_K_M` across 256, 512, and 1024 completion tokens. It
uses a bounded amd64 Kubernetes pod with `--probe-max-tokens 1024`, so the
request-error reconnect path, disconnect reconnect path, and all repeated
streaming chat scenarios use the same 1024-token budget.

This is one model and one bounded x86_64 pod. It closes the Qwen2.5-0.5B
Q4_K_M x86_64 generated-context length set, but it does not close the x86_64
generated-context matrix for the larger required artifacts, EOS-specific
behavior, longer steady-state serving, or memory-focused reruns.

## Environment

- Date: 2026-07-02 local time
- UTC timestamp during closeout: `2026-07-02 04:20:48 UTC`
- Commit: `11f08e82aeb879df234ef77af0660a35eda215ee`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-genctx-qwen05-1024`
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
- Priority class: `homelab-low`
- Rust toolchain: `rustc 1.96.0`, host `x86_64-unknown-linux-gnu`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path in pod: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Server PID for RSS sampling: `1680`
- Server port inside pod: `127.0.0.1:18150`
- Gate launcher PID: `1709`
- Gate process PID: `1713`
- Pod cgroup memory peak after build and proof: `1451917312` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `539M`
- Raw proof log:
  `target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-1024.log`
- Raw proof exit file:
  `target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-1024.exit`
- Server log:
  `target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-1024-server.log`

The pod-side release binaries were built inside the amd64 pod. `file` reported
both `target/release/ferrite-server` and
`target/release/ferrite-openai-long-chat-gate` as `ELF 64-bit LSB pie
executable, x86-64`.

Binary SHA256 values:

```text
8c199d1728a4a662d237cd7818a7722b8605c28b75dcd26cb116419dee69fac8  target/release/ferrite-server
698f895e374a19fc7acbbc246f5a03d6ee4b7cf4e23ddc4ee8ff825c14b3132d  target/release/ferrite-openai-long-chat-gate
```

## Pod Manifest

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ferrite-avx2-genctx-qwen05-1024
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
kubectl --context staging exec ferrite-avx2-genctx-qwen05-1024 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

The first build attempt failed before compiling Ferrite because Cargo timed out
while downloading `async-openai` from crates.io:

```text
curl failed
[28] Timeout was reached (Operation too slow. Less than 10 bytes/sec transferred the last 30 seconds)
```

The same build command was retried in the same pod and completed successfully:

```text
Finished `release` profile [optimized] target(s) in 1m 23s
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18150 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 2048
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen05-1024 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-0.5B-Instruct-Q4_K_M \
    --token-lengths 1024 \
    --turns 4 \
    --addr 127.0.0.1:18150 \
    --api-key local-secret \
    --rss-pid 1680 \
    --probe-max-tokens 1024 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-1024.log 2>&1; \
    echo $? > target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-1024.exit'
```

The gate wrote `0` to
`target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-1024.exit`.

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
`finish_reason=length`, usage accounting for 1024 completion tokens, token-limit
status, generated-context status, streaming timing, per-token latency summaries,
and RSS samples.

| Turn | Context | Max tokens | Completed | Finish | Prompt tokens | Completion tokens | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 1024 | 1 | length | 43 | 1024 | 371743 | 1025 | 13598 | 369715 | 2.772399 | 299 | 342 | 400 | 13598 | 454615040 | 456712192 | 456712192 |
| 2 | generated | 1024 | 1 | length | 1054 | 1024 | 792861 | 1025 | 364288 | 790841 | 1.296087 | 353 | 411 | 469 | 364288 | 456712192 | 484368384 | 484368384 |
| 3 | generated | 1024 | 1 | length | 1054 | 1024 | 795332 | 1025 | 364043 | 793312 | 1.292050 | 356 | 408 | 496 | 364043 | 484368384 | 484499456 | 484499456 |
| 4 | generated | 1024 | 1 | length | 1054 | 1024 | 788147 | 1025 | 363609 | 786126 | 1.303861 | 358 | 410 | 464 | 363609 | 484499456 | 484630528 | 484630528 |

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

The prompt-token count increased from `43` on the seed turn to `1054` on each
generated-context follow-up turn, showing that generated assistant output from a
prior completed streaming response was carried into later requests.

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

## Control-Plane Note

Several long `kubectl exec` polling streams failed with websocket EOF or
connection-reset errors while the gate was running. A few pod reads briefly
returned `ServiceUnavailable`, and `homelab-02` was observed as `NotReady`
during one retry. The proof pod was on `homelab-01`, remained Running with zero
restarts, and the in-pod Ferrite gate continued to exit code `0`. Both staging
nodes were `Ready` again during final evidence collection and after pod
cleanup.

## Cleanup

The server process was stopped after the run. The raw proof log, exit file, and
server log were copied back to local `target/proof/`, then the pod was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-genctx-qwen05-1024 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-genctx-qwen05-1024 --ignore-not-found
```

The final `get pod` command returned no output. Both staging nodes remained
`Ready` after cleanup.

## Interpretation

Ferrite now has real x86_64 AVX2 generated-context long-chat proof for the
OpenAI-compatible server path on Qwen2.5-0.5B Q4_K_M at 256, 512, and 1024
completion-token budgets. The 1024-token run proves generated follow-up
context, token-limit status, request-error recovery, disconnect/reconnect
recovery, per-token latency summaries, RSS sampling, and
`long_chat_summary_run_complete=true` on an amd64 AVX2 pod.

Remaining proof gaps:

- repeat the x86_64 generated-context matrix for Qwen2.5-1.5B Q8_0,
  Qwen2.5-1.5B Q6_K, and SmolLM2-1.7B Q4_K_M;
- broaden EOS-specific long-chat behavior across the required model set;
- run longer steady-state serving and memory-focused samples.
