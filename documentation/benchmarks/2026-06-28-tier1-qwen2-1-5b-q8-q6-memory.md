# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 and Q6_K Memory

## Scope

This benchmark records bounded local memory posture for Ferrite's CLI running
Qwen2.5-1.5B-Instruct Q8_0 and Q6_K.

This is a memory accounting, process peak RSS, and post-load current RSS note.
It does not prove full Tier 1 memory posture, server concurrency memory
behavior, long-context memory growth, or larger-model readiness.

## Tree State

- Branch: `main`
- Commit before run: `7f3b223`
- Working tree before run: clean

## Hardware and OS

- Machine: Apple M1 Pro
- Logical CPUs: 8
- Physical CPUs: 8
- RAM: 17,179,869,184 bytes
- OS: macOS Darwin 23.5.0 arm64

Commands:

```sh
sysctl -n machdep.cpu.brand_string hw.ncpu hw.physicalcpu hw.logicalcpu hw.memsize
uname -a
```

## Models

| Model | Local path | File size | SHA-256 |
| --- | --- | ---: | --- |
| Qwen2.5-1.5B-Instruct Q8_0 | `target/models/qwen2.5-1.5b-instruct-q8_0.gguf` | 1.8 GB | `d7efb072e7724d25048a4fda0a3e10b04bdef5d06b1403a1c93bd9f1240a63c8` |
| Qwen2.5-1.5B-Instruct Q6_K | `target/models/qwen2.5-1.5b-instruct-q6_k.gguf` | 1.4 GB | `e16d94f3b1eb243f6f6be9eee51090ef5dfd741324394fd5b6e0e425c33df5c7` |

## Protocol

- Host: local macOS aarch64
- Binary: `target/release/ferrite`
- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Generation limit: one token
- Expected generated token IDs: `198`
- Peak measurement: `/usr/bin/time -l` process summary plus Ferrite's component
  memory accounting
- Current RSS measurement: direct `target/release/ferrite` run with
  `--sleep-after-load-ms 5000`, then `ps -o rss= -p "$pid"` during the
  post-load sleep

## Commands

```sh
/usr/bin/time -l target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --generate-tokens 1 \
  --expect-generated-token-ids 198

/usr/bin/time -l target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --prompt 'hello world' \
  --generate-tokens 1 \
  --expect-generated-token-ids 198
```

Current RSS sample command shape:

```sh
target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --sleep-after-load-ms 5000 \
  --generate-tokens 1 \
  --expect-generated-token-ids 198 \
  > /tmp/ferrite-q8-load-sleep-direct.out &

pid=$!
# Wait until stdout contains: sleep_after_load_ms=5000
ps -o rss= -p "$pid"
```

The Q6_K run used the same shape with
`target/models/qwen2.5-1.5b-instruct-q6_k.gguf`.

The first attempted sampler wrapped the command in `/usr/bin/time -l`, which
samples the wrapper process when using `$!`. The retained current-RSS samples
therefore run `target/release/ferrite` directly and keep `/usr/bin/time -l`
only for peak process measurements.

## Peak RSS Results

| Model | model_file_bytes | model_file_retained_bytes | scalar_weight_bytes | kv_cache_bytes | Max RSS | Peak footprint | Real time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Qwen2.5-1.5B Q8_0 | 1,894,532,128 | 0 | 1,888,581,632 | 172,032 | 3,821,076,480 | 3,822,526,784 | 0.83 s |
| Qwen2.5-1.5B Q6_K | 1,464,178,720 | 0 | 1,458,228,224 | 172,032 | 2,961,850,368 | 2,962,856,576 | 1.23 s |

## Post-Load Current RSS Results

| Model | Current RSS sample 1 | Current RSS sample 2 | Current RSS bytes |
| --- | ---: | ---: | ---: |
| Qwen2.5-1.5B Q8_0 | 1,881,584 KiB | 1,881,584 KiB | 1,926,742,016 |
| Qwen2.5-1.5B Q6_K | 1,462,576 KiB | 1,462,576 KiB | 1,497,677,824 |

## Correctness Check

Both runs produced the expected first generated token:

```text
prompt_token_ids=14990,1879
generated_token_ids=198
generated_match=true
```

## Interpretation

Both Qwen2.5-1.5B quantizations fit within the local 16 GiB-class Mac memory
budget for this short one-token CLI probe. Q6_K materially reduces retained
weight bytes, peak process RSS, and post-load current RSS compared with Q8_0,
but it was slower in this one-token generation slice.

The reported `model_file_retained_bytes=0` confirms Ferrite does not retain the
raw GGUF byte buffer after loading. The process-level RSS remains close to the
retained scalar weight bytes after the raw byte buffer is dropped. The
`/usr/bin/time -l` peak RSS is much higher because it captures load-time overlap
and allocator behavior. A longer-running steady-state sampler is still needed
before making broader memory claims across context growth, server serving, and
larger model tiers.
