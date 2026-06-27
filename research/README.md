# CPU-Native LLM Inference Runtime — Research Knowledge Base

**Target Spec:** Run a 9B parameter model (Qwen2.5-7B, Llama-3.1-8B, Gemma-2-9B) on 2 vCPUs and 6 GB RAM, delivering 2–5 tokens/second for interactive use.

**Implementation Language:** Rust

**Date:** June 2025

---

## Overview

This research knowledge base provides the engineering foundation for building a CPU-native LLM inference runtime from scratch. It consists of 9 deeply-researched documents covering every aspect of the system — from state-of-the-art survey through memory architecture, quantization formats, SIMD kernels, runtime design, model compatibility, benchmarking, implementation roadmap, and Rust ecosystem readiness.

A software engineer (or AI implementation agent) can read these documents sequentially and proceed directly to building a working prototype.

---

## Documents

| # | Document | Description | Key Findings |
|---|----------|-------------|-------------|
| 1 | [State of the Art](01-state-of-the-art.md) | Survey of llms.cpp, vLLM, OpenVINO, ONNX, Candle, mistral.rs, etc. | llama.cpp achieves 3.5-5.2 tok/s at 2 vCPU; Rust ecosystem at ~60-70% |
| 2 | [Memory Architecture](02-memory-architecture.md) | Budget analysis for 9B model in 6 GB RAM | Llama-3.1-8B at Q4_K_M fits at context 4096; streaming weights essential for larger models |
| 3 | [Quantization Pipeline](03-quantization-pipeline.md) | Formats (GGUF, GPTQ, AWQ, QuIP#, BitNet), quality benchmarks | Q4_K_M is the quality floor; BitNet b1.58 is the future |
| 4 | [Compute Kernels](04-compute-kernels.md) | AVX2/AVX-512 SIMD, threading, roofline analysis | Workload is memory-bound at 0.8 FLOPs/byte; 2-thread speedup ≈1.5× |
| 5 | [Inference Engine Architecture](05-inference-engine-architecture.md) | Runtime design, batching, KV cache, speculative decoding | Batch size 1, contiguous KV, no PagedAttention on CPU |
| 6 | [Model Architecture Compatibility](06-model-architecture-compatibility.md) | Qwen2.5-7B, Llama-3.1-8B, Gemma-2-9B, Phi, MoE analysis | Llama-3.1-8B is optimal target; Qwen2.5-7B has best memory margin |
| 7 | [Benchmarks and Baselines](07-benchmarks-and-baselines.md) | Real benchmark data, performance targets, gap analysis | Achievable: 3.5-6.1 tok/s; our target 3.5-5.0 tok/s matches llama.cpp |
| 8 | [Implementation Roadmap](08-implementation-roadmap.md) | 6-phase plan, risks, open problems | 11 weeks (13 with buffer) for single engineer to production MVP |
| 9 | [Rust Ecosystem](09-rust-ecosystem.md) | Crate landscape, FFI analysis, gap analysis | Ecosystem 3.6/5 ready; SIMD kernels and KV cache must be custom-built |
| 10 | [Theoretical Frontiers](10-theoretical-frontiers.md) | **27 novel theories** — information theory limits, spectral decomposition, cache-resident attention, weight manifolds, asymmetric threading | Theoretical max ~10-15 tok/s; 9B unquantized at 1-2 tok/s possible with combined theories |
| 11 | [Testing Model Registry](11-testing-model-registry.md) | **17 open-weight models** across 5 tiers — from 135M bring-up to 32B streaming | Progressive testing: SmolLM2→Qwen2.5→Llama→Phi→Mistral→Gemma |

---

## Key Conclusions

### Is 9B @ 2 vCPU / 6 GB Physically Possible?

**YES, conditionally.**

- **Llama-3.1-8B at Q4_K_M with 4096 context:** 4.9 GB weights + 512 MB KV + 300 MB overhead = 5.7 GB ✅
- **Qwen2.5-7B at Q4_K_M with 8192 context:** 4.1 GB weights + 459 MB KV + 300 MB = 4.9 GB ✅
- **Gemma-2-9B at Q4_K_M:** Requires INT8 KV cache to fit; marginal at context 1024-2048

### What Throughput is Achievable?

| Model | 2 vCPU Decode | Prefill | Confidence |
|-------|--------------|---------|-----------|
| Llama-3.1-8B Q4_K_M | 3.5–5.0 tok/s | 40–60 tok/s | HIGH |
| Qwen2.5-7B Q4_K_M | 4.5–6.0 tok/s | 50–70 tok/s | HIGH |
| Gemma-2-9B Q4_K_M | 2.5–3.5 tok/s | 30–45 tok/s | MEDIUM |

### What Should the Runtime Do Differently from llama.cpp?

1. **Memory engineering:** mmap + madvise hints, INT8 KV cache, streaming weight execution
2. **Cloud-native:** Topology detection, memory pressure handling, graceful degradation
3. **Rust safety:** No buffer overflows, auditable unsafe surface (~850 lines)
4. **API-first:** Production axum server vs llama.cpp's basic HTTP
5. **Forward-looking:** BitNet/ternary format support before models appear

### What Format for Weight Storage?

- **Primary:** Parse GGUF, reorganize to internal CQR-4 format with 64-byte alignment
- **Block structure:** 256 weights, super-block scales (same as Q4_K_M), AVX2-optimized access pattern
- **Alignment:** 64-byte (cache line) for all tensor data boundaries
- **Loading:** mmap with MADV_SEQUENTIAL hint; optional MADV_DONTNEED per-layer for streaming

### Continuous Batching: Yes or No?

**NO at 2 threads.** Single-stream FIFO queue is optimal. The memory overhead of multiple KV caches and scheduling complexity outweigh the negligible throughput gain at batch sizes 1-2 on 2 vCPUs.

### KV Cache: RAM, mmap, or Disk?

**RAM (pre-allocated contiguous):** FP16 by default, INT8 optional. For models where weight + KV exceeds RAM, use streaming weight execution with mmap + madvise(DONTNEED). Disk-backed KV is a last resort (50% throughput penalty on NVMe, unusable on HDD).

### Speculative Decoding Worth It?

**Optional.** When base throughput < 3 tok/s, a 0.5B draft model (Qwen2.5-0.5B, ~0.3 GB) can provide 1.5-2.5× speedup. On CPU, speculative decoding is MORE effective than GPU because batched verification amortizes the memory-bound forward pass.

### Rust Ecosystem Split?

| Inference Core (from scratch) | Infrastructure (production crates) |
|-------------------------------|-----------------------------------|
| SIMD matmul kernels (AVX2/AVX-512/NEON) | HTTP server (`axum` + `tokio`) |
| Streaming weight executor (mmap + madvise) | JSON (`serde` + `serde_json`) |
| KV cache manager (contiguous + sliding + INT8) | CLI (`clap`) |
| Model architectures (Llama, Qwen2, Gemma) | Config (`toml`) |
| GGUF parser (alignment-sensitive) | Logging (`tracing`) |
| Sampler / speculative decoding | mmap wrapper (`memmap2`) |
| Topology detector | FP16 type (`half`) |

---

## Killer Milestone: Unquantized 9B on 5 GB RAM

**The differentiating feature.** Using mmap streaming with `madvise(MADV_DONTNEED)`, the runtime can:
- Load a 16 GB FP16 model into virtual memory (instant, no physical RAM)
- Process one layer at a time (~500 MB per layer in RAM)
- Release each layer back to disk after computation
- **Peak physical RAM: ~1.3 GB** (current layer + KV cache + overhead)
- Achieve 0.2–0.5 tok/s decode on NVMe SSD

This means **no quantization loss** on minimal hardware — a unique capability no other runtime offers. Users choose: Q4_K_M at 3-5 tok/s for interactive chat, or FP16 streaming at 0.2-0.5 tok/s for maximum quality.

---

## Reading Order

For the implementation agent:

1. **Start here** → Read Document 8 (Roadmap) for the implementation plan
2. **Understand the system** → Read Documents 1-2 (State of Art + Memory)
3. **Choose formats** → Read Document 3 (Quantization)
4. **Build kernels** → Read Documents 4, 9 (Kernels + Rust Ecosystem) simultaneously
5. **Design architecture** → Read Documents 5-6 (Engine + Models)
6. **Validate** → Read Document 7 (Benchmarks) for testing methodology

---

## Quick Reference: Recommended Stack

**Philosophy:** Inference core from scratch. Infrastructure via production crates.

```toml
[dependencies]
# --- Inference core (from scratch, no crate deps) ---
# SIMD kernels, KV cache, streaming executor, model architectures
# Built with: std, core::arch, libc

# --- Infrastructure (production crates) ---
axum = { version = "0.7", features = ["tokio"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
memmap2 = "0.9"
half = "2"
```

```
# Build
RUSTFLAGS="-C target-cpu=native -C opt-level=3" cargo build --release

# Run
./cpu-llm-runtime --model Llama-3.1-8B-Instruct.Q4_K_M.gguf --port 8080
```

---

## Open Questions for Future Research

1. What is the optimal CQR-4 block size for AMD Epyc vs Intel Xeon memory controllers?
2. Can FlashAttention-3 concepts be adapted for CPU with paged memory?
3. When will 7B+ BitNet models be available, and what format should we use?
4. Is INT4 KV cache viable for production use (quality floor)?
5. How does speculative decoding interact with our streaming weight execution?

---

*Total corpus: ~55,000 words across 9 documents + README*
