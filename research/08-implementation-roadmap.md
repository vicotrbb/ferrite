# Document 8: Implementation Roadmap and Open Problems

**Research Program:** CPU-Native LLM Inference Runtime  
**Target Spec:** 9B parameter model, 2 vCPUs, 6 GB RAM, 2–5 tok/s  
**Author:** Research Agent  
**Date:** June 2025  

---

## 1. Recommended Architecture (Synthesis)

Based on the research across Documents 1–7 and 9, the following architecture is recommended:

### 1.1 Core Design Decisions

| Decision | Choice | Justification |
|----------|--------|--------------|
| Language | Rust | Memory safety, zero-cost abstractions, no GC pauses, strong SIMD support |
| Quantization format | GGUF (parse) → internal CQR-4 aligned representation | GGUF ecosystem + CPU-optimized internal layout |
| Default quantization | Q4_K_M | Best quality/size tradeoff at ~5 GB for 8-9B models |
| KV cache | Contiguous, pre-allocated, FP16 default + INT8 option | CPU cache-friendly; INT8 doubles context headroom |
| KV eviction | Sliding window + LRU session eviction | Bounded memory; simple; covers multi-session |
| Threading | Topology-adaptive (1 or 2 threads) | Avoid hyperthreading contention on cloud VMs |
| Batching | Batch size 1, sequential processing | Memory + thread constraints make batching counterproductive |
| Weight loading | mmap + madvise(MADV_SEQUENTIAL) | Near-instant load; OS page cache management |
| Memory allocator | Bump arena (256 MB) for activations | O(1) alloc/free; zero fragmentation |
| API | axum + tokio, OpenAI-compatible | Production-grade; async streaming |
| SIMD kernels | Pure Rust `core::arch` (AVX2 primary, scalar fallback) | No C FFI build complexity |
| Speculative decoding | Optional, draft model (Qwen2.5-0.5B) | Enable when base throughput < 3 tok/s |
| Prefix caching | System prompt KV cache | Multi-turn chat optimization |

### 1.2 Primary Target Model

**Llama-3.1-8B-Instruct at Q4_K_M**
- Fits 6 GB with 4096-token context (FP16 KV) or 8192 (INT8 KV)
- Standard architecture (no exotic features)
- Massive community support and GGUF model availability
- 4:1 GQA ratio, reasonable KV cache

**Secondary:** Qwen2.5-7B (more memory headroom, 7:1 GQA)
**Stretch:** Gemma-2-9B (highest quality but tight fit, requires INT8 KV)

### 1.3 What to Borrow vs Build

**Borrow (reuse with minimal modification):**
- GGUF format specification and parsing logic (from `gguf` crate or write from spec)
- Quantization scheme definitions (Q4_K_M block structure, scale encoding)
- Model architecture implementations (adapted from Candle-transformers)
- Tokenizer (`tokenizers` crate from HuggingFace)
- HTTP API patterns (from mistral.rs axum server)

**Build from scratch (no adequate crate exists):**
- SIMD kernel layer (Q4_K_M dot product, attention, RoPE, RMSNorm)
- Memory manager (mmap + madvise + bump allocator + memory budget tracking)
- KV cache manager (contiguous allocation, sliding window, INT8 quantization)
- Streaming weight executor (layer-by-layer with prefetch hints)
- Topology-adaptive thread pool

**Borrow via FFI (optional, for performance validation):**
- oneDNN matmul (as benchmark reference, not runtime dependency)
- OpenBLAS/BLIS (for FP16 fallback matmul, if Rust kernels too slow initially)

### 1.4 License Verification

| Component | License | Reuse Safety |
|-----------|---------|-------------|
| ggml.cpp (llama.cpp's tensor lib) | MIT | ✅ Safe to study, safe to port algorithms |
| llama.cpp source | MIT | ✅ Safe to borrow algorithms and re-implement |
| Candle | MIT/Apache-2.0 | ✅ Safe to adapt model implementations |
| mistral.rs | MIT | ✅ Safe to adapt server patterns |
| GGUF format spec | Public domain | ✅ No license concerns |
| Model weights (Llama, Qwen, Gemma) | Various (community, research) | ⚠️ Check per-model license |

**Recommendation:** All MIT/Apache-2.0 licensed code can be freely studied and re-implemented. No copyleft concerns (no GPL dependencies in critical path).

---

## 2. Phased Implementation Plan

### Phase 1: Proof of Concept (3 weeks)

**Goal:** Load a GGUF model, run a single forward pass, generate 1 token. Everything built from scratch.

**Deliverables:**
1. Custom GGUF file parser (zero-copy mmap access, ~400 lines)
2. Custom `f16` type with IEEE-754 conversions (~200 lines)
3. Custom `libc::mmap` wrapper for model file loading (~100 lines)
4. Model architecture implementation (Llama-3.1-8B: attention, FFN, RMSNorm, RoPE)
5. Scalar (non-SIMD) matmul reference implementation
6. Custom BPE tokenizer (~500 lines)
7. Basic token sampling (greedy + temperature)
8. CLI tool: `cargo run -- generate <model.gguf> --prompt "Hello"`

**Technical notes:**
- `libc` crate is the ONLY external dependency, raw `mmap`, `munmap`, `madvise` calls
- Custom f16 struct: wrapping `u16` with `to_f32()`/`from_f32()` using bit manipulation
- Custom `cast_slice::<T>(bytes: &[u8]) -> &[T]` for zero-copy tensor access
- Tokenizer parses HuggingFace `tokenizer.json` using our custom JSON parser
- No performance optimization, correctness first

**Build vs reuse:**
- GGUF parser: from scratch (alignment-sensitive, tied to streaming design)
- Model architectures: from scratch (tightly coupled to our memory layout)
- Tokenizer: `tokenizers` crate or from scratch (either works, not a bottleneck)
- HTTP API: `axum` + `serde` (Phase 5, not blocking)

**Success criteria:**
- Output matches llama.cpp reference output for the same model + seed
- Model loads from GGUF file (or raw safetensors for streaming mode)

**Dependencies:**
- `libc`: syscall bindings (mmap, madvise, sched_getaffinity)
- `memmap2`: mmap convenience (optional, can use raw libc)
- `half`: f16 type (optional, can build custom)
- Rust `std` and `core::arch`, SIMD intrinsics are in the standard library

### Phase 2: Optimized Kernels (3 weeks)

**Goal:** AVX2 SIMD kernels for quantized matmul and attention. Throughput reaches 2.5+ tok/s.

**Deliverables:**
1. `q4k_matmul.rs`: Q4_K_M × FP16 vector dot product using AVX2 intrinsics
   - 256-weight block processing
   - Fused dequant + multiply + accumulate
   - Scalar fallback for non-AVX2 CPUs
2. `attention.rs`: Multi-head attention with GQA support
   - QKV projection (3 matmuls or fused wider matmul)
   - Scaled dot-product attention
   - RoPE application (fused with Q/K projection if possible)
3. `norm.rs`: RMSNorm (AVX2 vectorized)
4. `activation.rs`: SiLU element-wise (AVX2)
5. Benchmark suite: kernel-level throughput measurement

**Technical approach:**
```rust
// AVX2 kernel detection at compile time
#[cfg(target_feature = "avx2")]
pub unsafe fn q4k_dot_avx2(weights: &[u8], scales: &[u8], x: &[f16], n: usize) -> f32 {
    // _mm256_load_si256, _mm256_and_si256, _mm256_maddubs_epi16, etc.
}

#[cfg(not(target_feature = "avx2"))]
pub fn q4k_dot_scalar(weights: &[u8], scales: &[u8], x: &[f16], n: usize) -> f32 {
    // Plain Rust loop
}

// Runtime dispatch
pub fn q4k_dot(weights: &[u8], scales: &[u8], x: &[f16], n: usize) -> f32 {
    #[cfg(target_feature = "avx2")]
    if is_avx2_available() {
        return unsafe { q4k_dot_avx2(weights, scales, x, n) };
    }
    q4k_dot_scalar(weights, scales, x, n)
}
```

**Success criteria:**
- AVX2 Q4_K_M dot product matches scalar reference within 0.1% (numerical accuracy)
- Achieves ≥2.5 tok/s decode on reference hardware (Xeon Ice Lake 2-core)
- Kernel throughput >10 GFLOPs (INT4 ops)

### Phase 3: Memory Management (2 weeks)

**Goal:** mmap-based weight loading, KV cache management, memory budget enforcement.

**Deliverables:**
1. `mmap.rs`: Model weight loading via mmap with madvise hints
   - `mmap(MAP_PRIVATE)` for the model file
   - `madvise(MADV_SEQUENTIAL)` after loading
   - Optional `madvise(MADV_DONTNEED)` per layer (streaming mode)
2. `kv_cache.rs`: Pre-allocated contiguous KV cache
   - Initialization with configurable max_context
   - Append (write K/V for new token)
   - Read (get K/V for attention computation)
   - Sliding window support (circular buffer mode)
3. `arena.rs`: Bump allocator for activation tensors
   - Pre-allocate 256 MB
   - O(1) alloc and reset
4. Memory budget monitor
   - Track RSS via `/proc/self/status`
   - Alert/reject when approaching limit

**Success criteria:**
- Model "loads" in <1 second (mmap is instant)
- Peak RSS < 5.5 GB for Llama-3.1-8B Q4_K_M at context 4096
- Sliding window correctly bounds KV cache (no unbounded growth)

### Phase 4: Threading and Batching (1 week)

**Goal:** Optimal threading for 2 vCPU systems, simple request queue.

**Deliverables:**
1. Topology detection (physical vs logical cores via CPUID on x86, sys on ARM)
2. Thread pool with adaptive sizing (1 or 2 threads)
3. Row-sliced parallel matmul (split output rows across threads)
4. Micro-benchmark: 1 thread vs 2 threads, pick winner
5. Simple FIFO request queue with backpressure

**Success criteria:**
- Correctly detects 1 physical core + 1 hyperthread → uses 1 worker thread
- Correctly detects 2 physical cores → uses 2 worker threads
- 2-thread configuration achieves ≥1.4× speedup over 1-thread (when applicable)

### Phase 5: API Server (1 week)

**Goal:** Production-ready OpenAI-compatible HTTP API with SSE streaming.

This phase is a required product milestone, not optional polish. See
`documentation/adr/0008-openai-compatible-http-api.md` for the durable
compatibility contract and server module boundaries.

**Deliverables:**
1. `axum`-based HTTP server with SSE streaming support
2. OpenAI-compatible endpoints: `POST /v1/chat/completions`, `POST /v1/completions`, `GET /v1/models`, `GET /health`
3. `serde`-based JSON request/response handling
4. Request queue with backpressure (bounded channel, 429 on overflow)
5. `clap`-based CLI with `toml` config file support
6. `tracing`-based structured logging
7. Graceful shutdown on SIGTERM/SIGINT

**Success criteria:**
- API overhead < 50ms per request
- Streaming delivers tokens within 100ms of generation
- Compatible with openai-python, curl, and standard OpenAI client libraries

### Phase 6: Streaming Weight Execution (2 weeks)

**Goal:** Run unquantized 9B FP16 models on 5 GB RAM via mmap streaming from NVMe disk. This is the killer differentiator.

**Deliverables:**
1. **Streaming executor**, Layer-by-layer forward pass with `madvise(DONTNEED)` after each layer
2. **Intelligent readahead**, Prefetch next layer weights from disk while computing current layer
3. **`MADV_SEQUENTIAL` hints**, Optimize kernel readahead pattern for sequential layer access
4. **Hybrid mode**, Auto-select quantized (fast) vs streaming FP16 (quality) based on RAM and request
5. **Partial resident caching**, Keep embedding/lm_head + frequently accessed small layers pinned
6. **Disk speed benchmarking**, Auto-detect SSD vs HDD, adjust streaming parameters

**Success criteria:**
- Unquantized Llama-3.1-8B FP16 generates tokens on 5 GB RAM machine
- Achieves ≥0.2 tok/s decode (with NVMe)
- Prefill throughput ≥50 tok/s after initial weight load
- No OOM kills, graceful degradation on slow disk

**Why this works:** The model file is mmap'd into virtual memory (16 GB virtual, 0 physical). Each layer (~500 MB FP16) is page-faulted from SSD as needed, computed, then released via `madvise(DONTNEED)`. Peak physical RAM: ~1.3 GB (current layer + KV cache + overhead). The SSD acts as extended memory, essentially a "memory tier between RAM and network storage."

### Phase 7: Advanced Features (2 weeks)

**Goal:** Optional performance and UX enhancements.

**Deliverables:**
1. **INT8 KV cache**, Quantize KV cache entries to INT8 (half memory, ~0.2 ppl loss)
2. **Prefix caching**, Cache system prompt KV for multi-turn speedup
3. **Speculative decoding**, Optional draft model (Qwen2.5-0.5B) for throughput boost
4. **AVX-512 VNNI kernels**, When available, 30-50% per-core speedup
5. **Memory pressure handling**, Dynamic context reduction when RAM is tight

**Success criteria:**
- INT8 KV cache: no quality regression >0.5 ppl on standard benchmarks
- Prefix caching: >50% TTFT reduction for multi-turn with shared system prompt
- Speculative decoding: >1.5× speedup when enabled (with compatible draft model)

---

## 3. Timeline Summary

| Phase | Duration | Cumulative | Deliverable |
|-------|----------|-----------|-------------|
| Phase 1: PoC | 3 weeks | 3 weeks | Scalar forward pass, GGUF loader, basic generation |
| Phase 2: Kernels | 3 weeks | 6 weeks | AVX2/AVX-512 SIMD kernels, ≥2.5 tok/s |
| Phase 3: Memory | 2 weeks | 8 weeks | mmap streaming, KV cache, arena allocator |
| Phase 4: Threading | 1 week | 9 weeks | Adaptive threading, request queue |
| Phase 5: API | 1 week | 10 weeks | axum HTTP server, OpenAI-compatible API |
| Phase 6: Streaming | 2 weeks | 12 weeks | **Unquantized 9B FP16 on 5 GB RAM** via disk streaming |
| Phase 7: Advanced | 2 weeks | 14 weeks | INT8 KV, prefix cache, speculative decoding |
| **Total** | **~14 weeks** | | **MVP + streaming milestone** |

**Buffer:** Add 2-3 weeks for debugging, profiling, and optimization → **16-17 weeks (~4 months)** for a single engineer.

**Milestone targets:**

| Milestone | When | Achievement |
|-----------|------|-------------|
| M1: First Token | Week 3 | 9B model loads, generates 1 token (Q4_K_M, 6 GB) |
| M2: Interactive | Week 6 | 3-5 tok/s decode, usable for chat |
| M3: Production | Week 10 | OpenAI API, deployment-ready |
| **M4: Extreme** | **Week 12** | **9B unquantized FP16 on 5 GB RAM** (streaming) |
| M5: Advanced | Week 14 | Speculative decoding, INT8 KV, prompt cache |

---

## 4. Open Research Problems

### 4.1 Is 2 vCPU Genuinely Viable or Physics-Bound?

**Status: RESOLVED (conditionally viable)**

From Document 7 analysis:
- Theoretical max: 5.4–8.1 tok/s for Llama-3.1-8B Q4_K_M on good cloud VMs
- llama.cpp achieves: 3.5–5.2 tok/s on similar hardware
- Our target: 3.5–5.0 tok/s (matching llama.cpp baseline)

**Conclusion:** 2 vCPU IS viable for interactive use (>2 tok/s) on properly provisioned cloud VMs (non-burstable, local SSD, AVX2+). The physics bound is memory bandwidth, not compute, and cloud VM memory bandwidth (~20-30 GB/s) is sufficient.

**Risk:** Burstable instances (t3, e2-micro) or noisy neighbor scenarios can drop below 2 tok/s. The runtime should detect and report this condition.

### 4.2 Is There a CPU-Optimied Quantization Format That Doesn't Exist Yet?

**Status: PARTIALLY RESOLVED**

From Document 3 analysis:
- Current best: Q4_K_M (GGUF) with super-block scales, already well-optimized
- Proposed CQR-4 format: 64-byte aligned blocks, optimized for AVX2 sequential access
- Potential improvement: 64-byte alignment + precomputed weight ordering for stride-1 access → 5-10% bandwidth improvement over GGUF

**Open question:** Can we design a format that encodes weights in the *access order* of the compute graph (not just tensor-major order), enabling true zero-overhead sequential streaming? This would require encoding the execution plan into the weight file layout.

### 4.3 Context Lengths >4096 on 6 GB Without Disk

**Status: PARTIALLY RESOLVED**

Solutions identified:
1. INT8 KV cache: doubles effective context (4096→8192 for Llama-3.1-8B)
2. INT4 KV cache: quadruples (but quality loss ~1-2% accuracy)
3. Sliding window: bounds context at model-native window (4096 for Gemma-2-9B)
4. Streaming weight execution: frees memory for larger KV cache

**For >8K context on 6 GB:** Not achievable without either INT4 KV (significant quality loss) or disk offloading (throughput penalty). The runtime should document this limitation clearly.

### 4.4 NUMA-Unaware 2-Hyperthread Cloud VM: Distinct Optimization Target?

**Status: RESOLVED**

On 2 hyperthreads sharing 1 physical core:
- 1 thread outperforms 2 threads for SIMD workloads (resource contention)
- Memory bandwidth is shared, no benefit from 2 threads for memory-bound work
- Distinct optimization: **always use 1 thread on hyperthreaded single-core VMs**

This is already captured in our topology-adaptive threading design.

---

## 5. Risks and Mitigations

### 5.1 Risk Matrix

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| llama.cpp already solves this well | HIGH | MEDIUM | Differentiate on memory engineering + API-first design; see differentiation strategy below |
| Memory budget impossible at 6 GB for interactive chat | LOW (proven achievable) | HIGH | Fallback to 7B model (Qwen2.5-7B) or reduce context; runtime auto-detects and warns |
| 2 vCPU throughput < 1 tok/s on bad VMs | MEDIUM | HIGH | Document minimum instance specs; runtime benchmark + warning on startup |
| Rust SIMD performance < C++ parity | LOW | MEDIUM | Profile extensively; fallback to FFI oneDNN for hot kernels if needed |
| GGUF format changes breaking compatibility | LOW | LOW | Pin GGUF reader to specific version; abstract format layer |
| Quantization quality insufficient at Q4 | LOW | MEDIUM | Support Q5_K_M as premium option; implement INT8 KV to free memory for larger quant |
| Build complexity (SIMD intrinsics) | MEDIUM | LOW | Feature-flag AVX2/AVX-512/NEON; extensive unit tests against scalar reference |

### 5.2 Differentiation Strategy: Why Not Just Use llama.cpp?

Legitimate reasons to build this runtime instead of wrapping llama.cpp:

| llama.cpp Limitation | Our Solution | User Impact |
|---------------------|-------------|-------------|
| No memory pressure handling | Proactive budget management + graceful degradation | No OOM crashes in production |
| Basic HTTP server | Production axum server with connection pooling, health checks, metrics | Production-ready deployment |
| No INT8 KV cache | INT8 KV option doubles context headroom | Longer conversations fit in RAM |
| C/C++ (memory safety) | Rust (memory safe, auditable) | No buffer overflows; security audits easier |
| Monolithic architecture | Modular crate-based design | Embeddable in larger Rust applications |
| No speculative decoding | Draft model speculative decoding | 1.5-2.5× throughput when enabled |
| Not cloud-native | Topology detection, memory-adaptive, cloud-optimized | Better performance on constrained VMs |

**Honest assessment:** For most users, `llama.cpp`'s `llama-server` is sufficient. Our runtime targets teams that:
1. Need Rust integration (embedded in a larger Rust service)
2. Need production-grade serving infrastructure
3. Need aggressive memory engineering for constrained environments
4. Want forward-looking features (BitNet) not yet in llama.cpp

---

## 6. Required Follow-Up Research

### 6.1 Empirical Experiments to Run

These experiments require hardware access and should be performed during Phase 2:

1. **Memory bandwidth measurement on target cloud VMs:**
   - Run STREAM benchmark on AWS c6i.large, GCP e2-standard-2, Azure D2s_v5
   - Establish actual bandwidth baseline (vs theoretical)

2. **llama.cpp comparison benchmark:**
   - Run `llama-bench -m llama-3.1-8b-q4_k_m.gguf -t 1 -t 2 -r 10` on target VMs
   - Establish baseline to beat

3. **AVX2 vs AVX-512 matmul throughput:**
   - Implement reference Q4_K_M kernel in both instruction sets
   - Measure on VMs with/without AVX-512

4. **INT8 KV cache quality measurement:**
   - Run MMLU benchmark with FP16 KV vs INT8 KV
   - Quantify quality loss

5. **Hyperthreading impact measurement:**
   - Pin to 1 physical core (2 HT) vs 2 physical cores
   - Measure throughput difference

### 6.2 Papers to Read

| Paper | Relevance | Priority |
|-------|-----------|----------|
| "FlashAttention-3: Fast and Accurate Attention with Asynchronism and Low-precision Math" (2024) | Attention optimization applicable to CPU | MEDIUM |
| "KIVI: A Tuning-Free Asymmetric 2bit Quantization for KV Cache" (2024) | INT2 KV cache techniques | HIGH (if INT4 KV is insufficient) |
| "EfficientDM: An Efficient Diffusion Model Accelerator Using 4-bit Quantization" (2024) | Quantization techniques generalizable to LLMs | LOW |
| "Atom: Low-bit Quantization for Efficient and Accurate LLM Serving" (2024) | INT4 activation quantization | MEDIUM |
| "FlatAttention: Efficient KV Cache Compression for Long Context LLMs" (2025) | KV cache compression techniques | HIGH |
| "SpinQuant: LLM Quantization with Learned Rotations" (2024) | Rotation-based quantization (QuIP# variant) | MEDIUM |
| "SageAttention: Lossless 8-bit Attention" (2024) | INT8 attention without quality loss | HIGH |

---

## 7. Implementation Implications

### 7.1 Critical Path

The critical path to production-ready MVP:
```
Phase 1 (2w) → Phase 2 (3w) → Phase 3 (2w) → Phase 4 (1w) → Phase 5 (1w)
```
Phase 6 (advanced features) can be deferred without blocking the MVP.

**Earliest MVP date (single engineer full-time):** 9 weeks from start.

### 7.2 First Week Action Items

1. Create Cargo workspace with crate structure from Document 5
2. Implement GGUF parser (use `gguf` crate or write from spec)
3. Download Llama-3.1-8B-Instruct.Q4_K_M.gguf for development
4. Implement RMSNorm scalar reference (simplest layer to validate pipeline)

### 7.3 Success Metrics Dashboard

Track continuously during development:
- Decode tok/s on reference hardware per commit
- Peak RSS per commit
- Kernel test pass rate (correctness)
- Unsafe line count (minimize, track)
- API latency overhead per commit

---

*This roadmap synthesizes the complete research into actionable implementation steps. The next document (Document 9: Rust Ecosystem) provides the definitive crate landscape and gap analysis for implementation.*
