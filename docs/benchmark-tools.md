# Operational tools

Ferrite includes two focused server clients in addition to `scripts/eval.sh`.
They are built with the `ferrite-server` package and print machine-readable
`key=value` output.

## Throughput client

`ferrite-openai-throughput` drives completion or chat-completion requests and
reports request rate, streaming timing, token rate, finish reason, usage, token
ID coverage, text identity, and optional server RSS.

```sh
target/release/ferrite-openai-throughput \
  --addr 127.0.0.1:8080 \
  --endpoint chat-completions \
  --model qwen2.5-0.5b-q4_k_m \
  --prompt 'Write one sentence about iron.' \
  --requests 4 \
  --concurrency 4 \
  --max-tokens 64 \
  --stream \
  --stream-usage \
  --api-key local-secret
```

Use `--rss-pid <server-pid>` to sample server RSS before, during, and after the
request window. Use `--prompt-cache-key` and `--prompt-cache-trace` only when
evaluating the experimental prefix cache.

## Long-chat gate

`ferrite-openai-long-chat-gate` validates multi-turn continuity, finish
sources, cache accounting, error recovery, client disconnects, queue recovery,
and proof artifact output.

```sh
target/release/ferrite-openai-long-chat-gate \
  --execute \
  --error-probe \
  --disconnect-probe \
  --queue-probe \
  --require-probes error,disconnect,queue \
  --models qwen2.5-0.5b-q4_k_m \
  --token-lengths 256,512,1024 \
  --addr 127.0.0.1:8080 \
  --api-key local-secret
```

The gate can write a proof log and final exit code with `--proof-log` and
`--proof-exit-code`. Generated context caps, state capsule placement, required
finish sources, and required response substrings are available for controlled
long-context experiments.

## Metadata commands

All four executable entry points support `--help`, `-h`, `--version`, and `-V`
without requiring a model or starting a server.

## Complete evaluation

Prefer [`scripts/eval.sh`](evaluation.md#eval-harness) for release comparisons.
It orchestrates the CLI, server, throughput client, resource sampling, token
parity, and JSON plus Markdown output in one reproducible command.
