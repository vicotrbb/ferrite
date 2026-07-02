# OpenAI Long-Chat x86_64 Qwen 1.5B Q6 Generated-Context 1024-Token Probe

## Scope

This run closes the x86_64 generated-context proof set for the larger
`Qwen2.5-1.5B-Instruct-Q6_K` OpenAI-compatible HTTP model artifact at the
1024-token length. It uses a bounded amd64 Kubernetes pod with
`--probe-max-tokens 1024`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
1024-token budget.

Together with the earlier 256-token and 512-token runs, this proves the
generated assistant-context carry-forward shape for this Q6_K x86_64 slice at
256, 512, and 1024 completion tokens. It does not close x86_64
generated-context coverage for Qwen2.5-1.5B Q8_0 or SmolLM2-1.7B, broader
EOS-specific behavior, longer steady-state serving, memory-focused reruns, or
6Gi fit at this 1024-token length.

## Environment

- Date: 2026-07-02 local time
- Commit: `930c108f8bd13655e87d868b99f549ce9dc1b3be`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-genctx-qwen15-q6-1024`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.251`
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
- Model: `Qwen2.5-1.5B-Instruct-Q6_K`
- Model path in pod: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Model SHA256:
  `e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7`
- Server PID for RSS sampling: `1709`
- Server port inside pod: `127.0.0.1:18153`
- Gate process PID: `1749`
- Server RSS after model load: `1455508` KiB
- Pod cgroup memory current after health check: `2527948800` bytes
- Pod cgroup memory peak after build and proof: `4026712064` bytes
- Pod cgroup memory current after server stop: `92680192` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `1.6G`
- Raw proof log:
  `target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-1024.log`
- Raw proof exit file:
  `target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-1024.exit`
- Server log:
  `target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-1024-server.log`

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
  name: ferrite-avx2-genctx-qwen15-q6-1024
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

The 1024-token pod used an 8Gi memory limit to avoid turning this long proof
into an avoidable OOM test. This run is therefore not evidence that the same
1024-token path fits the 6Gi limit used by the 256-token and 512-token Q6_K
runs.

## Build Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen15-q6-1024 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

The release build completed successfully:

```text
Finished `release` profile [optimized] target(s) in 43.26s
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18153 \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q6_K \
  --api-key local-secret \
  --default-max-tokens 1024 \
  --hard-max-tokens 2048
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q6_K"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen15-q6-1024 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-1.5B-Instruct-Q6_K \
    --token-lengths 1024 \
    --turns 4 \
    --addr 127.0.0.1:18153 \
    --api-key local-secret \
    --rss-pid 1709 \
    --probe-max-tokens 1024 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-1024.log 2>&1; \
    echo $? > target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-1024.exit'
```

The first background launch wrapper wrote its PID file from the wrong shell
working directory and printed `cannot create ... .pid: Directory nonexistent`.
The gate itself had already started inside `/work/ferrite`; PID `1749` was
recorded afterward from inside the pod. The gate wrote `0` to
`target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-1024.exit`.

During the long run, the staging control plane intermittently returned
connection refusals, node `NotReady` states, and a kubelet proxy `502 Bad
Gateway` for exec checks. The pod remained `Running` with zero restarts, the
server continued consuming CPU during generation, and the in-pod gate
completed. This is recorded as staging control-plane instability, not as an
in-pod Ferrite proof failure.

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
| 1 | seed | 1024 | 1 | length | 43 | 1024 | 1285721 | 1025 | 48672 | 1283702 | 0.798472 | 1077 | 1191 | 1320 | 48672 | 1575395328 | 1575436288 | 1575436288 |
| 2 | generated | 1024 | 1 | length | 1075 | 1024 | 2651386 | 1025 | 1293975 | 2649367 | 0.386885 | 1189 | 1310 | 1459 | 1293975 | 1575436288 | 1648328704 | 1648328704 |
| 3 | generated | 1024 | 1 | length | 1060 | 1024 | 2613467 | 1025 | 1265139 | 2611448 | 0.392502 | 1196 | 1307 | 1401 | 1265139 | 1648328704 | 1647730688 | 1647730688 |
| 4 | generated | 1024 | 1 | length | 1055 | 1024 | 2618452 | 1025 | 1266045 | 2616429 | 0.391755 | 1188 | 1302 | 1433 | 1266045 | 1647730688 | 1647185920 | 1647185920 |

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

The prompt-token count increased from `43` on the seed turn to `1075` on the
first generated-context follow-up turn. Turns 3 and 4 reported `1060` and
`1055` prompt tokens respectively while still using generated assistant
context.

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
kubectl --context staging delete pod ferrite-avx2-genctx-qwen15-q6-1024 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-genctx-qwen15-q6-1024 --ignore-not-found
```

The final `get pod` command returned no output. Both staging nodes were `Ready`
after cleanup.

## Interpretation

Ferrite now has real x86_64 AVX2 generated-context long-chat proof for the
larger Qwen2.5-1.5B Q6_K OpenAI-compatible server path at 256, 512, and 1024
completion-token budgets. This run proves generated follow-up context,
token-limit status, request-error recovery, disconnect/reconnect recovery,
per-token latency summaries, RSS sampling, and
`long_chat_summary_run_complete=true` on an amd64 AVX2 pod at 1024 tokens.

Remaining proof gaps:

- repeat the x86_64 generated-context matrix for Qwen2.5-1.5B Q8_0 and
  SmolLM2-1.7B Q4_K_M;
- broaden EOS-specific long-chat behavior across the required model set;
- run longer steady-state serving and memory-focused samples;
- run a memory-focused 1024-token Q6_K sample if 6Gi fit is required.
