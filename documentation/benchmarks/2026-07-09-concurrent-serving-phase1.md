# Benchmark: concurrent serving phase 1 (configurable inference permits)

- Date: 2026-07-09 (commit: post slice-C head, dirty tree during run)
- Host: Apple M5 Pro (5+10 perflevels), 24 GiB, macOS 26.5.2, 10-thread
  inference pool (`inference_threads=10`)
- Model: target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf (Q5_0/Q8_0/
  Q6_K/Q4_K mixed; 468.6 MiB file)
- Server: `ferrite-server --default-max-tokens 48 --hard-max-tokens 64
  --inference-wait-ms 120000 --max-concurrent-inferences {1,4}`
- Client: `ferrite-openai-throughput --endpoint chat-completions
  --requests 8 --concurrency {1,2,4} --max-tokens 48 --stream`
- Prompt: "Write a short story about a rusty robot who learns to sail."
  (18 prompt tokens; every request completed 48 tokens)

## Results

| permits | client concurrency | requests/s | aggregate tok/s (req/s × 48) | first-request stream tok/s | TTFT ms |
| --- | --- | --- | --- | --- | --- |
| 4 | 1 | 1.006 | 48.3 | 49.5 | 267 |
| 4 | 2 | 1.492 | 71.6 | 36.9 | 355 |
| 4 | 4 | 1.834 | 88.0 | 22.8 | 563 |
| 1 (default) | 4 | 0.968 | 46.5 | 45.5 | — |

Aggregate throughput at 4 concurrent streams is +89% over the
serialized default (1.834 vs 0.968 req/s). Scaling is sub-linear because
concurrent sessions contend for the shared 10-thread rayon pool; the
per-stream rate drops as streams share the memory system. Batched
decode (ADR 0011 phase 2) is the designed answer: one weight stream
serving all sessions per token step.

## Correctness

Three distinct prompts, 32 tokens each, run sequentially and then
concurrently against `--max-concurrent-inferences 4`: outputs
byte-identical (greedy decoding, independent sessions). All 8 streaming
requests in every run finished with `finish_reason=completed` and 48/48
generated token ids.

## Notes

- Default remains `--max-concurrent-inferences 1`; behavior, queueing
  and 429/wait semantics are unchanged unless the operator raises it.
- `streaming_tokens_per_second` is computed from the first completed
  request and divides by elapsed-including-TTFT; requests/s is the
  aggregate-truth metric for multi-stream runs.
