# 2026-06-28 Tier 1 Qwen2.5 1.5B Q8_0 and Q6_K KV Growth

## Scope

This benchmark records bounded local KV-cache growth for Ferrite's CLI running
Qwen2.5-1.5B-Instruct Q8_0 and Q6_K.

This is a prompt-length KV-cache accounting probe. It does not prove full
long-context behavior, server KV-cache behavior, cache eviction, sliding-window
behavior, or memory pressure under large contexts.

## Tree State

- Branch: `main`
- Commit before run: `423bf75`
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
- Prompt source: explicit `--prompt-token-ids`
- Prompt-token counts: 2, 16, and 64
- Token IDs:
  - first token: `14990`
  - remaining prompt tokens: repeated `1879`
- Generation limit: one token
- Cached-token count: prompt-token count plus one generated token
- Process summary: `/usr/bin/time -l`

Command shape:

```sh
/usr/bin/time -l target/release/ferrite \
  --model target/models/qwen2.5-1.5b-instruct-q8_0.gguf \
  --prompt-token-ids '<ids>' \
  --generate-tokens 1
```

The Q6_K runs used the same command shape with
`target/models/qwen2.5-1.5b-instruct-q6_k.gguf`.

## Results

| Model | Prompt tokens | Cached tokens | Generated token | KV cache bytes | Bytes per cached token | Max RSS | Peak footprint | Real time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Q8_0 | 2 | 3 | 198 | 172,032 | 57,344 | 3,821,240,320 | 3,822,690,560 | 1.03 s |
| Q8_0 | 16 | 17 | 1879 | 974,848 | 57,344 | 3,820,781,568 | 3,822,231,808 | 2.00 s |
| Q8_0 | 64 | 65 | 1879 | 3,727,360 | 57,344 | 3,821,158,400 | 3,822,592,192 | 6.00 s |
| Q6_K | 2 | 3 | 198 | 172,032 | 57,344 | 2,962,046,976 | 2,963,086,016 | 1.22 s |
| Q6_K | 16 | 17 | 1879 | 974,848 | 57,344 | 2,961,899,520 | 2,962,938,560 | 4.70 s |
| Q6_K | 64 | 65 | 1879 | 3,727,360 | 57,344 | 2,961,457,152 | 2,962,479,744 | 17.28 s |

All runs reported:

```text
model_file_retained_bytes=0
```

The retained scalar weight bytes were stable per quantization:

```text
q8_0 scalar_weight_bytes=1888581632
q6_k scalar_weight_bytes=1458228224
```

## Interpretation

Ferrite's Qwen2.5-1.5B session KV-cache accounting grows linearly across this
bounded local prompt-length probe: every sampled cached-token state reports
`57,344` KV-cache bytes per cached token for both Q8_0 and Q6_K. The identical
per-token KV cost is expected because the cache shape follows the model
architecture, not the weight quantization.

The process peak RSS stays dominated by model loading and retained weights for
these small prompt lengths. The KV-cache delta from 3 to 65 cached tokens is
about 3.39 MiB, which is too small to materially change the process peak on
this host.

This narrows the Tier 1 KV-cache growth evidence gap for Qwen2.5-1.5B, but it
does not prove large-context memory pressure, server-context growth, cache
eviction, or bounded behavior near the model context limit.
