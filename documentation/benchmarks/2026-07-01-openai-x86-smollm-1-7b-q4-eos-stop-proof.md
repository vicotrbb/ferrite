# OpenAI x86_64 SmolLM 1.7B Q4 EOS Stop Proof

## Scope

This run adds bounded x86_64 staging evidence for tokenizer EOS termination
through Ferrite's OpenAI-compatible HTTP surfaces for
`SmolLM2-1.7B-Instruct-Q4_K_M`. It repeats the known EOS-sensitive prompt,
`The capital of France is`, and verifies that natural tokenizer EOS maps to
OpenAI `finish_reason: "stop"` without exposing the EOS control text as
assistant-visible output.

This is one model, one bounded amd64 pod, and one EOS-sensitive prompt shape. It
extends the local SmolLM2 EOS proof to x86_64, but it does not prove Qwen EOS
behavior, full long-chat EOS behavior, steady-state serving, or memory leak
freedom.

## Environment

- Date: 2026-07-01
- Commit: `af0c00da8881136986984eeacf542a5d7e8fe8cb`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-eos-smollm17-q4`
- Node: `homelab-01`
- Pod image: `rust:1.96-bookworm`
- Host architecture: `x86_64`
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
- Server PID for RSS sampling: `1688`
- Server port inside pod: `127.0.0.1:18134`
- Release build cgroup memory peak: `2725265408` bytes
- Post-health cgroup memory peak: `4190863360` bytes
- Pod cgroup memory peak after proof: `4190863360` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `1.2G`
- Raw probe log: `target/proof/x86-smollm-1-7b-q4-eos-probe.log`
- Raw probe exit file: `target/proof/x86-smollm-1-7b-q4-eos-probe.exit`
- Raw server log: `target/proof/x86-smollm-1-7b-q4-eos-server.log`

The first model copy was interrupted during a transient Kubernetes
websocket/control-plane reset and produced a truncated file. The bad copy was
detected by SHA256 mismatch, removed, and replaced. Only the correct-hash model
listed above was used for the build and proof run.

The first probe attempt had a Python quoting bug after the first valid
non-streaming response. The corrected probe was rerun successfully and wrote
`0` to `target/proof/x86-smollm-1-7b-q4-eos-probe.exit`.

## Pod Manifest

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ferrite-avx2-eos-smollm17-q4
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
kubectl --context staging exec ferrite-avx2-eos-smollm17-q4 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite; cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 40.38s
```

`file target/release/ferrite-server` reported an x86-64 Linux ELF binary, and
the release server SHA256 was:

```text
e5f148ec2c6686d532ac1ea37c48abf6f9ba7c3a4ba46c1ffae28a98ebed261a  target/release/ferrite-server
```

## Server Command

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18134 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 64
```

Health check response:

```json
{"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

Post-health process and cgroup samples:

```text
PID 1688, RSS 1042264 KiB
memory_current=3123949568
memory_peak=4190863360
```

After the proof run:

```text
PID 1688, RSS 1045056 KiB
memory_current=3126702080
memory_peak=4190863360
```

## Probe Results

All endpoint probes returned HTTP `200`, natural EOS mapped to
`finish_reason="stop"`, streaming responses emitted exactly one `[DONE]`, and
no visible `<|im_end|>` marker appeared in the output.

| Endpoint | Mode | Content type | Visible output | Finish | Usage | `[DONE]` |
| --- | --- | --- | --- | --- | --- | --- |
| `/v1/completions` | non-streaming | `application/json` | ` Paris.` | `stop` | prompt `5`, completion `3`, total `8` | n/a |
| `/v1/completions` | streaming | `text/event-stream` | ` Paris` then `.` | `stop` | prompt `5`, completion `3`, total `8` | exactly one |
| `/v1/chat/completions` | streaming | `text/event-stream` | content through ` Paris.` | `stop` | prompt `12`, completion `9`, total `21` | exactly one |

The corrected probe printed:

```text
completion_eos_finish_reason=stop
completion_eos_visible_text=Paris_period
completion_eos_usage_prompt_tokens=5
completion_eos_usage_completion_tokens=3
completion_eos_usage_total_tokens=8
completion_stream_eos_finish_reason=stop
completion_stream_done_count=1
completion_stream_usage_completion_tokens=3
chat_stream_eos_finish_reason=stop
chat_stream_done_count=1
chat_stream_usage_prompt_tokens=12
chat_stream_usage_completion_tokens=9
chat_stream_usage_total_tokens=21
visible_eos_marker_present=false
```

## Interpretation

Ferrite's OpenAI-compatible completion and streaming chat surfaces now have
bounded x86_64 SmolLM2 evidence that tokenizer EOS:

- terminates generation before the requested token budget;
- maps to OpenAI `finish_reason: "stop"`;
- preserves generated-token usage accounting;
- emits the streaming `[DONE]` marker exactly once;
- suppresses EOS control text from assistant-visible output.

The remaining EOS gaps are Qwen-specific EOS behavior, full long-chat EOS
matrix behavior across the required Tier 1 HTTP models, longer steady-state
serving, and memory-focused reruns.
