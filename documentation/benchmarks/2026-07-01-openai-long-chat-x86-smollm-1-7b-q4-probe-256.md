# OpenAI Long-Chat x86_64 SmolLM2 1.7B Q4 256-Token Probe Gate

## Scope

This run starts the x86_64 combined reconnect/error long-chat proof set for the
`SmolLM2-1.7B-Instruct-Q4_K_M` Tier 1 artifact. It exercises the
OpenAI-compatible HTTP server in a bounded amd64 Kubernetes pod with
`--probe-max-tokens 256`, so the request-error reconnect path, disconnect
reconnect path, and all repeated streaming chat scenarios use the same
256-token budget.

This is one model, one token length, and one bounded x86_64 pod. It proves the
256-token x86_64 long-chat budget for SmolLM2-1.7B Q4_K_M, but it does not
prove the 512-token or 1024-token x86_64 SmolLM2 budgets and does not close the
full x86_64 Tier 1 HTTP long-chat gate.

## Environment

- Date: 2026-07-01
- Commit: `1a79cd9`
- Kubernetes context: `staging`
- Pod: `ferrite-avx2-long-chat-smollm17-q4-256`
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
- Rust toolchain: `rustc 1.96.0`, host `x86_64-unknown-linux-gnu`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path in pod: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `77665ea4815999596525c636fbeb56ba8b080b46ae85efef4f0d986a139834d7`
- Server PID for RSS sampling: `1670`
- Server port inside pod: `127.0.0.1:18127`
- Gate launcher PID in pod: `1697`
- Gate process PID in pod: `1702`
- Gate exit-code file:
  `target/proof/x86-smollm-1-7b-q4-long-chat-probe-256.exit`
- Gate exit code: `0`
- Server initial RSS after model load: `1042252` KiB
- Pod cgroup memory peak after model load: `2806734848` bytes
- Pod cgroup memory peak after build and proof: `2806734848` bytes
- Workspace size after source copy, model copy, release build, and proof:
  `1.2G`
- Raw log: `target/proof/x86-smollm-1-7b-q4-long-chat-probe-256.log`
- Server log: `target/proof/x86-smollm-1-7b-q4-server-256.log`

The pod-side release binaries were built inside the amd64 pod. `file` reported
both `target/release/ferrite-server` and
`target/release/ferrite-openai-long-chat-gate` as `ELF 64-bit LSB pie
executable, x86-64`.

Pod-side release binary hashes:

```text
e5f148ec2c6686d532ac1ea37c48abf6f9ba7c3a4ba46c1ffae28a98ebed261a  target/release/ferrite-server
f613e12832ee0d9ccad126a8ab900e2fe0ee6e8612c181b3d7de137264a8ff24  target/release/ferrite-openai-long-chat-gate
```

## Pod Manifest

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ferrite-avx2-long-chat-smollm17-q4-256
spec:
  restartPolicy: Never
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
kubectl --context staging exec ferrite-avx2-long-chat-smollm17-q4-256 -- sh -lc \
  'export PATH=/usr/local/cargo/bin:$PATH; cd /work/ferrite && cargo build -p ferrite-server --release --bins'
```

Result:

```text
Finished `release` profile [optimized] target(s) in 38.74s
```

The cgroup memory peak immediately after the build and before server startup
was `1721065472` bytes.

## Server Command

The first server-start observation was interrupted by a staging API reset and
connection refusal before launch. A follow-up inspection showed no
`ferrite-server` process and an empty `target/proof` directory. The server was
then started cleanly with:

```sh
cd /work/ferrite
./target/release/ferrite-server \
  --bind 127.0.0.1:18127 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 256 \
  --hard-max-tokens 512
```

Health check response:

```json
{"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

Initial `ps` RSS for the server process was `1042252` KiB. The pod cgroup
memory peak after model load was `2806734848` bytes, below the `6Gi` memory
limit.

## Gate Command

The gate used the known full-length SmolLM2 operational prompt shape from the
earlier local SmolLM2 full matrix. The default short prompt can terminate early
for this model, so this run used explicit prompt, assistant context, and
follow-up text while still expecting `finish_reason=length`.

```sh
kubectl --context staging exec ferrite-avx2-long-chat-smollm17-q4-256 -- sh -lc \
  'cd /work/ferrite && nohup sh -lc '"'"'./target/release/ferrite-openai-long-chat-gate \
    --execute \
    --error-probe \
    --disconnect-probe \
    --models SmolLM2-1.7B-Instruct-Q4_K_M \
    --token-lengths 256 \
    --turns 4 \
    --addr 127.0.0.1:18127 \
    --api-key local-secret \
    --rss-pid 1670 \
    --probe-max-tokens 256 \
    --expect-finish-reason length \
    --prompt "Write a concise operational note about CPU inference stability." \
    --assistant-context "CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals." \
    --follow-up "Continue with reconnect and error-handling risks." \
    > target/proof/x86-smollm-1-7b-q4-long-chat-probe-256.log 2>&1; \
    echo $? > target/proof/x86-smollm-1-7b-q4-long-chat-probe-256.exit'"'"' \
    >/dev/null 2>&1 & echo $!'
```

The exit-code file contained:

```text
0
```

## Probe Results

Both probes completed and recorded the configured 256-token budget:

```text
long_chat_error_probe_unauthorized_status=401
long_chat_error_probe_reconnect_completed=true
long_chat_error_probe_max_tokens=256
long_chat_disconnect_probe_aborted_after_generated_event=true
long_chat_disconnect_probe_reconnect_completed=true
long_chat_disconnect_probe_max_tokens=256
```

## Scenario Results

All four 256-token streaming chat scenarios completed with
`finish_reason=length`, usage accounting for 256 completion tokens, streaming
timing, per-token latency summaries, and RSS samples.

| Turn | Max tokens | Completed | Finish | Total ms | Events | TTFT ms | Stream ms | Tok/s | Lat min ms | Lat p50 ms | Lat p95 ms | Lat max ms | RSS before | RSS after | RSS idle |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 256 | 1 | length | 264767 | 257 | 44125 | 262754 | 0.978100 | 797 | 851 | 906 | 44125 | 1135362048 | 1135087616 | 1135087616 |
| 2 | 256 | 1 | length | 264841 | 257 | 43971 | 262828 | 0.977825 | 792 | 852 | 906 | 43971 | 1135087616 | 1135259648 | 1135259648 |
| 3 | 256 | 1 | length | 268601 | 257 | 44218 | 266588 | 0.964034 | 789 | 847 | 1016 | 44218 | 1135259648 | 1134952448 | 1134952448 |
| 4 | 256 | 1 | length | 269881 | 257 | 44727 | 267869 | 0.959424 | 798 | 858 | 973 | 44727 | 1134952448 | 1135206400 | 1135206400 |

Usage was stable for every turn:

- prompt tokens: `53`;
- completion tokens: `256`;
- total tokens: `309`.

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

## Staging Control-Plane Notes

During setup, the initial server-start observation was interrupted by a
staging API reset and connection refusal. Inspection after API recovery showed
no server process and an empty proof directory, so the server was started again
cleanly.

During the long proof, `homelab-01` temporarily reported `NotReady`, and a
`kubectl exec` poll failed with a `502 Bad Gateway` proxy error while the API
server tried to dial the node kubelet. The pod later remained `Running` with
zero restarts, the node returned to `Ready`, and a recovered exec showed the
detached in-pod gate had reached `exit=0` and
`long_chat_summary_run_complete=true`.

This evidence is valid for the Ferrite server/gate behavior recorded in the
in-pod log, but it should not be used as a clean staging
control-plane-stability sample.

## Cleanup

The server process was stopped after the run. The raw proof log, server log,
and gate exit-code file were copied back to local `target/proof/`, then the pod
was deleted:

```sh
kubectl --context staging delete pod ferrite-avx2-long-chat-smollm17-q4-256 --wait=true --timeout=120s
kubectl --context staging get pod ferrite-avx2-long-chat-smollm17-q4-256 --ignore-not-found
kubectl --context staging get nodes -o wide
```

Final node check showed both `homelab-01` and `homelab-02` `Ready`.

## Conclusion

Ferrite now has one real x86_64 AVX2 combined long-chat reconnect/error proof
for the OpenAI-compatible server path on `SmolLM2-1.7B-Instruct-Q4_K_M` at the
256-token budget. The 512-token and 1024-token x86_64 SmolLM2 budgets remain
unproven.
