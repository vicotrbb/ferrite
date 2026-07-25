# Rejected session workspace experiment

Date: 2026-07-25

## Scope

This experiment tested reusable normalization and attention buffers owned by
each inference session. The candidate preserved the public interface and the
attention arithmetic order. For a model with `L` transformer layers, the
single-stream path could avoid one normalization allocation and `2L` attention
allocations per token after its score buffer reached sufficient capacity. The
batched path could avoid the corresponding per-session buffers.

The candidate also retained two hidden-width vectors per session and a score
vector that grew with context length. Allocation count alone did not establish
lower CPU time, lower peak memory, or better request latency.

## Fixed inputs

- Host: Apple M5 Pro, 15 logical CPUs, 24 GiB RAM
- OS: macOS 26.5.2 arm64, build 25F84
- Toolchain: Rust 1.96.0, LLVM 22.1.2
- Source base: `ced523b43762b0bad52702acbb19ba345d459263`
- Cargo profile: repository `release` profile, no `RUSTFLAGS`
- Baseline CLI SHA-256:
  `6986884686d0dbe3744e947ab51d76282f7d20f39b63e8e1e17b317cd391b7a1`
- Candidate CLI SHA-256:
  `f01c19630890bd31f62a57e50d0aa37efecc2ad83357c6cfb4879c52af6fbb60`
- Model: official Microsoft Phi-3 Mini 4K Instruct Q4
- Model size: 2,393,231,072 bytes
- Model SHA-256:
  `8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef`
- Prompt:
  `<|user|>\nWrite one sentence about Rust.<|end|>\n<|assistant|>`
- Workers: seven
- Kernel provider: automatic

## Correctness evidence

Focused tests compared attention output with the independent allocating oracle
and checked that fixed-width workspace pointers and capacities remained stable.
The existing batched-decode suite retained bit-identical sequential and batch
results. Baseline and candidate release binaries also produced the same ordered
32-token Phi-3 trace:

```text
504,338,263,6757,8720,4086,21309,373,15332,29892,6210,29892,322,3022,10880,29892,8688,304,5557,3619,24557,322,23180,11614,1476,297,916,10276,29889,32007,32001,390
```

Correctness parity passed for the covered inputs. It did not establish a
performance or memory improvement.

## Diagnostic timing

Command:

```sh
<baseline-or-candidate-binary> \
  --model /absolute/path/to/Phi-3-mini-4k-instruct-q4.gguf \
  --prompt '<|user|>
Write one sentence about Rust.<|end|>
<|assistant|>' \
  --benchmark-runs 8 \
  --threads 7
```

The order was candidate, baseline, candidate, baseline, baseline, candidate.
Every run produced the same eight token IDs.

| Variant | Average step samples | Median step | Diagnostic rate |
| --- | --- | ---: | ---: |
| Baseline | 345.927, 338.762, 336.351 ms | 338.762 ms | 2.95 tok/s |
| Session workspace | 346.106, 353.177, 334.980 ms | 346.106 ms | 2.89 tok/s |

The candidate median was 2.17% slower. This is not accepted performance
evidence because the clean-host preflight rejected the machine. Surrounding
snapshots reported one-minute load per core from 0.510 to 1.231 against the
0.250 limit, with one background process reaching 719.9% CPU. CPU consumption,
RSS, time to first token, and tail latency were not sampled for these runs.

## Decision

Rejected and reverted. The candidate reduced a deterministic category of
temporary allocations, but it did not pass Ferrite's clean repeated-median
promotion gate and increased retained per-session scratch capacity. No runtime
code from this experiment remains in the repository.

Reconsider session-owned workspaces only after allocation profiling shows these
sites are material and a clean interleaved evaluation proves non-regressing
throughput, latency, CPU, and memory for single-stream and batched workloads.
