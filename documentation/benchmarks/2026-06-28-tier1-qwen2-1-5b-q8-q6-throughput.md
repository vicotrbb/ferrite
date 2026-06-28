# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 and Q6_K Throughput

## Scope

This benchmark records bounded local throughput for the Qwen2.5-1.5B Q8_0 and
Q6_K artifacts after their local and x86_64 AVX2 six-prompt parity checks.

This is benchmark evidence only. It does not prove the Tier 1 throughput gate:
both quantizations remain below the 10 tok/s target on this local host. The
follow-up x86_64 AVX2 pod run is recorded separately in
`documentation/benchmarks/2026-06-28-tier1-avx2-qwen2-1-5b-q8-q6-throughput.md`.

## Tree State

- Branch: `main`
- Commit: `fa912df`
- Working tree before run: clean

## Models

- Repo: `Qwen/Qwen2.5-1.5B-Instruct-GGUF`
- Q6_K local path: `target/models/qwen2.5-1.5b-instruct-q6_k.gguf`
- Q8_0 local path: `target/models/qwen2.5-1.5b-instruct-q8_0.gguf`

Ferrite reported:

```text
q6_k model_file_bytes=1464178720
q6_k scalar_weight_bytes=1458228224
q8_0 model_file_bytes=1894532128
q8_0 scalar_weight_bytes=1888581632
kv_cache_bytes=401408
```

## Protocol

- Host: local macOS aarch64
- Prompt: `hello world`
- Prompt token IDs: `14990,1879`
- Benchmark runs: 5 repeated token-id decode steps after initial prompt
  next-token computation
- Memory evidence: `/usr/bin/time -l`

Commands:

```sh
/usr/bin/time -l target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --prompt 'hello world' \
  --benchmark-runs 5

/usr/bin/time -l env RAYON_NUM_THREADS=2 target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q6_k.gguf \
  --prompt 'hello world' \
  --benchmark-runs 5

/usr/bin/time -l target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --benchmark-runs 5

/usr/bin/time -l env RAYON_NUM_THREADS=2 target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt 'hello world' \
  --benchmark-runs 5
```

All four runs produced:

```text
next_token_id=198
benchmark_cached_tokens=7
benchmark_token_ids=9707,11,1879,0,2585
experimental_q8_k_activation_matvec=false
compare_q8_k_activation_matvec=false
```

## Results

| Model | Thread setting | benchmark_avg_ns | Approx tok/s | Real time | User time | Max RSS | Peak footprint |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Q6_K | default pool | 271,304,008 | 3.69 | 4.35 s | 5.03 s | 2,449,014,784 bytes | 2,963,643,264 bytes |
| Q6_K | `RAYON_NUM_THREADS=2` | 451,274,383 | 2.22 | 4.63 s | 4.18 s | 2,422,538,240 bytes | 2,963,676,096 bytes |
| Q8_0 | default pool | 154,366,350 | 6.48 | 3.40 s | 1.25 s | 2,256,715,776 bytes | 3,823,444,480 bytes |
| Q8_0 | `RAYON_NUM_THREADS=2` | 156,033,066 | 6.41 | 3.33 s | 1.26 s | 2,510,667,776 bytes | 3,823,034,880 bytes |

## Interpretation

Q8_0 is the faster of the two newly added Qwen2.5-1.5B quantizations on the
local aarch64 host for this prompt. The default-pool Q8_0 run measured about
154.4 ms per benchmark token, or about 6.48 tok/s. The two-thread Q8_0 run was
similar at about 6.41 tok/s.

Q6_K was slower: about 3.69 tok/s on the default pool and about 2.22 tok/s with
`RAYON_NUM_THREADS=2`.

This reinforces the existing Tier 1 throughput status: Qwen2.5-1.5B remains
below the 10 tok/s target in local bounded runs. The next throughput slice
should use profile evidence to isolate why Q8_0 is faster here and whether a
specific hot path can be improved without regressing correctness.
