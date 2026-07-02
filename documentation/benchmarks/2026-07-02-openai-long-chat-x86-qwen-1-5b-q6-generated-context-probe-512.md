# OpenAI Long-Chat x86_64 Qwen 1.5B Q6 Generated-Context 512-Token Probe

## Scope

This run extends the x86_64 generated-context proof set for the larger
`Qwen2.5-1.5B-Instruct-Q6_K` OpenAI-compatible HTTP model artifact. It uses a
bounded amd64 Kubernetes pod with `--probe-max-tokens 512`, so the
request-error reconnect path, disconnect reconnect path, and all repeated
streaming chat scenarios use the same 512-token budget.

This is one model, one token length, and one bounded x86_64 pod. Together with
the earlier 256-token run, it proves the generated assistant-context
carry-forward shape for this Q6_K x86_64 slice at 256 and 512 tokens, but it
does not close the x86_64 generated-context matrix across the remaining
1024-token length, Qwen2.5-1.5B Q8_0, SmolLM2-1.7B, EOS-specific behavior,
longer steady-state serving, or memory-focused reruns.

## Environment

- Date: 2026-07-02 local time
- Commit: `61819b929882e3e9e0ccd37b48e56b6e32dc6323`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-genctx-qwen15-q6-512`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.228`
- Host architecture: `x86_64`
- CPU features: `/proc/cpuinfo` included `avx` and `avx2`
- CPU request: `500m`
- CPU limit: `2`
- Memory request: `1Gi`
- Memory limit: `6Gi` (`memory.max=6442450944`)
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
- Server PID for RSS sampling: `1651`
- Server port inside pod: `127.0.0.1:18152`
- Gate process PID: `1703`
- Server RSS after model load: `1455160` KiB
- Pod cgroup memory current after health check: `2795798528` bytes
- Pod cgroup memory peak after build and proof: `4333760512` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `1.6G`
- Raw proof log:
  `target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-512.log`
- Raw proof exit file:
  `target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-512.exit`
- Server log:
  `target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-512-server.log`

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
  name: ferrite-avx2-genctx-qwen15-q6-512
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
          memory: "6Gi"
          ephemeral-storage: "12Gi"
      volumeMounts:
        - name: work
          mountPath: /work
  volumes:
    - name: work
      emptyDir:
        sizeLimit: 12Gi
```

## Build Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen15-q6-512 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

The release build completed successfully:

```text
Finished `release` profile [optimized] target(s) in 38.06s
```

A binary identity command initially tried to run `rustc -Vv` without the Cargo
path exported and printed `sh: 1: rustc: not found`. Rerunning the same identity
check with `PATH=/usr/local/cargo/bin:$PATH` produced the Rust toolchain details
listed above. This was a command setup issue, not a build or proof failure.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18152 \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q6_K \
  --api-key local-secret \
  --default-max-tokens 512 \
  --hard-max-tokens 1024
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q6_K"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen15-q6-512 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-1.5B-Instruct-Q6_K \
    --token-lengths 512 \
    --turns 4 \
    --addr 127.0.0.1:18152 \
    --api-key local-secret \
    --rss-pid 1651 \
    --probe-max-tokens 512 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-512.log 2>&1; \
    echo $? > target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-512.exit'
```

The launch wrapper printed
`cannot create target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-512.pid:
Directory nonexistent` while the gate process was already running. The proof
directory, log, and exit file were present afterward, and the gate wrote `0` to
`target/proof/x86-qwen-1-5b-q6-long-chat-generated-context-probe-512.exit`.

During the long run, Kubernetes API and exec checks intermittently returned
connection resets, connection refusals, or API `ServiceUnavailable` responses
from the staging control plane. The pod remained `Running` with zero restarts,
the server continued consuming CPU during generation, and the in-pod gate
completed. This is recorded as staging control-plane instability, not as an
in-pod Ferrite proof failure.

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
| 1 | seed | 512 | 1 | length | 43 | 512 | 645061 | 513 | 47964 | 643044 | 0.797768 | 1054 | 1152 | 1239 | 47964 | 1537286144 | 1536749568 | 1536749568 |
| 2 | generated | 512 | 1 | length | 548 | 512 | 1266880 | 513 | 640079 | 1264866 | 0.405577 | 1136 | 1205 | 1302 | 640079 | 1536749568 | 1575460864 | 1575460864 |
| 3 | generated | 512 | 1 | length | 543 | 512 | 1252492 | 513 | 623678 | 1250476 | 0.410244 | 1133 | 1213 | 1302 | 623678 | 1575460864 | 1574916096 | 1574916096 |
| 4 | generated | 512 | 1 | length | 538 | 512 | 1236790 | 513 | 620604 | 1234775 | 0.415460 | 1126 | 1194 | 1260 | 620604 | 1574916096 | 1574428672 | 1574428672 |

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

The prompt-token count increased from `43` on the seed turn to `548` on the
first generated-context follow-up turn. Turns 3 and 4 reported `543` and `538`
prompt tokens respectively while still using generated assistant context.

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
kubectl --context staging delete pod ferrite-avx2-genctx-qwen15-q6-512 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-genctx-qwen15-q6-512 --ignore-not-found
```

The final `get pod` command returned no output. Both staging nodes remained
`Ready` after cleanup.

## Interpretation

Ferrite now has real x86_64 AVX2 generated-context long-chat proof for the
larger Qwen2.5-1.5B Q6_K OpenAI-compatible server path at the 256 and 512
completion-token budgets. This run proves generated follow-up context,
token-limit status, request-error recovery, disconnect/reconnect recovery,
per-token latency summaries, RSS sampling, and
`long_chat_summary_run_complete=true` on an amd64 AVX2 pod at 512 tokens.

Remaining proof gaps:

- repeat this x86_64 generated-context shape for Qwen2.5-1.5B Q6_K at the
  1024 completion-token budget;
- repeat the x86_64 generated-context matrix for Qwen2.5-1.5B Q8_0 and
  SmolLM2-1.7B Q4_K_M;
- broaden EOS-specific long-chat behavior across the required model set;
- run longer steady-state serving and memory-focused samples.
