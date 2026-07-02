# OpenAI Long-Chat Qwen 0.5B Memory Retention 3x128 Probe

## Scope

This run is a small local memory-retention probe for the KV cache memory
pressure theory. It runs three identical generated-context long-chat sessions
against one `Qwen2.5-0.5B-Instruct-Q4_K_M` server process, with RSS sampled
before, after, and after idle for every request.

This is a warm-retention baseline only. It does not prove 6Gi fit, x86_64
behavior, 1.5B/1.7B behavior, 1024-token memory posture, multi-client memory
safety, or steady-state leak freedom.

## Environment

- Date: 2026-07-02
- Commit: `40618a4`
- Host: local macOS development machine
- Server port: `127.0.0.1:18182`
- Server PID for RSS sampling: `69133`
- Model: `Qwen2.5-0.5B-Instruct-Q4_K_M`
- Model path: `target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`
- Model SHA256:
  `6eb923e7d26e9cea28811e1a8e852009b21242fb157b26149d3b188f3a8c8653`
- Server binary SHA256:
  `1c5ccbb758fd03ab2dee640f9346f342c693dff49094f148abef3fea6867b8d3`
- Long-chat gate binary SHA256:
  `6775eafed6979b602bc64646ec09b2e01d59124f3c46f53ad777b2b45a18c9bd`
- API key: `local-secret`
- Raw proof logs:
  - `target/proof/qwen-0-5b-long-chat-memory-retention-session-1.log`
  - `target/proof/qwen-0-5b-long-chat-memory-retention-session-2.log`
  - `target/proof/qwen-0-5b-long-chat-memory-retention-session-3.log`

After the sessions completed and the server was stopped, `lsof -nP
-iTCP:18182 -sTCP:LISTEN` returned no listener.

## Server Command

```sh
target/release/ferrite-server \
  --model target/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf \
  --model-id Qwen2.5-0.5B-Instruct-Q4_K_M \
  --bind 127.0.0.1:18182 \
  --api-key local-secret \
  --default-max-tokens 128 \
  --hard-max-tokens 256 \
  --inference-wait-ms 30000
```

Health check response:

```json
{"status":"ok","ready":true,"model":"Qwen2.5-0.5B-Instruct-Q4_K_M"}
```

## Session Command

Each session used the same gate shape:

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --addr 127.0.0.1:18182 \
  --api-key local-secret \
  --models Qwen2.5-0.5B-Instruct-Q4_K_M \
  --token-lengths 128 \
  --turns 4 \
  --rss-pid 69133 \
  --prompt 'Write a concise operational note about CPU inference stability.' \
  --assistant-context 'CPU inference stability depends on bounded memory use, predictable token latency, and clear server health signals.' \
  --follow-up 'Continue with reconnect and error-handling risks.'
```

The three sessions ran sequentially in the same server process with a short
idle delay between sessions.

## Results

All three sessions completed four streaming chat turns with generated assistant
context on turns 2-4, token-limit status, usage accounting, streaming token IDs,
RSS samples, and `long_chat_summary_run_complete=true`.

| Session | First request RSS before | Final idle RSS | Delta from first before | Max RSS after | Min idle RSS | Generated prompt avg | Generated TTFT avg | Generated stream avg |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 61472768 | 427032576 | +365559808 | 449265664 | 427032576 | 158 | 7543.33 | 14699.00 |
| 2 | 426508288 | 425689088 | -819200 | 435961856 | 424984576 | 158 | 7577.33 | 14820.67 |
| 3 | 425639936 | 429015040 | +3375104 | 433635328 | 428736512 | 158 | 7330.67 | 14207.00 |

The first session includes server/model warmup in the first request RSS
transition. The more useful warm-retention comparison is sessions 2 and 3:

- session 2 first-before to final-idle delta: `-819200` bytes;
- session 3 first-before to final-idle delta: `+3375104` bytes;
- warm final-idle range across sessions 2-3: `425689088` to `429015040`
  bytes.

Each raw log recorded:

```text
long_chat_summary_all_follow_up_turns_use_generated_context=true
long_chat_summary_all_streaming_content_chunks_have_token_ids=true
long_chat_summary_rss_required=true
long_chat_summary_all_rss_present=true
long_chat_summary_run_complete=true
```

## Interpretation

For this small local model and 128-token generated-context gate, RSS stabilized
after the first warm session. The second and third sessions ended within about
3.4 MB of their first pre-request RSS samples, and their final idle RSS values
were within about 3.3 MB of each other.

This falsifies a simple "every repeated 128-token Qwen 0.5B session retains a
large new RSS chunk" hypothesis for the local Mac shape. It does not falsify
the broader KV-cache memory-pressure theory, because the open risk is larger
models, longer 512/1024-token contexts, x86_64 pod limits, multi-client queues,
and explicit cache-retention or eviction policy.
