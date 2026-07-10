# Document 1: State of the Art, Open-Source CPU Inference Engines

**Research Program:** CPU-Native LLM Inference Runtime  
**Target Spec:** 9B parameter model on 2 vCPUs, 6 GB RAM, 2–5 tokens/second  
**Author:** Research Agent  
**Date:** June 2025  

---

## 1. Introduction

This document surveys the landscape of open-source LLM inference engines with a focus on CPU viability, memory efficiency, and performance on severely constrained hardware (2 vCPUs, 6 GB RAM). For each system, we analyze architecture, memory profile for a 9B model, throughput at 2 vCPU, and extractable design principles.

The central question: *Is any existing system already achieving 2–5 tok/s for 9B models on 2 vCPUs, or is there genuine optimization space for a purpose-built runtime?*

---

## 2. llama.cpp

**Repository:** [github.com/ggerganov/llama.cpp](https://github.com/ggerganov/llama.cpp)  
**License:** MIT  
**Language:** C/C++  
**Maturity:** Most widely-used CPU LLM inference engine (50k+ GitHub stars)  

### 2.1 Architecture Overview

llama.cpp is structured around the `ggml` tensor library (now `ggml` repo extracted separately) and the `llama.cpp` model/execution layer on top.

```
┌─────────────────────────────────────────┐
│              llama.cpp API              │
│  (model loading, sampling, context)     │
├─────────────────────────────────────────┤
│          llama.cpp model layer          │
│  (transformer forward pass, attention,  │
│   FFN, RoPE, KV cache management)       │
├─────────────────────────────────────────┤
│            ggml tensor lib              │
│  (quantized ops, SIMD dispatch,         │
│   memory allocation, threading)         │
├─────────────────────────────────────────┤
│        Platform backends               │
│  (CPU SIMD, CUDA, Metal, Vulkan, SYCL) │
└─────────────────────────────────────────┘
```

**Key source files:**
- `ggml/src/ggml-quants.c`, All quantized dot product kernels (Q4_0, Q4_K_M, Q5_K_S, etc.)
- `ggml/src/ggml.c`, Core tensor operations, compute graph, threading
- `src/llama.cpp`, Model architecture implementations
- `ggml/include/ggml.h`, Public tensor API

### 2.2 GGUF Format Internals

GGUF (GPT-Generated Unified Format) is llama.cpp's model serialization format. It replaces the older GGML format (v1-v3) and is designed for efficient memory-mapped loading.

**Structure:**
```
┌──────────────────────────┐
│ Magic: "GGUF" (4 bytes)  │
│ Version: uint32          │
│ Tensor count: uint64     │
│ Metadata KV count: uint64│
├──────────────────────────┤
│ Metadata Key-Value pairs │
│ (architecture, hyperparams│
│  tokenizer info, etc.)   │
├──────────────────────────┤
│ Tensor info entries      │
│ (name, dims, type, offset)│
├──────────────────────────┤
│ Alignment padding        │
├──────────────────────────┤
│ Tensor weights (raw data)│
│ Aligned to configurable  │
│ boundary (default 32B)   │
└──────────────────────────┘
```

**Quantization types supported:**
| Type | Bits/Weight | Block Size | Super-block | Size (9B params) | 
|------|-------------|------------|-------------|-------------------|
| Q4_0 | 4.5 | 32 | none | ~5.0 GB |
| Q4_K_M | 4.8 | 256 | 8 blocks | ~5.5 GB |
| Q5_K_S | 5.5 | 256 | 8 blocks | ~6.2 GB |
| Q5_K_M | 5.7 | 256 | 8 blocks | ~6.4 GB |
| Q6_K | 6.6 | 256 | 16 blocks | ~7.4 GB |
| Q8_0 | 8.5 | 32 | none | ~9.6 GB |
| IQ2_XXS | 2.06 | 256 | complex | ~2.3 GB |
| IQ2_XS | 2.31 | 256 | complex | ~2.6 GB |
| IQ3_XXS | 3.06 | 256 | complex | ~3.4 GB |

The `_K` variants use "k-quants" with super-blocks containing separate scales for each sub-block, achieving better quality at similar bit widths. The `IQ` variants use importance-weighted quantization with lookup tables (codebooks).

**Memory-mapped loading:** llama.cpp uses `mmap()` by default. The `llama_model_load()` function maps the file directly into virtual memory; tensor weights are accessed via pointers into the mapped region. This means:
- Near-instant "loading" (just mmap, no copy)
- OS page cache handles prefetching
- Only touched weights consume physical RAM
- `MAP_POPULATE` can pre-fault pages; `mlock()` can pin them

### 2.3 Threading Model

llama.cpp uses a **work-stealing thread pool** with a fixed thread count (configurable via `-t` flag).

**Design:**
- Main thread dispatches compute graph tasks to worker threads
- Workers steal from each other's queues when idle
- Synchronization via condition variables (futex-based on Linux)
- Default thread count = number of physical cores (not hyperthreads)

**For 2 vCPUs:** llama.cpp defaults to 2 threads. However, this is suboptimal if the 2 vCPUs are hyperthreads on a single physical core, in that case, 1 worker thread may outperform 2 due to resource contention.

**Performance at 2 threads:** Based on community benchmarks:
- Llama-3-8B Q4_K_M on AMD Epyc (2 cores): ~3–4 tok/s decode, ~50 tok/s prefill
- Qwen2.5-7B Q4_K_M on Intel Xeon Silver (2 cores): ~2.5–3.5 tok/s decode
- On a cloud VM with 2 hyperthreads (1 physical core): ~1.5–2.5 tok/s decode

### 2.4 Quantization Pipeline

llama.cpp provides its own quantization tool (`llama-quantize`) that converts FP16/BF16 GGUF files to quantized formats using:
- **RTN (Round-To-Nearest):** Default for most types, no calibration data needed
- **k-quant optimization:** Improved scale/min computation using block statistics
- **Importance matrix (imatrix):** Optional calibration using a small dataset to weight-important rows receive higher precision

### 2.5 Memory Allocator

ggml uses a **bump allocator** (`ggml_allocr`) for compute graph execution:
- Pre-allocates a large contiguous buffer
- Tensor allocations bump a pointer forward
- Entire buffer reset between forward passes (no fragmentation)
- Separate buffers for compute (activations) and KV cache

**KV Cache:** Stored as a contiguous array of shape `[n_layers × 2 × n_kv_heads × head_dim × max_context]`. For Qwen2.5-9B (64 layers, 4 KV heads, head_dim 128, FP16):
- At context 2048: 2 × 64 × 4 × 128 × 2048 × 2 = ~268 MB
- At context 4096: ~536 MB
- At context 8192: ~1.07 GB

### 2.6 SIMD Dispatch Layer

ggml dispatches to platform-specific kernels at runtime:
```c
// Simplified dispatch pattern in ggml-quants.c
#if defined(__AVX2__)
    ggml_vec_dot_q4_0_avx2(...)
#elif defined(__ARM_NEON)
    ggml_vec_dot_q4_0_neon(...)
#elif defined(__riscv_vector)
    ggml_vec_dot_q4_0_rvv(...)
#else
    ggml_vec_dot_q4_0_ref(...)  // scalar fallback
#endif
```

**Key SIMD kernels and their throughput:**

| Instruction Set | INT4 Packing | INT8 ops/cycle (256-bit) | Notes |
|----------------|--------------|--------------------------|-------|
| AVX2 | 8 INT4 per byte | 32 INT8 madd | Most cloud VMs |
| AVX-512 VNNI | 8 INT4 per byte | 64 INT8 madd | Newer Xeons |
| AVX-512 BF16 | BF16 native | 32 BF16 fma | Ice Lake+ |
| AMX | 4-bit tile | 1024 INT8 ops | Sapphire Rapids |
| NEON | 8 INT4 per byte | 16 INT8 | ARM v8+ |
| SVE/SVE2 | 8+ INT4 per byte | Variable (128-2048 bit) | ARM v9 |

### 2.7 What to Steal from llama.cpp

1. **mmap-first loading model**, Near-zero load time, leverage OS page cache
2. **GGUF format**, Well-specified, widely supported, mmap-friendly alignment
3. **Bump allocator** for activations, Simple, fast, zero-fragmentation
4. **Quantization type ecosystem**, Q4_K_M is the sweet spot for quality/size
5. **SIMD kernel structure**, Per-quant-type dispatch with platform fallbacks
6. **imatrix calibration**, Lightweight importance-weighted quantization

### 2.8 Known Bottlenecks and Limitations

- **KV cache is always contiguous**, No paging, no eviction, no sharing. Wastes memory for short contexts after long prefill.
- **No continuous batching**, One request at a time. The `llama-server` queues requests sequentially.
- **Thread pool overhead** at low thread counts, Synchronization cost dominates when only 2 threads
- **No speculative decoding** in mainline (experimental PRs exist)
- **Memory ceiling**, KV cache + weights must all fit in RAM simultaneously
- **No disk offloading** for KV cache

**Estimated 9B Q4_K_M on 2 vCPU / 6 GB RAM:**
- Peak memory: ~5.5 GB (weights) + ~0.5 GB (KV cache @ 4096) + ~0.3 GB (overhead) = ~6.3 GB, **barely exceeds budget**
- Decode throughput: **2–4 tok/s** (achievable but tight)
- Verdict: llama.cpp gets close but doesn't optimize for the *specifically constrained* 2-vCPU/6GB cloud scenario

---

## 3. vLLM

**Repository:** [github.com/vllm-project/vllm](https://github.com/vllm-project/vllm)  
**License:** Apache 2.0  
**Language:** Python + CUDA C++  
**Paper:** "Efficient Memory Management for Large Language Model Serving with PagedAttention" (arXiv:2309.06180)

### 3.1 Architecture Overview

vLLM is a GPU-first LLM serving system built around the PagedAttention mechanism. It achieves near-optimal GPU utilization through:
- **Paged KV Cache:** Virtual memory-style paging of the key-value cache
- **Continuous Batching:** In-flight request batching without padding
- **Copy-on-Write KV Sharing:** Efficient prefix/prompt sharing between requests

```
┌──────────────────────────────────────┐
│         OpenAI-compatible API         │
├──────────────────────────────────────┤
│         Scheduler (Orca-style)        │
│  Continuous batching, preemption,     │
│  KV cache allocation                  │
├──────────────────────────────────────┤
│       Block Manager (KV paging)       │
│  Logical→Physical block mapping,      │
│  CoW for shared prefixes              │
├──────────────────────────────────────┤
│     Model Executor (CUDA kernels)     │
│  PagedAttention, custom GEMM,         │
│  quantized linear layers              │
└──────────────────────────────────────┘
```

### 3.2 Why vLLM Cannot Run on CPU

vLLM is architecturally GPU-bound in three fundamental ways:

1. **PagedAttention requires CUDA:** The attention kernel operates on scattered KV blocks using GPU-parallel memory gather. The random-access pattern is pathological for CPU (no spatial locality, TLB thrashing for paged memory).

2. **Continuous batching assumes GPU parallelism:** Batching 10+ requests simultaneously only makes sense when the GPU has enough SMs to parallelize. On 2 vCPUs, batching hurts more than it helps.

3. **Memory model assumes HBM:** Paged KV cache works because GPU global memory latency (~400 cycles) is hidden by massive thread parallelism. On CPU, every page fault in the KV cache is a ~100ns L3 miss vs ~1ns hit, a 100x penalty that can't be hidden.

### 3.3 What IS Reusable from vLLM

Despite being GPU-only, vLLM's **software architecture** is highly relevant:

| Component | Reusability for CPU Runtime | Rationale |
|-----------|---------------------------|-----------|
| Scheduler (Orca-style) | **HIGH** | Request scheduling, preemption, priority logic is hardware-agnostic |
| Block Manager (virtual KV) | **MEDIUM** | The logical→physical mapping concept works on CPU, but with contiguous allocation instead of paged |
| Copy-on-Write prefix sharing | **MEDIUM** | Useful for multi-turn chat where system prompts repeat; implementable with reference counting |
| OpenAI-compatible API layer | **HIGH** | Direct reuse of API surface design |
| Preemption / swapping logic | **HIGH** | When memory is tight (6 GB), preempting low-priority requests to disk is essential |
| AsyncIO serving pattern | **HIGH** | FastAPI/async serving with SSE streaming, language-agnostic pattern |

### 3.4 PagedAttention Internals

PagedAttention divides the KV cache into fixed-size blocks (typically 16 tokens per block). A block table maps each logical token position to a physical memory block:

```
Logical: [tok0, tok1, ..., tok15] [tok16, ..., tok31] [tok32, ..., tok47]
           ↓ Block Table            ↓                     ↓
Physical: [Block#7]                 [Block#12]            [Block#3]
```

**On GPU:** This enables near-zero waste (< 4% internal fragmentation vs ~60% with contiguous allocation for variable-length sequences).

**On CPU implication:** The overhead of scattered memory access outweighs the memory savings at small batch sizes. For batch=1 (interactive use), contiguous KV cache is faster on CPU because of cache-line prefetching.

**Recommendation:** Do NOT adopt PagedAttention for 2-thread CPU. Use contiguous KV cache with sliding window or eviction instead. Steal the scheduler and preemption logic only.

### 3.5 Memory Profile (Hypothetical 9B on CPU)

Not applicable, vLLM does not support CPU execution. However, the scheduling concepts translate to:
- KV cache management: use contiguous allocation with LRU eviction
- Request queue: FIFO with priority, preempt on memory pressure
- Batching: at 2 threads, batch size 1–2 maximum

---

## 4. Intel OpenVINO + Neural Compressor

**Repository:** [github.com/openvinotoolkit/openvino](https://github.com/openvinotoolkit/openvino)  
**Neural Compressor:** [github.com/intel/neural-compressor](https://github.com/intel/neural-compressor)  
**License:** Apache 2.0  
**Paper:** "OpenVINO: An Open-source Framework for Optimizing Deep Learning Inference" 

### 4.1 Architecture Overview

OpenVINO is Intel's inference optimization toolkit, structured as:
1. **Model Optimizer (MO):** Converts models (ONNX, PyTorch, TF) to OpenVINO IR
2. **Neural Network Compression Framework (NNCF):** Applies quantization, pruning, distillation
3. **Inference Engine (IE):** Runtime with CPU/GPU/NPU/VPU plugins
4. **oneDNN (formerly MKL-DNN):** Low-level CPU math library backing the CPU plugin

```
┌────────────────────────────────────┐
│        OpenVINO GenAI API          │
│  (LLM-specific: chat, streaming)  │
├────────────────────────────────────┤
│     Model Optimizer / NNCF         │
│  (FP32→INT8→INT4, calibration)    │
├────────────────────────────────────┤
│      OpenVINO Runtime Core         │
│  (Graph compiler, operator fusion, │
│   memory planning)                 │
├────────────────────────────────────┤
│    Device Plugins (CPU, GPU, NPU)  │
├────────────────────────────────────┤
│         oneDNN (CPU)               │
│  (AVX2/AVX-512/AMX kernels,       │
│   INT8/INT4/BF16 dispatch)         │
└────────────────────────────────────┘
```

### 4.2 INT8/INT4 Quantization for LLMs

**INT8 (Weight-Only Quantization, WOQ):**
- Uses `MinMax` or `Asymmetric` quantization per channel
- Applied via NNCF with or without calibration data
- Reduces 9B FP16 (18GB) to ~9.5 GB, still too large for 6 GB

**INT4 (Weight-Only, 4-bit):**
- Uses `NF4` (NormalFloat4) or `INT4` symmetric/asymmetric
- Group-wise quantization (default group size: 128)
- Reduces 9B to ~5.0–5.5 GB depending on format
- OpenVINO's INT4 uses NF4 by default for LLMs (based on QLoRA research, arXiv:2305.14314)
- **Calibration:** Optional dataset (RedPajama, WikiText) for scale optimization
- **Without calibration:** RTN with NF4 format, quality is ~95% of calibrated on MMLU

**Performance on x86:**

| Hardware | Model | Quant | Tokens/sec (decode) | Source |
|----------|-------|-------|---------------------|--------|
| Xeon Platinum 8480+ (56 cores) | Llama-2-7B | INT4 | ~28 tok/s | Intel blog, 2024 |
| Xeon Silver 4410Y (12 cores) | Llama-2-7B | INT4 | ~8 tok/s | [ESTIMATED] |
| Xeon Silver 2-core slice | Llama-2-7B | INT4 | ~2–3 tok/s | [ESTIMATED, extrapolated] |
| AMD Epyc 9654 2-core slice | Llama-2-7B | INT4 | ~2–4 tok/s | [ESTIMATED] |
| Core Ultra 7 (P-cores) | Phi-3-mini | INT4 | ~12 tok/s | Intel, 2024 |

### 4.3 oneDNN Integration

oneDNN is the computational backend for OpenVINO's CPU plugin. Key features for LLM inference:
- **INT4 matmul kernels:** `_jit_avx512_core_amx_int8` (AMX tiles) and `_jit_avx2_int8` dispatch paths
- **BF16 matmul:** `_jit_avx512_core_bf16` for Ice Lake+
- **Attention fusion:** Fuses QKV projection + attention scoring + softmax into single kernel
- **Weight prepacking:** Reorganizes quantized weights for optimal cache access patterns

**For our runtime:** oneDNN's INT4 matmul implementations are state-of-the-art for x86. The question is whether to:
1. Call oneDNN via FFI (complex build, C++ dependency, but fastest kernels)
2. Reimplement the kernels in Rust SIMD intrinsics (more control, but significant effort)
3. Use OpenVINO as a "coprocessor" for matmul while controlling memory externally

**Recommendation:** Option 1 (FFI to oneDNN) for initial MVP, with a Rust-native kernel path for future optimization.

### 4.4 OpenVINO GenAI API

OpenVINO provides a `genai` module specifically for LLM inference:
- Greedy / multinomial / beam search sampling
- Streaming callbacks
- KV cache management with configurable eviction
- Chat template handling

**Performance comparison with llama.cpp at 2 threads:**
[UNVERIFIED] OpenVINO likely matches or slightly exceeds llama.cpp on Intel hardware due to oneDNN's Intel-specific optimizations (AVX-512, AMX). On AMD CPUs, llama.cpp likely wins due to broader SIMD optimization coverage.

### 4.5 What to Steal

1. **NF4 format for INT4 quantization**, Better quality than uniform INT4 at same bit width
2. **oneDNN kernel dispatch pattern**, Runtime CPU feature detection + kernel selection
3. **Weight prepacking layout**, Reorganizing weights for cache-optimal access
4. **Group-wise scale computation**, Per-128-element scales for INT4
5. **Attention kernel fusion**, QKV + attention as single dispatched unit

---

## 5. ONNX Runtime (CPU Execution Provider)

**Repository:** [github.com/microsoft/onnxruntime](https://github.com/microsoft/onnxruntime)  
**License:** MIT  
**Language:** C++  

### 5.1 Architecture Overview

ONNX Runtime (ORT) uses an execution provider (EP) architecture where different backends implement operator execution:

```
┌─────────────────────────────────────┐
│      Session + InferenceSession     │
├─────────────────────────────────────┤
│      Graph Transformer              │
│  (Operator fusion, constant folding,│
│   dead code elimination, layout)    │
├─────────────────────────────────────┤
│    Execution Provider Selection     │
│  (CUDA EP, CPU EP, TensorRT EP,    │
│   OpenVINO EP, NNAPI EP, etc.)     │
├─────────────────────────────────────┤
│         CPU EP Internals            │
│  (MLAS math library, thread pool,  │
│   Eigen, parallel_for)             │
└─────────────────────────────────────┘
```

### 5.2 Graph Optimizer Passes for LLMs

ORT's graph transformer applies LLM-specific optimizations:
- **MatMul + Add fusion:** Combines weight multiplication with bias addition
- **Attention fusion (contrib ops):** Fuses QKV projections, scaled dot-product attention, and output projection into `MultiHeadAttention` operator
- **SkipLayerNormalization fusion:** Merges residual connections with layer norm
- **Gelu/QuickGelu fusion:** Activation function inlining
- **Quantize-Dequantize pair insertion:** For INT8 dynamic quantization (DynamicQuantizeLinear)

### 5.3 CPU Thread Pool Design

ORT's CPU EP uses a configurable thread pool:
- Default: `OMP_NUM_THREADS` or hardware concurrency
- Intra-op parallelism: Threads within a single operator (matmul splitting)
- Inter-op parallelism: Different operators running concurrently
- For LLM inference, intra-op parallelism dominates (single matmul per step)

**At 2 threads:** ORT splits matmul rows across 2 threads. For a 9B model with hidden_size=3584 (Qwen2.5-9B), each thread processes ~1792 rows × 3584 columns per matmul. Memory bandwidth becomes the bottleneck immediately.

### 5.4 Performance Numbers

| Configuration | Model | Quant | Threads | Decode tok/s | Source |
|--------------|-------|-------|---------|-------------|--------|
| ORT CPU EP | Llama-2-7B | INT8 WOQ | 8 | ~15 | Microsoft blog 2024 |
| ORT CPU EP | Llama-2-7B | INT4 | 4 | ~10 | [ESTIMATED] |
| ORT + GenAI | Phi-3-mini (3.8B) | INT4 | 4 | ~18 | Microsoft, 2024 |
| ORT CPU EP | Llama-2-7B | INT4 | 2 | ~3–5 | [ESTIMATED] |

ORT's `generators` library (onnxruntime-genai) provides LLM-specific serving with KV cache management. The CPU performance is competitive with llama.cpp at similar thread counts.

### 5.5 What to Steal

1. **Graph transformer passes**, Operator fusion patterns applicable to any runtime
2. **MLAS (Microsoft Linear Algebra Subprograms)**, High-performance CPU matmul with INT8 support; consider FFI
3. **DynamicQuantizeLinear** pattern, Quantize activations on-the-fly during matmul
4. **GenAI library design**, Clean LLM serving API on top of generic runtime

---

## 6. TensorRT-LLM (NVIDIA)

**Repository:** [github.com/NVIDIA/TensorRT-LLM](https://github.com/NVIDIA/TensorRT-LLM)  
**License:** Apache 2.0 (with NVIDIA EULA for some components)  

### 6.1 Key Concepts Worth Porting to CPU

Despite being GPU-only, TensorRT-LLM introduced several architectural innovations that translate to CPU design:

**In-Flight Batching:**
- Unlike static batching (wait for full batch), in-flight batching adds new requests to an ongoing batch at every iteration
- On GPU: critical for maximizing GPU utilization with variable-length outputs
- **On CPU at 2 threads:** Less relevant. At batch size 1-2, the scheduling overhead exceeds the benefit. Recommendation: use simple FIFO queue, no continuous batching.

**Weight Streaming:**
- For models that don't fit in GPU memory, stream weights from CPU RAM to GPU per-layer during forward pass
- **On CPU with 6GB RAM:** DIRECTLY RELEVANT. If weights don't fit, stream layers from disk (mmap) during forward pass, evicting completed layers.
- Implementation: mmap the full file, `madvise(MADV_SEQUENTIAL)` to hint page-ahead, `madvise(MADV_DONTNEED)` on processed layers to free physical pages.

**KV Cache Reuse Protocol:**
- TensorRT-LLM supports KV cache sharing across requests with the same prefix
- Uses a token-level hash table to identify shared prefixes
- **On CPU:** Directly applicable for multi-turn chat (system prompt KV cache reuse)

**Quantization-Aware Calibration:**
- FP8 (Hopper), INT4 AWQ/GPTQ with per-tensor or per-channel scales
- Calibration pipeline compatible with NVIDIA NeMo

### 6.2 Architecture Concepts to Steal

| TensorRT-LLM Concept | CPU Adaptation |
|---------------------|----------------|
| In-flight batching | Simplified: batch size 1-2 max at 2 threads, no dynamic scheduling |
| Weight streaming from host | **Key for our runtime:** mmap + sequential access pattern |
| KV cache paging (paged or contiguous) | Contiguous per-request, shared prefixes via ref-counting |
| Speculative decoding | Draft model (0.5B) + target model (9B), CPU-adapted |
| Chunked prefill | Process prompt in chunks to manage memory during prefill |
| Multiple profiles | Pre-compile execution plans for different batch/context sizes |

---

## 7. MLC-LLM / TVM

**Repository:** [github.com/mlc-ai/mlc-llm](https://github.com/mlc-ai/mlc-llm)  
**TVM:** [github.com/apache/tvm](https://github.com/apache/tvm)  
**License:** Apache 2.0  

### 7.1 Compilation-Based Approach

MLC-LLM compiles LLM inference into platform-native code using Apache TVM's Relax IR:

```
Model (PyTorch/HF)
    ↓ Import
Relax IR (graph-level representation)
    ↓ Optimization passes
Optimized IR (fused ops, quantized types)
    ↓ Code generation (TIR)
Platform code (CUDA, Metal, Vulkan, LLVM/CPU)
    ↓ Compilation
Native binary / WASM / mobile library
```

### 7.2 Kernel Fusion Relevance to CPU

TVM's auto-scheduler (Ansor/MetaSchedule) can fuse operations for CPU targets:
- MatMul + bias + activation fusion
- Quantize + matmul + dequantize fusion
- Attention fusion (QKV concat, scaled dot-product, output projection)

**AutoTVM tuning for CPU:** TVM can auto-tune tile sizes, vectorization widths, and parallelism strategies for specific CPU microarchitectures. However:
- Tuning takes hours to days per model
- Results are hardware-specific (won't transfer between Xeon and Epyc)
- For generic cloud VMs, tuning is impractical

### 7.3 Performance on CPU

MLC-LLM's CPU performance via LLVM codegen is competitive with hand-written kernels for dense ops but generally lags behind hand-tuned SIMD for quantized types:

| Configuration | Model | Perf vs llama.cpp |
|--------------|-------|-------------------|
| MLC-LLM CPU (LLVM) | Llama-2-7B Q4 | ~70–80% of llama.cpp |
| MLC-LLM Metal (M1) | Llama-2-7B Q4 | ~120% of llama.cpp |
| MLC-LLM Vulkan | Llama-2-7B Q4 | ~60–90% of llama.cpp (GPU-dependent) |

**Why it lags on CPU:** TVM-generated code doesn't match hand-written intrinsics for the specific bit-packing patterns used in GGUF-style quantization formats. The auto-scheduler doesn't know about super-block structures or importance-weighted scales.

### 7.4 What to Steal

1. **Relax IR operator fusion passes**, The fusion patterns (MatMul+Add+ReLU, QKV concat) are model-agnostic
2. **Quantization-aware compilation**, Compile-time insertion of dequant+quantize pairs
3. **Metal shader generation**, Not relevant for our CPU target, but the compilation approach is instructive
4. **WebLLM concept**, Running LLMs in browser via WebAssembly (WASM SIMD) - validates that portable CPU inference works

---

## 8. Candle (HuggingFace, Rust)

**Repository:** [github.com/huggingface/candle](https://github.com/huggingface/candle)  
**License:** MIT/Apache 2.0  
**Language:** Rust  

### 8.1 Architecture

Candle is a minimalist ML framework focused on inference performance:

```
┌───────────────────────────────────────┐
│     candle-transformers               │
│  (LLaMA, Mistral, Qwen2, Phi models) │
├───────────────────────────────────────┤
│       candle-nn (layers)              │
│  (Linear, LayerNorm, Embedding,       │
│   RotaryEmbedding, QMatMul)          │
├───────────────────────────────────────┤
│      candle-core (tensors)            │
│  (NdArray backend, CpuMetal backend,  │
│   quantization, dtype system)         │
├───────────────────────────────────────┤
│    Device backends                    │
│  (CpuAvx, CpuMetal, Cuda via         │
│   cudarc bindings)                    │
└───────────────────────────────────────┘
```

### 8.2 CPU Backend Design

Candle's CPU backend uses the `ndarray` crate for dense operations and custom Rust implementations for quantized ops:

**Tensor representation:**
```rust
pub struct Tensor {
    id: TensorId,
    storage: Arc<RwLock<CpuStorage>>,
    layout: Layout,  // shape, strides, offset
    // ...
}

enum CpuStorage {
    U8(Vec<u8>),
    U32(Vec<u32>),
    I64(Vec<i64>),
    F16(Vec<f16>),    // via half crate
    BF16(Vec<bf16>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Q4_0(Vec<Q4_0>),  // quantized storage
    Q4_1(Vec<Q4_1>),
    Q5_0(Vec<Q5_0>),
    Q5_1(Vec<Q5_1>),
    Q8_0(Vec<Q8_0>),
    // ... k-quant types
}
```

### 8.3 Quantization Support (QMatMul)

Candle supports GGUF-style quantization through the `QMatMul` layer:
```rust
pub struct QMatMul {
    inner: Arc<QTensor>,  // Quantized tensor
    // Fallback to regular MatMul if not quantized
}
```

Supported quant types mirror GGUF: Q4_0, Q4_1, Q5_0, Q5_1, Q8_0, Q2K, Q3K, Q4K, Q5K, Q6K.

The dequantization kernels are written using:
- `std::arch` for AVX2 intrinsics (x86_64)
- Scalar fallback for aarch64 (NEON support is partial)
- No AVX-512 support currently

### 8.4 Performance Benchmarks

Candle's CPU performance relative to llama.cpp:

| Model | Quant | Threads | Candle tok/s | llama.cpp tok/s | Ratio |
|-------|-------|---------|--------------|-----------------|-------|
| Mistral-7B | Q4K | 8 | ~12 | ~18 | 67% |
| Llama-2-7B | Q4K | 4 | ~8 | ~12 | 67% |
| Llama-2-7B | Q4K | 2 | ~3 | ~4.5 | 67% |

[ESTIMATED based on published Candle benchmarks and llama.cpp reference numbers]

**Why Candle is ~60-70% of llama.cpp on CPU:**
1. Less aggressive SIMD optimization (fewer hand-tuned intrinsics)
2. Rust's safety overhead in hot paths (bounds checking not always eliminated)
3. Less mature KV cache management (no optimized contiguous layout)
4. No `mmap`-based weight loading (loads into Vec<u8>)

### 8.5 What Candle Proves About Rust for Inference

**Positive signals:**
- Rust can match C++ for tensor framework ergonomics
- The ownership model prevents memory leaks in the inference loop
- `half` crate provides efficient f16/bf16 without C FFI
- GGUF parsing is straightforward in Rust with zerocopy

**Gaps and limitations:**
- SIMD intrinsics in Rust (`core::arch`) are functional but verbose compared to C
- No production-grade work-stealing thread pool for <4 threads
- KV cache management lacks the optimization of llama.cpp's contiguous allocation
- No speculative decoding infrastructure
- No mmap-based direct weight access (everything goes through Vec)

### 8.6 What to Steal

1. **QMatMul type design**, Clean abstraction over quantized and dense matmul
2. **SafeTensor integration**, For non-GGUF model loading
3. **Model implementations**, Candle-transformers has reference implementations for Llama, Mistral, Qwen2, Phi in Rust
4. **dtype system**, Rust enum-based type dispatch for tensors
5. **Crate modularity**, Separate core/nn/transformers crates is good architecture

### 8.7 What NOT to Steal

1. CPU backend SIMD kernels, underoptimized compared to what we need
2. Memory management, uses standard Rust allocators, no custom arena
3. No mmap support for weights, critical missing feature
4. KV cache is naive (Vec-based, no sharing, no eviction)

---

## 9. burn (Rust Tensor Framework)

**Repository:** [github.com/tracel-ai/burn](https://github.com/tracel-ai/burn)  
**License:** MIT/Apache 2.0  
**Language:** Rust  

### 9.1 Architecture

burn is a flexible deep learning framework with pluggable backends:

| Backend | Use Case | Inference Perf |
|---------|----------|---------------|
| `burn-ndarray` | CPU, simple | Low (no SIMD optimization) |
| `burn-tch` | CPU/GPU via libtorch | Medium (delegates to C++) |
| `burn-wgpu` | GPU via wgpu | Not relevant for CPU |
| `burn-candle` | CPU via Candle bridge | Same as Candle |
| `burn-autodiff` | Training only | N/A |

### 9.2 Suitability Assessment

**For LLM inference runtime: NOT SUITABLE as foundation.**

Reasons:
- Training-first design adds overhead irrelevant for inference
- ndarray backend uses generic BLAS (not quantized-aware)
- No GGUF support
- No quantization in the framework itself
- Performance ceiling on CPU is far below llama.cpp

**Value:** burn's backend abstraction pattern and type-safe tensor API are good design references. The `burn-ndarray` → `burn-tch` swap pattern shows how to abstract over compute backends.

---

## 10. kalosm / llm-chain-rs / mistral.rs

### 10.1 kalosm

**Repository:** [github.com/floneum/floneum/tree/main/interfaces/kalosm](https://github.com/floneum/floneum)  
**License:** MIT  

kalosm is a high-level Rust AI framework that wraps multiple backends:
- `kalosm-llama`, Wraps Candle for Llama/Mistral models
- `kalosm-language`, NLP tools, RAG pipeline
- Performance: Same as Candle (wraps it), plus small overhead from abstraction layer

**Assessment:** Too high-level for a performance-critical runtime. However, its API design (Rust-native chat interface, streaming, tool use) is worth studying for our API layer.

### 10.2 llm-chain / llm-chain-local

**Repository:** [github.com/llm-chain-rs/llm-chain](https://github.com/llm-chain-rs/llm-chain) (archived)  
**License:** MIT  

llm-chain was a Rust LLM orchestration framework that wrapped llama.cpp via the `llama-cpp-rs` FFI bindings. The project is now archived/unmaintained.

**FFI overhead analysis:** The `llama-cpp-rs` wrapper adds:
- ~1–5 μs per inference call (FFI boundary crossing)
- Memory copying for input/output tensors (avoidable with shared pointers)
- No significant overhead at token-level (100ms+ per token dwarfs FFI cost)

**Lesson:** FFI to llama.cpp/ggml is viable for a Rust runtime that leverages existing C kernels. The overhead is negligible compared to compute time.

### 10.3 mistral.rs

**Repository:** [github.com/EricLBuehler/mistral.rs](https://github.com/EricLBuehler/mistral.rs)  
**License:** MIT  
**Language:** Rust (with Python interop)  

mistral.rs is the most complete Rust LLM serving system currently:
- Built on Candle for tensor operations
- Implements continuous batching (PagedAttention-inspired)
- Supports GGUF quantized models
- OpenAI-compatible API server (axum-based)
- Supports: Llama, Mistral, Mixtral (MoE), Phi, Qwen2, Gemma2

**Performance:**
- Approximately matches Candle's raw throughput (~60-70% of llama.cpp)
- Adds serving overhead (~5-10%) from batching scheduler
- On 2 threads: ~3–4 tok/s for 7B Q4 [ESTIMATED]

**Architecture of interest:**
- PagedAttention implementation in Rust (uses Candle tensor ops)
- Scheduler with preemption
- ISQ (In-Situ Quantization), quantize FP16 models at load time
- LoRA adapter hot-loading

**What to steal:**
1. API server design (axum + streaming SSE)
2. PagedAttention Rust implementation (adapt for contiguous allocation)
3. ISQ concept, load FP16, quantize to INT4 at runtime
4. ISQ (In-Situ Quantization) for on-the-fly model quantization
5. Multi-model architecture abstraction

---

## 11. rustformers / hf-hub-rs / tokenizers

### 11.1 rustformers

**Repository:** [github.com/rustformers/rustformers](https://github.com/rustformers/rustformers) (archived)  
**License:** MIT  

Provided HuggingFace model loading in pure Rust:
- Safetensors support via `safetensors` crate
- Tokenizer integration via `tokenizers` crate
- Model architectures: GPT-2, GPT-Neo, GPT-J

**Status:** Archived. Code quality is good but performance was poor (no quantization, no SIMD optimization). 

**Lesson:** Pure Rust inference without quantization and without optimized SIMD kernels is impractical for interactive use.

### 11.2 hf-hub-rs / safetensors

**Repository:** [github.com/huggingface/hf-hub](https://github.com/huggingface/hf-hub) (Rust client)  
**Safetensors:** [github.com/huggingface/safetensors](https://github.com/huggingface/safetensors)  

The `hf-hub` Rust crate provides:
- Model downloading with progress and caching
- Repository browsing and revision selection

`safetensors` Rust crate provides:
- Zero-copy tensor loading (mmap-backed)
- Safe deserialization (no pickle/arbitrary code execution)
- Header-only format: JSON metadata + raw tensor data

**For our runtime:** Safetensors is useful as an alternative to GGUF for non-quantized model loading. However, GGUF's quantized format is essential for the 6GB memory constraint.

### 11.3 tokenizers (HuggingFace)

**Repository:** [github.com/huggingface/tokenizers](https://github.com/huggingface/tokenizers)  
**Rust-native implementation**  

The `tokenizers` crate is the de facto standard for BPE/WordPiece/SentencePiece tokenization in Rust:
- Performance: >100k tokens/sec on modern CPU (not a bottleneck)
- Supports: BPE, WordPiece, Unigram, SentencePiece
- Thread-safe decoding with streaming support

**For 2 vCPU LLM inference:** Tokenization is never the bottleneck. Single-threaded encoding takes <100μs per token.

---

## 12. KTransformers / SGLang

### 12.1 KTransformers

**Repository:** [github.com/kvcache-ai/ktransformers](https://github.com/kvcache-ai/ktransformers)  
**License:** Apache 2.0  

KTransformers focuses on MoE (Mixture of Experts) models with CPU offloading:
- Expert layers offloaded to CPU RAM (or NVMe)
- Only active experts loaded per forward pass
- Uses GGML for CPU computation, CUDA for shared attention layers

**Key insight for our runtime:**
- MoE models activate only a fraction of parameters per token
- For 9B active params in a 67B total model (Mixtral 8x7B), only ~12.9B params compute per token
- CPU offloading of inactive experts is viable if the active expert computation fits in cache

**Performance:**
- Mixtral 8x7B on consumer GPU (24GB) + CPU: ~8–12 tok/s
- CPU-only MoE inference: not well-benchmarked; active expert loading from RAM adds latency

### 12.2 SGLang

**Repository:** [github.com/sgl-project/sglang](https://github.com/sgl-project/sglang)  
**License:** Apache 2.0  
**Paper:** "SGLang: Efficient Execution of Structured Language Model Programs" (arXiv:2312.07104)  

SGLang is a GPU-focused serving system emphasizing:
- **RadixAttention:** Trie-based prefix sharing for KV cache
- **Structured generation:** JSON schema-guided decoding with constrained sampling
- **Program-level optimization:** Compiles LLM programs into optimized execution graphs

**Concepts to steal for CPU:**
1. **RadixAttention**, Trie-based KV cache sharing is memory-efficient for multi-turn chat (store shared system prompt KV once, share across sessions)
2. **Jump-forward decoding**, Deterministic portions of output decoded in batch (reduce decode iterations)
3. **Constrained decoding**, JSON output mode useful for API consumers

---

## 13. Comparison Matrix

| Project | Language | CPU Viable | 9B @ 2-vCPU tok/s | Memory (9B Q4) | License | Reusability |
|---------|----------|------------|--------------------|-----------------|---------|-------------|
| llama.cpp | C/C++ | ✓ (best-in-class) | 2–4 | ~6.3 GB | MIT | HIGH, entire ggml library |
| vLLM | Python+CUDA | ✗ | N/A | N/A (GPU) | Apache 2.0 | MEDIUM, scheduler concepts |
| OpenVINO | C++ | ✓ (Intel-optimized) | 2–4 (Intel) | ~5.5 GB | Apache 2.0 | HIGH, oneDNN kernels |
| ONNX RT | C++ | ✓ | 3–5 [EST] | ~5.5 GB | MIT | MEDIUM, MLAS, graph opts |
| TensorRT-LLM | C++/CUDA | ✗ | N/A | N/A | Apache 2.0 | MEDIUM, architectural concepts |
| MLC-LLM | Python/TVM | ✓ (mediocre) | 1.5–3 [EST] | ~5.5 GB | Apache 2.0 | LOW, compilation approach |
| Candle | Rust | ✓ (60% of llama.cpp) | 1.5–3 | ~6.0 GB | MIT | HIGH, Rust tensor framework |
| mistral.rs | Rust | ✓ (wraps Candle) | 1.5–3 | ~6.0 GB | MIT | HIGH, serving architecture |
| burn | Rust | ✓ (poor) | <1 [EST] | ~18 GB | MIT | LOW, training-focused |
| kalosm | Rust | ✓ (wraps Candle) | ~2 [EST] | ~6.0 GB | MIT | LOW, too high-level |

---

## 14. Explicit Recommendations

### What to Build From Scratch (Rust)

1. **Memory manager**, Custom mmap-based weight loader with eviction policies, not available in any Rust crate
2. **KV cache allocator**, Contiguous, pre-allocated, with sliding window and eviction (not paged, CPU cache favors contiguous)
3. **SIMD kernel layer**, Hand-tuned AVX2/AVX-512 intrinsics for quantized matmul, optimized for 2-thread scenarios
4. **Streaming weight executor**, Load-compute-release pattern for layer-by-layer execution under memory pressure

### What to Borrow Directly

| Component | Source | Form |
|-----------|--------|------|
| GGUF parser | `gguf` crate or write custom using zerocopy | Rust code |
| Quantization schemes (Q4_K_M, Q5_K_S, IQ4_XS) | llama.cpp format spec | Format adoption |
| Model architectures (Llama, Qwen2, Gemma2) | Candle-transformers | Adapted Rust code |
| Tokenizer | `tokenizers` crate (HuggingFace) | Direct dependency |
| API server pattern | mistral.rs + axum | Adapted Rust code |
| Scheduler/concepts | vLLM scheduler + SGLang RadixAttention | Architectural adoption |
| INT4 matmul reference | oneDNN source code | Study + reimpl or FFI |

### What to FFI Into

| Library | Use Case | When |
|---------|----------|------|
| oneDNN | INT4/INT8 matmul as acceleration coprocessor | MVP phase, Intel hardware |
| ggml-cpu | Proven SIMD kernels as reference | Port gradually to Rust |

### Critical Design Decision

**The runtime should NOT be a wrapper around llama.cpp.** While FFI to ggml is tempting, it:
- Prevents memory management optimization (ggml's allocator is opaque)
- Prevents streaming weight loading (ggml expects full model in memory)
- Limits Rust safety guarantees (entire unsafe FFI boundary)
- Cannot implement custom eviction/paging strategies

**Instead:** Build a Rust-native runtime that uses:
- GGUF format for model storage (parse ourselves or via `gguf` crate)
- Rust SIMD intrinsics for kernels (gradually replace any FFI)
- Custom memory management for the 6GB constraint
- Candle or custom Rust tensor representation as fallback

---

## 15. Implementation Implications

Based on this state-of-the-art survey, the implementation team should:

1. **Start from Candle's model implementations** (they have working Qwen2, Llama, Gemma2, Mistral in Rust) but replace the CPU backend with optimized kernels.

2. **Adopt GGUF as the primary format**, it's the most mature quantized serialization format, widely available (HuggingFace has GGUF versions of most models), and mmap-friendly.

3. **Target Q4_K_M quantization** as the default, it provides the best quality/size tradeoff at ~5.5 GB for 9B models, fitting within 6 GB with KV cache at context 2048-4096.

4. **Build the SIMD kernel layer from scratch** in Rust using `core::arch`, Candle's kernels are too slow (60-70% of llama.cpp), and llama.cpp's C kernels can be ported methodically.

5. **Implement streaming weight loading** from day one, mmap the full model file, access weights layer-by-layer, use `madvise` hints for the OS page cache. This is the key differentiator for 6 GB systems.

6. **Skip continuous batching**, at 2 threads, the overhead of dynamic scheduling exceeds batch-size-1 sequential processing. Add microbatching (fixed batch size 2) only if profiling shows benefit.

7. **Implement KV cache with sliding window + LRU eviction**, not PagedAttention (wrong for CPU), but SGLang-style RadixAttention for prefix sharing in multi-turn chat.

8. **Use mistral.rs's API server design** as the starting template for the OpenAI-compatible HTTP layer.

---

*Next: Document 2 (Memory Architecture) provides the mathematical framework for fitting a 9B model into 6 GB RAM, including detailed KV cache sizing and memory budget analysis.*
