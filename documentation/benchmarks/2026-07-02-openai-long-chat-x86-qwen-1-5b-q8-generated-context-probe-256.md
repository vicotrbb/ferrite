# OpenAI Long-Chat x86_64 Qwen 1.5B Q8 Generated-Context 256-Token Probe

## Scope

This run starts the x86_64 generated-context proof set for the larger
`Qwen2.5-1.5B-Instruct-Q8_0` OpenAI-compatible HTTP model artifact. It uses a
bounded amd64 Kubernetes pod with `--probe-max-tokens 256`, so the
request-error reconnect path, disconnect reconnect path, and all repeated
streaming chat scenarios use the same 256-token budget.

This is one model, one token length, and one bounded x86_64 pod. It proves the
generated assistant-context carry-forward shape for this Q8_0 x86_64 slice, but
it does not close the x86_64 generated-context matrix across the 512/1024-token
Q8_0 lengths, SmolLM2-1.7B, EOS-specific behavior, longer steady-state serving,
memory-focused reruns, or 6Gi fit for this Q8_0 path.

## Environment

- Date: 2026-07-02 local time
- Commit: `0a73876be6c6a258e001f063cd60742e9523e7aa`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-genctx-qwen15-q8-256`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Pod IP: `10.42.248.201`
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
- Model SHA256:
  `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8`
- Server PID for RSS sampling: `1733`
- Server port inside pod: `127.0.0.1:18154`
- Gate process PID: `1773`
- Server RSS after model load: `1875488` KiB
- Pod cgroup memory current after health check: `2252763136` bytes
- Pod cgroup memory peak after build and proof: `4146102272` bytes
- Pod cgroup memory current after server stop: `332021760` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `2.0G`
- Raw proof log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-256.log`
- Raw proof exit file:
  `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-256.exit`
- Server log:
  `target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-256-server.log`

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
  name: ferrite-avx2-genctx-qwen15-q8-256
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
close to the 6Gi pod memory limit. This run is therefore not evidence that this
Q8_0 generated-context path fits a 6Gi limit.

## Build Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen15-q8-256 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

The release build completed successfully:

```text
Finished `release` profile [optimized] target(s) in 39.04s
```

The first `kubectl cp` model transfer failed with a websocket broken-pipe and
left a partial 1.1G model file with SHA256
`82f5476a9ab8bf45781a08e84a6d73c31653a447b3dd0ccd662c51dea56c0bb4`. The
partial file was deleted, the transfer was retried after the staging API
recovered, and the final in-pod SHA256 matched the expected Q8_0 hash above.

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18154 \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --model-id Qwen2.5-1.5B-Instruct-Q8_0 \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-1.5B-Instruct-Q8_0"}
```

## Gate Command

```sh
kubectl --context staging exec ferrite-avx2-genctx-qwen15-q8-256 -- sh -lc \
  'cd /work/ferrite && ./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models Qwen2.5-1.5B-Instruct-Q8_0 \
    --token-lengths 256 \
    --turns 4 \
    --addr 127.0.0.1:18154 \
    --api-key local-secret \
    --rss-pid 1733 \
    --probe-max-tokens 256 \
    --expect-finish-reason length \
    > target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-256.log 2>&1; \
    echo $? > target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-256.exit'
```

The first background launch wrapper wrote its PID file from the wrong shell
working directory and printed `cannot create ... .pid: Directory nonexistent`.
The gate itself had already started inside `/work/ferrite`; PID `1773` was
recorded afterward from inside the pod. The gate wrote `0` to
`target/proof/x86-qwen-1-5b-q8-long-chat-generated-context-probe-256.exit`.

## Probe Results

Both reconnect/error probes completed with the configured 256-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_reconnect_generated_event=true
long_chat_disconnect_probe_reconnect_started_new_generation=true
long_chat_disconnect_probe_max_tokens=256
```

The disconnect reconnect response included generated stream content and started
a fresh generation after reconnect.

## Scenario Results

All four 256-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 256 completion tokens, token-limit
status, generated-context status, streaming timing, per-token latency summaries,
and RSS samples.

| Turn | Context | Max tokens | Completed | Finish | Prompt tokens | Completion tokens | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | seed | 256 | 1 | length | 43 | 256 | 77783 | 257 | 9930 | 75767 | 3.391947 | 221 | 240 | 355 | 9930 | 1940291584 | 1940291584 | 1940291584 |
| 2 | generated | 256 | 1 | length | 287 | 256 | 144228 | 257 | 72951 | 142214 | 1.807133 | 244 | 267 | 308 | 72951 | 1940291584 | 1954840576 | 1954840576 |
| 3 | generated | 256 | 1 | length | 287 | 256 | 141397 | 257 | 70276 | 139382 | 1.843847 | 241 | 268 | 304 | 70276 | 1954840576 | 1954840576 | 1954840576 |
| 4 | generated | 256 | 1 | length | 282 | 256 | 140057 | 257 | 68826 | 138043 | 1.861732 | 243 | 266 | 311 | 68826 | 1954840576 | 1955495936 | 1955495936 |

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

The prompt-token count increased from `43` on the seed turn to `287` on the
first two generated-context follow-up turns. Turn 4 reported `282` prompt
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
kubectl --context staging delete pod ferrite-avx2-genctx-qwen15-q8-256 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-genctx-qwen15-q8-256 --ignore-not-found
```

The first delete attempt failed with `Unable to connect to the server:
unexpected EOF`. A later retry deleted the pod, the final `get pod` command
returned no output, and both staging nodes were `Ready` after cleanup.

## Interpretation

Ferrite now has the first real x86_64 AVX2 generated-context long-chat proof
for the larger Qwen2.5-1.5B Q8_0 OpenAI-compatible server path at the 256
completion-token budget. This run proves generated follow-up context,
token-limit status, request-error recovery, disconnect/reconnect recovery,
per-token latency summaries, RSS sampling, and
`long_chat_summary_run_complete=true` on an amd64 AVX2 pod at 256 tokens.

Remaining proof gaps:

- repeat this x86_64 generated-context shape for Qwen2.5-1.5B Q8_0 at 512 and
  1024 completion-token budgets;
- repeat the x86_64 generated-context matrix for SmolLM2-1.7B Q4_K_M;
- broaden EOS-specific long-chat behavior across the required model set;
- run longer steady-state serving and memory-focused samples;
- run memory-focused Q8_0 samples if 6Gi fit is required.
