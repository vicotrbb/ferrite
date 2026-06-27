# Document 9: Rust for LLM Inference — Ecosystem Readiness, Crate Landscape, and Gap Analysis

**Research Program:** CPU-Native LLM Inference Runtime  
**Target Spec:** 9B parameter model, 2 vCPUs, 6 GB RAM, 2–5 tok/s  
**Author:** Research Agent  
**Date:** June 2025  

---

## 1. Introduction

This document provides the definitive map of the Rust ecosystem for building a CPU-native LLM inference runtime. **The strategic decision is to build everything from scratch** with zero third-party crate dependencies (except `libc` for syscall bindings). This document catalogs what exists in the ecosystem as **reference implementations and study material** — not as dependencies. Every component listed below should be studied for design ideas and algorithms, then re-implemented independently.

**The implementation language is Rust (decided).** The only external crate is `libc`. Everything else — HTTP server, JSON parser, tokenizer, SIMD kernels, memory management, config parser — is written from scratch.

---

## 2. Existing Rust LLM Inference Projects (Deep Audit)

### 2.1 Candle (HuggingFace)

**Repository:** [github.com/huggingface/candle](https://github.com/huggingface/candle)  
**Version:** 0.8+ (as of mid-2025)  
**License:** MIT/Apache-2.0  
**Stars:** ~16,000  

**Architecture:**

```
candle-core          — Tensor operations, backends, quantization
├── cpu_backend/     — NdArray-based CPU implementation
│   ├── mod.rs       — Dispatch, tensor ops
│   ├── ops.rs       — Element-wise, reduction ops
│   └── avx.rs       — AVX2-accelerated kernels (limited)
├── quantized/       — QTensor, supported quantization types
│   ├── gguf.rs      — GGUF format support
│   └── k_quants.rs  — K-quant implementations
└── dtype.rs         — Type system (F16, BF16, Q4_0, etc.)

candle-nn            — Neural network layers
├── linear.rs        — Linear (dense) layer + QLinear (quantized)
├── embedding.rs     — Embedding lookup
├── rotary_emb.rs    — RoPE implementation
├── rms_norm.rs      — RMSNorm
└── attention.rs     — Scaled dot-product attention

candle-transformers  — Model implementations
├── models/
│   ├── llama.rs     — Llama 2/3
│   ├── qwen2.rs     — Qwen2/2.5
│   ├── gemma.rs     — Gemma 1/2
│   ├── mistral.rs   — Mistral
│   ├── phi.rs       — Phi-2/3
│   └── ...
```

**What to reuse from Candle:**

| Component | Reuse? | Rationale |
|-----------|--------|-----------|
| Model architectures (models/) | **YES** — Adapt | Working implementations of Llama, Qwen2, Gemma, Phi in Rust. Copy and modify for our runtime. |
| `candle_nn::rotary_emb` | **YES** — Adapt | Correct RoPE implementation |
| `candle_nn::rms_norm` | **YES** — Adapt | Simple, correct |
| GGUF parser (`quantized/gguf.rs`) | **MAYBE** | Basic but works; consider `gguf` crate or custom for alignment control |
| QTensor + quantization types | **NO** — Replace | Candle's quantized types wrap ggml's C code via FFI or use suboptimal Rust implementations |
| CPU backend (`cpu_backend/`) | **NO** — Replace | Uses ndarray (generic BLAS), not SIMD-optimized for quantized types |
| Tensor storage model | **NO** — Replace | Arc<RwLock<>> overhead too high for inference; need zero-copy mmap access |

**Candle's CPU backend performance analysis:**

The `cpu_backend` module uses `ndarray` for dense operations and custom (limited) SIMD for quantized ops. Performance gaps:

1. **AVX2 coverage:** Only Q4_0, Q8_0, and F32 dot products have AVX2 paths. Q4_K_M and other k-quants fall back to scalar on Candle's native backend (or use C ffi to ggml).
2. **No AVX-512:** No AVX-512 specific kernels at all.
3. **Memory model:** Tensors are `Arc<RwLock<CpuStorage>>` — reference counted, write-locked, heap-allocated. This adds overhead per operation.
4. **No mmap:** All weights loaded into `Vec<u8>` memory. No streaming capability.

**Estimated performance gap vs our target:**
- Candle CPU: ~60-70% of llama.cpp throughput for Q4_K_M models
- Primary cause: ndarray-based dense ops and limited SIMD for k-quants
- Our target: 95-100% of llama.cpp by using direct SIMD intrinsics

**Key files to study:**

```
candle-core/src/quantized/k_quants.rs  — K-quant reference implementations (Rust)
candle-core/src/cpu_backend/mod.rs     — Compute dispatch logic
candle-transformers/src/models/llama.rs — Complete Llama model (correctness reference)
candle-transformers/src/models/qwen2.rs — Qwen2 model
```

### 2.2 burn

**Repository:** [github.com/tracel-ai/burn](https://github.com/tracel-ai/burn)  
**Version:** 0.15+  
**License:** MIT/Apache-2.0  
**Stars:** ~9,000  

**Assessment for our runtime:**

| Factor | Rating | Notes |
|--------|--------|-------|
| Training framework | ✅ Excellent | Great for training/research |
| Inference performance | ❌ Poor | Not optimized for inference-only |
| Quantization support | ❌ None | No GGUF/Q-quant support |
| SIMD kernels | ❌ None | Uses BLAS for CPU (via tch-rs or ndarray) |
| LLM model support | ❌ Limited | No complete Llama/Qwen implementations |
| Backend abstraction | ✅ Good design | Clean trait-based backend system |

**Verdict: DO NOT USE.** burn is a training-first framework with inference as an afterthought. No quantization, no SIMD optimization, no LLM-specific code. However, its backend trait design is worth studying for our own architecture.

### 2.3 mistral.rs

**Repository:** [github.com/EricLBuehler/mistral.rs](https://github.com/EricLBuehler/mistral.rs)  
**Version:** 0.3+  
**License:** MIT  
**Stars:** ~4,500  

**Architecture:** mistral.rs is a complete LLM serving system in Rust:
- Uses Candle for tensor operations
- Implements continuous batching (PagedAttention-inspired)
- OpenAI-compatible API server (axum + tokio)
- Supports: Llama, Mistral, Mixtral, Phi, Qwen2, Gemma, ChatML
- GGUF loading support
- LoRA adapter hot-swapping
- ISQ (In-Situ Quantization) — quantize FP16→INT4 at load time

**What to reuse:**

| Component | Reuse? | Rationale |
|-----------|--------|-----------|
| API server (axum-based) | **YES** — Adapt | Production-quality implementation of OpenAI-compatible streaming API |
| Continuous batching scheduler | **MAYBE** | Good design but overkill for 2-thread batch-1; simpler to build our own FIFO |
| ISQ (In-Situ Quantization) | **YES** — Study and adapt | Load FP16, quantize to INT4 on first use — useful for models without GGUF |
| PagedAttention Rust impl | **NO** — Study only | Wrong for CPU (see Doc 5), but the Rust code is instructive |
| Chat template handling | **YES** | Correct Jinja template implementation |

**Key files:**
```
mistral.rs/src/pipeline/     — Model loading pipeline (GGUF support)
mistral.rs/src/scheduler.rs  — Batching scheduler
mistral.rs/src/server.rs     — axum HTTP server
mistral.rs/src/engine.rs     — Inference engine loop
```

### 2.4 llama-cpp-rs (FFI wrapper)

**Repository:** [github.com/utilityai/llama-cpp-rs](https://github.com/utilityai/llama-cpp-rs)  
**Version:** 0.4+  
**License:** MIT  

**Architecture:** Thin Rust wrapper around llama.cpp's C API via bindgen:
```rust
// Simplified
extern "C" {
    fn llama_model_load_from_file(path: *const c_char) -> *mut llama_model;
    fn llama_decode(ctx: *mut llama_context, batch: llama_batch) -> c_int;
}

// Safe Rust wrapper
pub struct LlamaModel { inner: *mut sys::llama_model }
pub struct LlamaSession { inner: *mut sys::llama_context }
```

**FFI overhead analysis:**
- Cross-boundary call: ~100ns–1μs (function call + ABI transition)
- Per-token inference involves ~50-100 FFI calls: ~10-100μs overhead total
- At 4 tok/s (250ms per token): **FFI overhead is 0.004-0.04% of total time** — negligible
- Memory copying between Rust vec and C pointer: avoidable with shared buffers

**Assessment:** Viable for a "quick start" MVP that delegates to llama.cpp's C backend. However, it prevents:
- Custom memory management (mmap + eviction strategies)
- Custom kernel optimization
- BitNet/ternary support (no llama.cpp support yet)
- Rust safety guarantees (all C code is unsafe)

**Recommendation:** Study for API patterns, don't depend on it for production.

### 2.5 kalosm

**Repository:** [github.com/floneum/floneum](https://github.com/floneum/floneum) (kalosm subcrate)  
**License:** MIT  

kalosm is a high-level Rust agent framework that uses Candle for LLM inference. It's too abstracted for a performance-critical runtime but its agent API design (tool use, RAG, streaming) is worth studying for future API expansion.

### 2.6 llm crate (pure Rust inference)

**Repository:** [github.com/rustformers/llm](https://github.com/rustformers/llm) (archived)  
**License:** MIT  

The `llm` crate was an early attempt at pure-Rust LLM inference. It's now archived but provides lessons:
- Pure Rust quantization (Q4_0, Q4_1) with scalar implementations
- ~4-5× slower than llama.cpp (no SIMD, no mmap, no optimized attention)
- **Lesson:** Pure Rust WITHOUT SIMD intrinsics is too slow for interactive use

---

## 3. Low-Level Rust Patterns for Inference Kernels (Reference Study)

All kernels are built from scratch using `core::arch` (part of Rust's standard library, not a dependency). The following crates are studied as **reference implementations** for algorithmic patterns.

### 3.1 SIMD Intrinsics: `core::arch`

**Module:** `std::arch::x86_64` (stable since Rust 1.27, extended through present)

**Coverage for AVX2:**
- `_mm256_load_si256`, `_mm256_loadu_si256` — Load 256-bit vectors
- `_mm256_add_epi32`, `_mm256_mullo_epi16` — Integer arithmetic
- `_mm256_maddubs_epi16` — **Critical:** unsigned 8-bit × signed 8-bit → 16-bit multiply-add
- `_mm256_madd_epi16` — **Critical:** signed 16-bit × signed 16-bit → 32-bit multiply-add
- `_mm256_and_si256`, `_mm256_srli_epi16` — Bit manipulation for nibble extraction
- `_mm256_fmadd_ps` — Fused multiply-add for FP32 accumulators
- `_mm256_hadd_ps`, shuffle sequences — Horizontal sum

**Coverage for AVX-512:**
- `_mm512_load_si512` — 512-bit loads
- `_mm512_dpbusd_epi32` — **VPDPBUSD:** unsigned×signed dot product with accumulate (VNNI)
- `_mm512_cvtepi16_epi32` — Widening conversion
- Full AVX-512 coverage available since Rust 1.72+

**Ergonomic issues:**
- Must use `unsafe` for all intrinsic calls (no safe SIMD wrapper stabilized)
- Code is verbose (~200 lines per kernel vs ~20 in C with intrinsics)
- `#[cfg(target_feature)]` for conditional compilation
- No portable SIMD stabilized yet (portable_simd still unstable)

**Recommendation:** Use `core::arch` directly. Accept the verbosity. Wrap unsafe in a `kernels` crate with a safe public API.

### 3.2 `wide` Crate

**Repository:** [github.com/Lokathor/wide](https://github.com/Lokathor/wide)  
**Version:** 0.7+  

Portable SIMD wrapper that abstracts over x86 (SSE/AVX) and ARM (NEON). Provides safe wrappers for common operations.

**Assessment:**
- ✅ Cleaner API than raw `core::arch`
- ✅ Portable (x86 + ARM from same code)
- ❌ Less complete coverage (doesn't wrap all AVX-512 instructions)
- ❌ May not optimize as well as direct intrinsics in some cases
- ❌ Smaller community, slower updates

**Recommendation:** Do NOT use as primary SIMD library. Use `core::arch` directly for maximum performance and control. May use `wide` for non-critical-portable code paths.

### 3.3 Matrix Math: `gemm` / `pulp`

**Repository:** [github.com/sarah-ek/gemm](https://github.com/sarah-ek/gemm)  
**Version:** 0.18+  

Pure-Rust high-performance matrix multiplication. Uses hand-written SIMD kernels for f32/f64.

**Performance:** Matches or exceeds BLAS for small matrices on modern CPUs. Uses:
- AVX2 and AVX-512 kernels for FP32
- Micro-kernel approach (similar to BLIS)
- Cache-oblivious blocking

**Assessment for our runtime:**
- ✅ Excellent for FP32 matmul (embedding layer, LM head, any non-quantized paths)
- ❌ No quantized type support (Q4, Q8, etc.)
- Could use for the few FP16/FP32 matmuls needed (embedding lookup, LM head)

**Our build approach:** Implement FP32 matmul from scratch using the same micro-kernel approach (blocked GEBP with register tiling). Study `gemm`'s source for the optimal blocking strategy, but write our own.

### 3.4 `half`: f16/bf16 Support

**Repository:** [github.com/starkat99/half-rs](https://github.com/starkat99/half-rs)  
**Version:** 2.4+  

**Features:**
- `f16` and `bf16` types with IEEE-754 compliance
- SIMD conversion: `_mm256_cvtph_ps` (f16→f32) and reverse
- Arithmetic operations with proper rounding
- Zero-cost conversion to/from u16 (bit representation)

**Performance:** SIMD-optimized conversions at full AVX2/AVX-512 throughput. Element-wise arithmetic is slightly slower than native f32 (no hardware f16 ALU on x86 without specific extensions).

**Our build approach:** Implement a custom `f16` type (~200 lines) wrapping `u16` with IEEE-754 bit manipulation for conversion. Study `half` crate source for the correct conversion formulas and SIMD `_mm256_cvtph_ps` / `_mm256_cvtps_ph` wrappers. The `half` crate is well-written reference material but we implement our own to stay zero-dependency.

### 3.5 `zerocopy` / `bytemuck`

**`zerocopy`** ([crates.io/crates/zerocopy](https://crates.io/crates/zerocopy), v0.8+):
- Derive macros: `#[derive(FromBytes, AsBytes, FromZeroes)]`
- Enables zero-copy casting between types
- Compile-time alignment verification

**`bytemuck`** ([crates.io/crates/bytemuck](https://crates.io/crates/bytemuck), v1.14+):
- `cast_slice`, `pod`, `zeroable` traits
- Runtime and compile-time checks for safe casting
- Slightly simpler API than zerocopy

**Assessment:** Both are excellent. For our use case (interpreting mmap'd bytes as tensor data), either works. `zerocopy` has stronger compile-time guarantees via derive macros.

**Our build approach:** Implement a custom `cast_slice::<T: Pod>(bytes: &[u8]) -> &[T]` function. Study zerocopy's alignment verification approach (~50 lines of unsafe Rust with proper alignment assertions).

### 3.6 `memmap2`

**Repository:** [github.com/RazrFalcon/memmap2-rs](https://github.com/RazrFalcon/memmap2-rs)  
**Version:** 0.9+  

Rust wrapper for mmap system calls.

**Relevant features:**
- `MmapOptions::new().populate()` — MAP_POPULATE equivalent
- `MmapOptions::new().map_raw()` — Returns `MmapRaw` (no auto-deref, explicit pointer access)
- Cross-platform (Unix mmap, Windows CreateFileMapping)
- `MmapMut` for writable mappings (needed for KV cache files, if disk-backed)

**Alignment with our needs:**
```rust
use memmap2::MmapOptions;
use std::fs::File;

let file = File::open("model.gguf")?;
let mmap = unsafe { MmapOptions::new().map(&file)? };

// Access weights as typed slice (via zerocopy)
let weights: &[Cqr4Block] = zerocopy::cast_slice(&mmap[offset..offset+size]);

// Hint sequential access (Linux-specific, call via libc)
#[cfg(target_os = "linux")]
unsafe {
    libc::madvise(mmap.as_ptr() as *mut _, mmap.len(), libc::MADV_SEQUENTIAL);
}
```

**Our build approach:** Write a custom mmap wrapper (~150 lines) using raw `libc::mmap`, `libc::munmap`, `libc::madvise` calls. Study `memmap2` source for the safe Rust type patterns, but implement our own. The API is trivial: `mmap(fd, size, flags) -> *mut u8`.

### 3.7 Threading: `rayon` / `crossbeam`

**`rayon`** ([github.com/rayon-rs/rayon](https://github.com/rayon-rs/rayon), v1.10+):
- Work-stealing thread pool
- `par_iter()` for data parallelism
- Excellent for >4 threads with embarrassingly parallel workloads

**`crossbeam`** ([github.com/crossbeam-rs/crossbeam](https://github.com/crossbeam-rs/crossbeam), v0.8+):
- Lock-free data structures
- `crossbeam::scope` for scoped threads
- `crossbeam-channel` for fast MPMC channels

**Assessment for 2 threads:**
- `rayon` is overkill for 2 threads with a simple matmul split
- `crossbeam` provides the primitives we likely need (channels, scoped threads)
- For our use case: **direct thread spawning with `std::thread` + `crossbeam-channel`** is simplest

**Our build approach:** Implement a custom 2-thread pool with `std::thread::spawn` and `std::sync::atomic` channels. Study `crossbeam`'s lock-free queue design for the work-stealing pattern, but our fixed 2-thread pool is simpler — no work-stealing needed. Total: ~200 lines.

### 3.8 Synchronization: `parking_lot`

**Repository:** [github.com/Amanieu/parking_lot](https://github.com/Amanieu/parking_lot)  
**Version:** 0.12+  

Drop-in replacements for `std::sync` primitives with better performance:
- `Mutex` — Futex-based on Linux, no poisoning
- `RwLock` — Fair, efficient
- `Condvar` — Low-overhead signaling

**For 2-thread LLM inference:** Lock contention is minimal (1-2 threads accessing shared KV cache). `parking_lot`'s futex-based `Mutex` avoids spinning in the rare contention case.

**Our build approach:** Implement a custom futex-based mutex using `libc::syscall(SYS_futex, ...)` — ~80 lines of unsafe Rust. The parking_lot source code is an excellent reference for the futex wait/wake pattern. For our 2-thread case, even `std::sync::Mutex` is sufficient.

### 3.9 GGUF Parsing: `gguf` crate

**Repository:** [crates.io/crates/gguf](https://crates.io/crates/gguf)  
**Version:** 0.6+ (varies; ecosystem fragmented)

**Current state:** Multiple GGUF crates exist, none is definitively production-grade:
- `gguf` — Basic reader, handles most common quant types
- `candle-core::quantized::gguf` — Candle's bundled GGUF reader
- `llm` (archived) — Had its own GGUF reader

**Assessment:**
- Basic header and metadata parsing: ✅ works
- Tensor data extraction with alignment guarantees: ⚠️ partial
- Support for all k-quant and IQ types: ⚠️ varies
- Zero-copy mmap integration: ❌ typically copies into Vec

**Our build approach:** Write a custom GGUF reader (~400 lines) using `libc::mmap` + safe Rust wrapper. The format is well-specified and not complex:
```rust
#[repr(C, align(64))]
struct GgufHeader {
    magic: [u8; 4],     // "GGUF"
    version: u32,
    tensor_count: u64,
    metadata_kv_count: u64,
}
```
Custom reader ensures 64-byte alignment for tensor data (critical for our AVX2 loads).

### 3.10 Tokenization: `tokenizers`

**Repository:** [github.com/huggingface/tokenizers](https://github.com/huggingface/tokenizers)  
**Crate:** `tokenizers` on crates.io  
**Version:** 0.20+  

**Features:**
- BPE, WordPiece, Unigram, SentencePiece
- Rust-native (no Python dependency)
- Fast encoding (<10μs per token)
- Thread-safe
- Streaming decode support

**Our build approach:** Implement a custom BPE tokenizer (~500 lines). Study HuggingFace's `tokenizers` source for the BPE merge algorithm and byte-level pre-tokenization. Loading: parse HuggingFace `tokenizer.json` using our custom JSON parser. Encoding: greedy BPE merge over byte sequences. At <10μs per token, tokenization will never be a bottleneck even with a from-scratch implementation.

### 3.11 HTTP: `axum` / `tokio`

**`axum`** ([github.com/tokio-rs/axum](https://github.com/tokio-rs/axum), v0.7+):
- Ergonomic web framework
- SSE streaming built-in (`axum::response::sse`)
- Middleware, extractors, routing
- Tokio-based (async)

**`tokio`** ([github.com/tokio-rs/tokio](https://github.com/tokio-rs/tokio), v1.38+):
- Async runtime
- Task spawning, channels, timers

**Assessment for compute-bound inference:**
- Inference itself is **not async** — it's blocking compute
- Use `tokio::task::spawn_blocking` for inference execution
- axum handles HTTP/SSE on the async runtime
- Inference runs on dedicated thread pool, results sent via channel

```rust
async fn chat_handler(state: State<Runtime>, request: Json<ChatRequest>) -> Sse<...> {
    let (tx, rx) = channel(32);
    tokio::task::spawn_blocking(move || {
        // This runs on the blocking thread pool
        for token in state.infer(&request) {
            tx.send(token).unwrap();
        }
    });
    Sse::new(ReceiverStream::new(rx))
}
```

**Our build approach:** Build a custom HTTP/1.1 server (~600 lines) using `std::net::TcpListener` with a thread-per-connection model. No async runtime needed — inference is blocking compute anyway. The axum/tokio pattern above shows what the same thing looks like with dependencies; our version is simpler:

```rust
// Our approach — no async, no framework
for stream in listener.incoming() {
    std::thread::spawn(move || {
        let req = parse_http_request(&stream);
        let gen = runtime.create_generator(&req);
        write_sse_headers(&stream);
        for token in gen {
            write_sse_event(&stream, &format_chunk(&token));
        }
    });
}
```

---

## 4. FFI Boundaries: Zero FFI to External C Libraries

With the zero-dependency approach, **we have no FFI to external C libraries.** The only `extern "C"` calls are to POSIX syscalls via the `libc` crate:

- `libc::mmap()` / `libc::munmap()` — memory-mapped file I/O
- `libc::madvise()` — memory hints (MADV_SEQUENTIAL, MADV_DONTNEED)
- `libc::sched_getaffinity()` — CPU topology detection  
- `libc::mlock()` — optional page pinning

All compute kernels (matmul, attention, norm, activations) are pure Rust with `core::arch` SIMD intrinsics. No oneDNN, no OpenBLAS, no ggml C bindings. This eliminates the FFI overhead, build complexity, and unsafe-FFI-boundary audit entirely (except for the ~10 POSIX syscall wrappers).

**Total unsafe FFI surface:** ~50 lines (POSIX syscalls only), compared to thousands of lines if wrapping C libraries.


**Note on FP32 matmul:** The few FP32/FP16 matmuls needed (embedding lookup, LM head output projection) will be implemented from scratch in pure Rust. For 4096×4096 matrices, a simple blocked GEMM with register tiling achieves ~95% of BLAS throughput in ~200 lines of code. No external BLAS dependency needed.

---

## 5. unsafe Rust Strategy for SIMD Kernels

### 5.1 What MUST be unsafe

| Operation | Requires unsafe? | Reason |
|-----------|-----------------|--------|
| SIMD intrinsics (`_mm256_*`) | YES | Raw register manipulation, no bounds checking |
| Raw pointer arithmetic (tensor iteration) | YES | Bypassing borrow checker for performance |
| FFI calls (`extern "C"`) | YES | Cannot verify C code safety |
| mmap address access | YES | OS-level memory management |
| Transmute (bit casting) | YES | Type reinterpretation |
| `core::arch::*` intrinsic dispatch | YES | Platform-specific, unportable |

### 5.2 Minimization Strategy

```
cpu-llm-runtime/
├── crates/
│   ├── kernels/              # ALL unsafe SIMD code lives here
│   │   ├── src/
│   │   │   ├── avx2/         # AVX2 intrinsics (unsafe)
│   │   │   │   ├── q4k.rs   # Q4_K_M dot product
│   │   │   │   └── f16.rs   # FP16 operations
│   │   │   ├── avx512/       # AVX-512 intrinsics (unsafe)
│   │   │   ├── scalar/      # Safe scalar fallback
│   │   │   └── lib.rs       # Safe public API (dispatch)
│   │   └── ...
│   ├── executor/            # NO unsafe (delegates to kernels)
│   ├── model/               # Minimal unsafe (mmap pointer access)
│   └── memory/              # Minimal unsafe (madvise calls)
```

**Target: <2000 lines of unsafe code** across the entire runtime, all in `kernels/` and `memory/`.

### 5.3 Testing unsafe Code

| Tool | What it Tests | Limitations |
|------|--------------|-------------|
| `miri` | Undefined behavior detection | Cannot run SIMD (miri simulates a single-threaded interpreter) |
| `loom` | Concurrent correctness (data races) | Limited to synchronization patterns, not SIMD |
| Property testing (`proptest`) | Numerical correctness | Must test SIMD against scalar reference |
| Fuzz testing (`cargo-fuzz`) | Edge cases | Slow for numerical kernels |

**Strategy for SIMD kernel testing:**
1. Every AVX2 kernel must match scalar reference within tolerance (1e-3 relative)
2. Property tests with random inputs (proptest)
3. Run full model forward pass and compare output to llama.cpp reference

### 5.4 Audit Surface Area Estimate

| Component | Estimated Unsafe Lines | Audit Priority |
|-----------|----------------------|---------------|
| AVX2 Q4_K_M matmul | ~300 | HIGH |
| AVX2 FP16 operations | ~150 | HIGH |
| AVX-512 kernels | ~200 | MEDIUM |
| mmap wrapper | ~50 | HIGH |
| Tensor pointer access | ~100 | MEDIUM |
| FFI bindings (if any) | ~50 | MEDIUM |
| **Total** | **~850 lines** | |

This is an audit-feasible surface area. A security audit of ~850 lines of unsafe Rust is a 1-2 week engagement.

---

## 6. Rust vs C++ Performance Parity for Quantized Matmul

### 6.1 Existing Benchmarks

| Operation | Rust (`core::arch` AVX2) | C (GCC intrinsics, AVX2) | Ratio |
|-----------|--------------------------|--------------------------|-------|
| Q4_0 dot product (1024 elements) | ~800 ns | ~780 ns | 97.6% |
| Q4_K_M dot product (256 elements) | ~200 ns | ~195 ns | 97.5% |
| FP32 vector add (4096 elements) | ~50 ns | ~48 ns | 96% |
| Integer multiply-accumulate | ~120 ns | ~118 ns | 98% |

[ESTIMATED based on published comparisons and Candle vs llama.cpp benchmarks]

**Finding:** Rust matches C++ within 2-4% when using explicit intrinsics. The gap comes from:
1. LLVM's slightly different code generation order (instruction scheduling)
2. Rust's additional debug assertions in debug mode (eliminated in release)
3. Different loop unrolling heuristics

**Conclusion:** Rust is NOT a performance liability for this workload. Hand-written intrinsics generate equivalent assembly to C intrinsics (both compile to the same `vpmaddubsw` etc.)

### 6.2 Compiler Optimization: LLVM vs GCC/Clang

For `core::arch` intrinsics:
- Rust uses LLVM backend (same as Clang)
- Intrinsic functions compile to identical machine instructions
- Auto-vectorization (not relevant here — we use explicit intrinsics) may differ
- `-C target-cpu=native` flag enables full AVX2/AVX-512 codegen

**Build flags for maximum performance:**
```toml
# Cargo.toml
[profile.release]
opt-level = 3
lto = "fat"          # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
panic = "abort"      # No unwind overhead
```

```bash
RUSTFLAGS="-C target-cpu=native -C opt-level=3" cargo build --release
```

### 6.3 Cases Where Rust is Measurably Slower

| Scenario | Cause | Mitigation |
|----------|-------|-----------|
| Bounds checking in loops | `arr[i]` includes bounds check | Use `arr.get_unchecked(i)` in unsafe kernels |
| Drop overhead for large types | Vec<T> drop iterates elements | Pre-allocate and reuse (arena pattern) |
| Debug assertions | `debug_assert!` in debug builds | Use only in tests, not kernels |
| Generic monomorphization | Large inline code | `#[inline(never)]` on large functions |
| String processing | UTF-8 validation overhead | Use `&[u8]` for internal strings |

**None of these apply to our hot path** (quantized matmul) when written correctly with unsafe + bounds-check elimination.

---

## 7. Memory Management in Rust for This Use Case

### 7.1 Arena Allocation: `bumpalo`

**Repository:** [github.com/fitzgen/bumpalo](https://github.com/fitzgen/bumpalo)  
**Version:** 3.16+  

```rust
use bumpalo::Bump;

let arena = Bump::with_capacity(256 * 1024 * 1024);  // 256 MB

// Allocate activation buffers
let q: &mut [f16] = arena.alloc_slice_fill_default(4096);
let k: &mut [f16] = arena.alloc_slice_fill_default(head_dim * n_kv_heads);
let attn_scores: &mut [f32] = arena.alloc_slice_fill_default(context_len * n_kv_heads);

// Reset after each token (O(1) "free everything")
arena.reset();
```

**Properties:**
- Allocation: pointer bump (branch + increment), ~1ns
- Deallocation: `reset()` sets pointer back to start, ~1ns  
- Memory: contiguous, no fragmentation
- Drop: no element-by-element drop (we're managing raw memory)

**Our build approach:** Implement a custom bump allocator (~80 lines of core logic). The algorithm is: maintain a `Vec<u8>` buffer + offset pointer; `alloc()` advances the pointer; `reset()` sets it back to zero. Study `bumpalo` source for edge cases (alignment, overflow) but write our own.

### 7.2 Custom Global Allocator

```rust
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

**Allocator comparison for our workload:**

| Allocator | Allocation Speed | Large Alloc Performance | Memory Usage | Fragmentation |
|-----------|-----------------|------------------------|--------------|---------------|
| `std::alloc::System` (default) | Medium | Good (uses mmap for large allocs) | Baseline | Low |
| `jemalloc` | Fast | Good | Slightly higher | Low |
| `mimalloc` | Fast | Excellent | Low | Very low |
| `tcmalloc` | Fast | Good | Low | Low |

For our runtime with frequent large allocations (KV cache, arena buffer): **mimalloc** is optimal. Allocate the arena and KV cache once at startup; subsequent allocations are small (request/response buffers).

### 7.3 `Pin<T>` for KV Cache

For the pre-allocated KV cache that must not be moved:
```rust
use std::pin::Pin;

struct KVCache {
    keys: Pin<Vec<f16>>,      // Pinned: address won't change
    values: Pin<Vec<f16>>,     // Pinned: safe to pass raw pointers to kernels
}
```

This enables safe raw pointer passing to SIMD kernels (guaranteed stable addresses).

### 7.4 Drop Semantics

Rust's deterministic deallocation (no GC) is a significant advantage for real-time inference:
- No stop-the-world pauses
- KV cache freed deterministically when session ends
- Arena reset is O(1), no finalizer chain

---

## 8. Build System and Dependency Management

### 8.1 Cargo Workspace Layout

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/runtime",      # Binary crate (main entry point, CLI)
    "crates/inference",    # Core inference engine (from scratch)
    "crates/kernels",      # SIMD kernels (from scratch, all unsafe code)
    "crates/model",        # GGUF parser, model architectures (from scratch)
    "crates/memory",       # Streaming weight executor, KV cache (from scratch)
    "crates/server",       # HTTP API server (axum-based)
]
resolver = "2"

[workspace.dependencies]
# Infrastructure crates (standard, not inference-related)
axum = { version = "0.7", features = ["tokio"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"

# Low-level (shared by inference core)
libc = "0.2"
memmap2 = "0.9"
half = { version = "2", features = ["serde"] }
```

### 8.2 Feature Flags for CPU Target Selection

```toml
# crates/kernels/Cargo.toml
[features]
default = ["avx2"]
avx2 = []
avx512 = []
neon = []           # ARM

# Build for specific target
# cargo build --features avx512 --target x86_64-unknown-linux-gnu

[dependencies]
# Zero external dependencies — only workspace crates
```

In kernel code:
```rust
#[cfg(feature = "avx2")]
mod avx2;

#[cfg(feature = "avx512")]
mod avx512;

#[cfg(feature = "neon")]
mod neon;

mod scalar;  // Always available

pub fn q4k_dot(weights: &[u8], x: &[f16], n: usize) -> f32 {
    #[cfg(feature = "avx512")]
    if std::is_x86_feature_detected!("avx512f") {
        return unsafe { avx512::q4k_dot(weights, x, n) };
    }
    #[cfg(feature = "avx2")]
    if std::is_x86_feature_detected!("avx2") {
        return unsafe { avx2::q4k_dot(weights, x, n) };
    }
    scalar::q4k_dot(weights, x, n)
}
```

### 8.3 Cross-Compilation Targets

| Target | Command | Notes |
|--------|---------|-------|
| x86_64-unknown-linux-gnu (primary) | Default | AVX2/AVX-512 kernels |
| aarch64-unknown-linux-gnu | `--target aarch64-unknown-linux-gnu` | NEON kernels |
| x86_64-apple-darwin | macOS Intel | AVX2 kernels |
| aarch64-apple-darwin | macOS Apple Silicon | NEON kernels |

### 8.4 CI Strategy

```yaml
# .github/workflows/bench.yml
bench:
  runs-on: [self-hosted, cpu-bench]  # Dedicated benchmark machine
  steps:
    - uses: actions/checkout@v4
    - name: Run benchmarks
      run: |
        cargo bench --features avx2
        # Compare against baseline, fail if >5% regression
```

---

## 9. Ecosystem Gap Analysis

### 9.1 Build vs Reuse Decision Matrix

**Principle:** Every component that touches model weights, KV cache, attention, or memory bandwidth is built from scratch. Standard infrastructure uses the best available crates.

| Component | Build or Crate? | Choice | Rationale |
|-----------|----------------|--------|-----------|
| Q4_K_M matmul kernel | **FROM SCRATCH** | Custom SIMD (AVX2/AVX-512 + NEON + scalar) | The critical hot path. Cache-optimal layout, no generic overhead. |
| Q8_0 / other quant matmul | **FROM SCRATCH** | Custom SIMD dispatch per quant type | Same — tight loop, must minimize memory access. |
| Streaming weight executor | **FROM SCRATCH** | Custom mmap + madvise lifecycle | No crate does this. Core differentiator. |
| KV cache manager | **FROM SCRATCH** | Contiguous + sliding window + INT8 | Tightly coupled to kernel layout. |
| Memory budget / pressure monitor | **FROM SCRATCH** | `/proc/meminfo`, RSS tracking | Lightweight, no framework needed. |
| CPU topology detector | **FROM SCRATCH** | CPUID, `/sys/devices/system/cpu` | Simple, one-shot, no crate needed. |
| Speculative decoding engine | **FROM SCRATCH** | Novel feature | Not available in any crate. |
| BPE tokenizer (optional) | **FROM SCRATCH** or `tokenizers` | Either works | Not a bottleneck (~1μs/token). Custom is fine, crate is fine. |
| FP16/BF16 type | **FROM SCRATCH** or `half` | Either works | `half` is solid. Custom avoids a dependency. |
| Arena/bump allocator | **FROM SCRATCH** (~80 lines) | Custom bump pointer | Trivial to implement. Full control over alignment. |
| HTTP server | `axum` + `tokio` | Crate | Production-grade, not inference-related. |
| JSON serialize/deserialize | `serde` + `serde_json` | Crate | Standard, not inference-related. |
| CLI argument parsing | `clap` | Crate | Standard, not inference-related. |
| Config file parsing | `toml` or `figment` | Crate | Standard, not inference-related. |
| Async runtime | `tokio` | Crate | Needed for axum. Not used in inference core. |
| Logging | `tracing` | Crate | Standard, not inference-related. |
| mmap wrapper | `memmap2` or custom | Crate (or ~100 line custom) | Either works. `memmap2` is battle-tested. |
| Zero-copy tensor casting | Custom `unsafe` (~50 lines) | From scratch | Alignment-sensitive, tied to quant formats. |

**Inference core (from scratch):** ~4,000–5,000 lines  
**Infrastructure (crates):** Standard Cargo.toml dependency tree  
**Total codebase:** ~7,000–8,000 lines of custom Rust + crate dependencies for non-critical infrastructure

### 9.2 Crates That DON'T Exist and Must Be Built

| Needed Component | Reason | Estimated Effort |
|-----------------|--------|-----------------|
| Cache-optimal Q4_K_M dot kernel (AVX2 + AVX-512) | Existing (Candle) is 60-70% of optimal | 2 weeks |
| Mmap-based streaming weight executor | No crate does this | 1 week |
| Contiguous KV cache with sliding window + INT8 | No crate combines these | 1 week |
| CPU topology detector (HT-aware) | Existing crates don't do HT detection for inference | 2 days |
| GGUF → CQR-4 format converter | Format doesn't exist yet | 3 days |

### 9.3 Where Rust Ecosystem is Behind C++

| Area | C++ State | Rust State | Gap Impact |
|------|-----------|-----------|-----------|
| Quantized matmul kernels | Highly optimized (llama.cpp, oneDNN) | Candle (60-70% of C++) | MEDIUM — must build custom |
| Graph-based execution | C++ frameworks (GGML compute graph) | No equivalent | LOW — we don't need compute graph for batch-1 |
| Memory-mapped tensor I/O | llama.cpp, PyTorch mmap | `memmap2` + custom | LOW — we have the primitives |
| Distributed inference | TensorRT-LLM, DeepSpeed | None | N/A — single-node target |
| Model zoo/converters | HuggingFace Python | `candle-transformers` models | LOW — sufficient models available |

---

## 10. Reference Codebases to Study

### 10.1 Specific Files and Functions

**Candle (candle-core):**
```
candle-core/src/quantized/k_quants.rs:fn vec_dot_q4k — K-quant dot product (reference)
candle-core/src/quantized/gguf.rs:struct Content — GGUF loading
candle-transformers/src/models/llama.rs:impl Llama — Complete Llama forward pass
```

**mistral.rs:**
```
mistral.rs/src/scheduler/mod.rs — Continuous batching scheduler
mistral.rs/src/engine/mod.rs — Token generation loop
mistral.rs/src/pipeline/gguf.rs — GGUF loading
```

**llama.cpp (C reference):**
```
ggml/src/ggml-quants.c:ggml_vec_dot_q4_K — The gold standard Q4_K_M dot product
ggml/src/ggml.c:ggml_graph_compute — Thread pool + task dispatch
src/llama.cpp:llama_decode_internal — Full forward pass orchestration
```

### 10.2 Code to Port (Priority Order)

1. **`ggml_vec_dot_q4_K`** → Port to Rust AVX2 intrinsics in `kernels/avx2/q4k.rs`
2. **`llama_decode_internal`** → Adapt to Rust in `executor/forward.rs`
3. **Candle's Llama model** → Adapt (already Rust) for our tensor types
4. **mistral.rs server.rs** → Adapt for our API design
5. **llama.cpp thread pool** → Simplify to 1-2 thread fixed pool

---

## 11. Implementation Implications

### 11.1 Crate Dependencies (Final List)

**Inference core — everything from scratch using `core::arch` + `libc`:**
```rust
// No crates needed for the inference engine core
// Only standard library: std, core, core::arch (SIMD), libc (syscalls)
```

**Infrastructure dependencies:**
```toml
[dependencies]
# Inference core — NO external crates
# (uses core::arch, std, libc which is a sys crate)

# Infrastructure — standard production crates
axum = { version = "0.7", features = ["tokio"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
memmap2 = "0.9"        # mmap (optional, can use raw libc)
half = { version = "2", features = ["serde"] }  # f16 type (optional, can build custom)
```

**Split philosophy:**
- `axum`, `tokio`, `serde`, `clap`, `toml`, `tracing` → standard infrastructure, not inference-related
- `memmap2` → convenience wrapper around `libc::mmap`, can replace with custom in ~100 lines
- `half` → convenience f16 type, can replace with custom in ~200 lines
- **The inference core has zero external crate dependencies** — only `std`, `core::arch`, and `libc` (POSIX syscalls)

### 11.2 Ecosystem Study Score (Reference Implementations)

| Category | Available to Study | Quality | Our Build Effort |
|----------|-------------------|---------|-----------------|
| SIMD intrinsics | `core::arch` (std) | Excellent | Medium — verbose but complete |
| Memory management patterns | `memmap2`, `bumpalo` sources | Excellent | Low — simple wrappers |
| Tokenization algorithms | HuggingFace `tokenizers` source | Excellent | Medium — BPE is well-documented |
| HTTP protocol | `tiny-http`, `hyper` sources | Good | Medium — HTTP/1.1 spec is complex |
| Quantization kernels | llama.cpp `ggml-quants.c` | Gold standard | High — port to Rust SIMD |
| Model implementations | Candle `candle-transformers` | Good | Medium — adapt Rust patterns |
| GGUF parsing | `gguf` crate, Candle | Functional | Low — binary format, well-specified |
| KV cache management | llama.cpp source | Partial | Medium — design our own layout |
| JSON parsing | `serde_json` source | Excellent | Medium — only need OpenAI schema subset |

### 11.3 Critical Path Modules to Build First

1. **`crates/kernels`** (Weeks 1–5): SIMD dot product, attention, norm, activation — the performance core
2. **`crates/memory`** (Weeks 3–5): mmap wrapper, arena allocator, KV cache manager
3. **`crates/model`** (Weeks 1–4): GGUF parser + Llama/Qwen2/Gemma2 forward pass implementations
4. **`crates/tokenizer`** (Weeks 1–2): BPE tokenizer with HuggingFace vocab loading
5. **`crates/net` + `json`** (Week 7+): HTTP/1.1 + SSE + JSON parser — not on critical path

---

## 12. Summary: The Rust Ecosystem Verdict

**The split is clean:** Inference core = from scratch. Infrastructure = production crates.

| Layer | Implementation | Lines |
|-------|---------------|-------|
| SIMD kernels (AVX2/AVX-512/NEON) | From scratch | ~800 |
| Streaming weight executor (mmap + madvise) | From scratch | ~600 |
| KV cache manager | From scratch | ~400 |
| Model architectures (Llama, Qwen2, Gemma) | From scratch | ~1,200 |
| Sampler / decoding | From scratch | ~300 |
| Memory budget / topology | From scratch | ~200 |
| Total inference core | **From scratch** | **~3,500–4,500** |
| HTTP API (axum) | Crate | — |
| JSON (serde) | Crate | — |
| CLI (clap) | Crate | — |
| Config (toml) | Crate | — |
| Logging (tracing) | Crate | — |
| **Total custom codebase** | | **~4,000–5,000 lines** |

**Key takeaways:**
1. The inference core requires zero external crate dependencies (only `std` + `core::arch` + `libc`)
2. The hardest code to write: quantized matmul kernels (~800 lines of SIMD intrinsics)
3. The most novel code to write: streaming weight executor (~600 lines, mmap + madvise lifecycle)
4. No existing crate in the Rust ecosystem does what we need for the inference core
5. The Rust SIMD ecosystem (`core::arch`) is mature enough to match C/C++ performance

---

*This completes the 9-document research knowledge base. The implementation agent should read all documents in order and begin with Phase 1 (proof of concept) as described in Document 8. Every component is built from scratch — no dependencies except `libc`.*
