# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 and Q6_K Memory

## Scope

This benchmark records bounded local memory posture for Ferrite's CLI running
Qwen2.5-1.5B-Instruct Q8_0 and Q6_K.

This is a memory accounting and process RSS note. It does not prove full Tier 1
memory posture, server concurrency memory behavior, long-context memory growth,
or larger-model readiness.

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
- Measurement: `/usr/bin/time -l` process summary plus Ferrite's component
  memory accounting

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

## Results

| Model | model_file_bytes | model_file_retained_bytes | scalar_weight_bytes | kv_cache_bytes | Max RSS | Peak footprint | Real time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Qwen2.5-1.5B Q8_0 | 1,894,532,128 | 0 | 1,888,581,632 | 172,032 | 3,821,076,480 | 3,822,526,784 | 0.83 s |
| Qwen2.5-1.5B Q6_K | 1,464,178,720 | 0 | 1,458,228,224 | 172,032 | 2,961,850,368 | 2,962,856,576 | 1.23 s |

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
weight bytes and process RSS compared with Q8_0, but it was slower in this
one-token generation slice.

The reported `model_file_retained_bytes=0` confirms Ferrite does not retain the
raw GGUF byte buffer after loading. The process-level RSS remains close to the
sum of raw file bytes plus retained scalar weight bytes because `/usr/bin/time`
captures peak load-time overlap and allocator behavior. A steady-state sampler
or dedicated in-process memory probe is still needed before making stronger
steady-state memory claims.
