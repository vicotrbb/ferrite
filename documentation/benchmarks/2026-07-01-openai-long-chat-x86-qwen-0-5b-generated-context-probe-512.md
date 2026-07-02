# OpenAI Long-Chat x86_64 Qwen 0.5B Generated-Context 512-Token Probe

## Scope

This run extends the x86_64 generated-context proof set for the
OpenAI-compatible long-chat gate from 256 to 512 completion tokens for
`Qwen2.5-0.5B-Instruct-Q4_K_M`. It uses a bounded amd64 Kubernetes pod with
`--probe-max-tokens 512`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
512-token budget.

This is one model, one token length, and one bounded x86_64 pod. It proves the
generated assistant-context carry-forward shape for this x86_64 slice, but it
does not close the x86_64 generated-context matrix across the 1024-token length,
larger artifacts, EOS-specific behavior, longer steady-state serving, or
memory-focused reruns.

## Environment

- Date: 2026-07-01 local time, with the run crossing into
  `2026-07-02` UTC
- Commit: `cc8bab2ee1b9ad82eb1c67dbade8d93f79440143`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-genctx-qwen05-512`
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
- Server PID for RSS sampling: `1669`
- Server port inside pod: `127.0.0.1:18149`
- Gate launcher PID: `1698`
- Gate process PID: `1702`
- Pod cgroup memory peak after build and proof: `1481105408` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `539M`
- Raw proof log:
  `target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-512.log`
- Raw proof exit file:
  `target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-512.exit`
- Server log:
  `target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-512-server.log`

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
  name: ferrite-avx2-genctx-qwen05-512
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
kubectl --context staging exec ferrite-avx2-genctx-qwen05-512 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 43.74s
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18149 \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen05-512 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-0.5B-Instruct-Q4_K_M \
    --token-lengths 512 \
    --turns 4 \
    --addr 127.0.0.1:18149 \
    --api-key local-secret \
    --rss-pid 1669 \
    --probe-max-tokens 512 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-512.log 2>&1; \
    echo $? > target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-512.exit'
```

The gate wrote `0` to
`target/proof/x86-qwen-0-5b-q4-long-chat-generated-context-probe-512.exit`.

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
| 1 | seed | 512 | 1 | length | 43 | 512 | 184077 | 513 | 13430 | 182060 | 2.817741 | 300 | 326 | 360 | 13430 | 443621376 | 443752448 | 443752448 |
| 2 | generated | 512 | 1 | length | 542 | 512 | 364410 | 513 | 178208 | 362394 | 1.415584 | 325 | 356 | 393 | 178208 | 443752448 | 454500352 | 454500352 |
| 3 | generated | 512 | 1 | length | 542 | 512 | 365580 | 513 | 178242 | 363562 | 1.411037 | 327 | 356 | 411 | 178242 | 454500352 | 455548928 | 455548928 |
| 4 | generated | 512 | 1 | length | 542 | 512 | 364977 | 513 | 178643 | 362959 | 1.413380 | 327 | 356 | 396 | 178643 | 455548928 | 455548928 | 455548928 |

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

The prompt-token count increased from `43` on the seed turn to `542` on each
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

One `kubectl exec` polling stream failed with a websocket EOF, and one
subsequent API request briefly reported the Kubernetes API as refused on
`192.168.50.132:6443`. A retry showed both staging nodes Ready and the proof
pod still Running with zero restarts. The in-pod Ferrite gate continued and
wrote exit code `0`.

## Cleanup

The server process was stopped after the run. The raw proof log, exit file, and
server log were copied back to local `target/proof/`, then the pod was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-genctx-qwen05-512 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-genctx-qwen05-512 --ignore-not-found
```

The final `get pod` command returned no output. Both staging nodes remained
`Ready` after cleanup.

## Interpretation

Ferrite now has real x86_64 AVX2 generated-context long-chat proof for the
OpenAI-compatible server path on Qwen2.5-0.5B Q4_K_M at both 256 and 512
completion-token budgets. The 512-token run proves generated follow-up context,
token-limit status, request-error recovery, disconnect/reconnect recovery,
per-token latency summaries, RSS sampling, and
`long_chat_summary_run_complete=true` on an amd64 AVX2 pod.

Remaining proof gaps:

- repeat this x86_64 generated-context shape for the 1024 completion-token
  budget;
- repeat the x86_64 generated-context matrix for Qwen2.5-1.5B Q8_0,
  Qwen2.5-1.5B Q6_K, and SmolLM2-1.7B Q4_K_M;
- broaden EOS-specific long-chat behavior across the required model set;
- run longer steady-state serving and memory-focused samples.
