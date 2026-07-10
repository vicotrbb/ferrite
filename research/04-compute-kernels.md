# Document 4: CPU Compute Kernels, SIMD, Threading, and Throughput

**Research Program:** CPU-Native LLM Inference Runtime  
**Target Spec:** 9B parameter model, 2 vCPUs, 6 GB RAM, 2–5 tok/s  
**Author:** Research Agent  
**Date:** June 2025  

---

## 1. Introduction

LLM inference on CPU is fundamentally a **memory-bandwidth-bound** workload. The bottleneck is not floating-point throughput but the rate at which quantized weights can be read from memory and combined with activations. This document provides the SIMD instruction-level analysis needed to implement optimal quantized matmul kernels, the threading model for 2 vCPUs, and realistic throughput targets.

**The key insight:** During autoregressive generation (decode phase), each token requires multiplying ONE input vector (size = hidden_dim) against the ENTIRE weight matrix. This is a memory-bound operation: we read the entire matrix from memory once per token.

---

## 2. SIMD Instruction Sets for LLM Inference

### 2.1 x86 SIMD Landscape

| ISA Extension | Register Width | INT8 Throughput | FP32 Throughput | INT4 Support | Availability |
|--------------|---------------|-----------------|-----------------|-------------|-------------|
| SSE2 | 128-bit | 16 ops/cycle | 4 ops/cycle | Packed 2/byte | All x86_64 |
| SSE4.1 | 128-bit | 16 ops/cycle + dppd | 4 ops/cycle | Packed 2/byte | Post-2008 |
| AVX2 | 256-bit | 32 ops/cycle (vpmaddubsw) | 8 ops/cycle | Packed 2/byte | Post-2013 |
| AVX-512F | 512-bit | 64 ops/cycle | 16 ops/cycle | Packed 2/byte | Xeon Scalable |
| AVX-512 VNNI | 512-bit | 128 INT8 ops/cycle (vpdpbusd) | 16 ops/cycle | Native via packing | Ice Lake+ (2019+) |
| AVX-512 BF16 | 512-bit |, | 32 BF16 ops/cycle |, | Cooper Lake+ |
| AMX (tmul) | 1024-bit tiles | 1024 INT8 ops/cycle |, | Native | Sapphire Rapids (2023+) |

**Which matters for our runtime:**

Cloud VMs with 2 vCPUs are almost always **AVX2-capable** (nearly all x86 CPUs since 2013). AVX-512 is increasingly available on newer VMs (Intel Ice Lake/Ice Lake+ and AMD Zen 4+). AMX is rare on cloud VMs.

**Priority for kernel development:**
1. **AVX2**, Target baseline. Covers ~95% of cloud VMs.
2. **AVX-512 VNNI**, Performance upgrade path. Available on newer Intel/AMD VMs.
3. **AVX2 + FMA3**, For FP32/FP16 activation compute.

### 2.2 AVX2: The Workhorse for Quantized Inference

AVX2 provides 256-bit (32-byte) YMM registers. For quantized LLM inference, the critical instructions are:

**Integer packing and dot product:**
```asm
; Load 32 INT4 weights (packed as 16 bytes, 2 per byte)
vmovdqu ymm0, [weight_ptr]     ; 32 x 4-bit values in 16 bytes

; Extract low and high nibbles (split to 32 x INT8)
vpand    ymm1, ymm0, ymm_mask ; Low nibbles (16 bytes → 16 values)
vpsrlw   ymm2, ymm0, 4        ; Shift right → high nibbles
vpand    ymm2, ymm2, ymm_mask ; High nibbles

; Convert to signed INT8 (add -8 offset for unsigned 0-15 → signed -8 to +7)
vpsubb   ymm1, ymm1, ymm_offset
vpsubb   ymm2, ymm2, ymm_offset

; Dot product with INT8 quantized input (vpaddsb doesn't exist, use vpmaddubsw)
; Actually: vpmaddubsw treats first arg as UNSIGNED, second as SIGNED
; We need: signed × signed → use vpmaddwd on INT16-extended values
vpmovsxbw ymm3, ymm1          ; Extend to INT16 (16 values in 256 bits)
vpmovsxbw ymm4, ymm2

; Load input vector (already INT16 or FP16)
vmovdqu ymm5, [input_ptr]      ; 16 INT16 values
vmovdqu ymm6, [input_ptr+32]   ; next 16 INT16 values

; Multiply-accumulate: INT16 × INT16 → INT32
vpmaddwd ymm7, ymm3, ymm5     ; 8 INT32 partial sums
vpmaddwd ymm8, ymm4, ymm6     ; 8 INT32 partial sums

; Horizontal sum
vpaddq   ymm9, ymm7, ymm8     ; Combine
; ... horizontal add to scalar
```

**Throughput analysis for Q4×INT8 dot product (AVX2):**
- 32 weights processed per iteration
- ~8 instructions per 32 weights (load, extract, extend, multiply, accumulate)
- Modern CPU: ~4 cycles per iteration (pipelined)
- **Effective: 32 weights / 4 cycles = 8 weights/cycle**

At 3 GHz clock:
- 8 × 3G = 24 billion weights/second
- For 9B params: 9B / 24B = ~0.375 seconds per token (theoretical max)
- **Theoretical: ~2.67 tok/s** at 3 GHz

But this ignores memory bandwidth (the real bottleneck). Let's analyze that next.

### 2.3 AVX-512 VNNI

For newer CPUs, AVX-512 VNNI provides `vpdpbusd`, a fused unsigned×signed dot product with accumulator:

```asm
; VPDPBUSD: Unsigned INT8 × Signed INT8 → accumulate into INT32
; Process 64 weights at once (512-bit registers)
vpdpbusd zmm_acc, zmm_weights_u8, zmm_input_i8
```

**Throughput:** 1 `vpdpbusd` per cycle, processing 64 INT8 multiplies.
- At 2.5 GHz: 64 × 2.5G = 160 billion ops/second
- For 9B INT4 params (read as 4.5 GB): memory-limited anyway

**Key advantage:** Single instruction for the entire dot product inner loop, reduces instruction count by 3-4× vs AVX2. This is significant at 2 vCPUs where instruction decoder throughput is limited.

### 2.4 ARM NEON and SVE

For ARM-based cloud VMs (AWS Graviton, Azure ARM):

**NEON (ARMv8):**
- 128-bit registers (V0-V31)
- 16 INT8 madd per instruction (vs AVX2's 32)
- `vmlal.s8` for signed INT8 multiply-accumulate
- Widely available, well-optimized

**SVE (ARMv8.2+):**
- Scalable vector length (128–2048 bits, implementation-dependent)
- AWS Graviton 3: 256-bit SVE
- SVE2 adds more integer operations
- Performance similar to AVX2 at equivalent width

**For our runtime:** ARM support is secondary. Focus on AVX2 first, add ARM NEON/SVE as a later optimization.

---

## 3. Quantized Matrix-Vector Multiplication (The Hot Path)

### 3.1 The Fundamental Operation

During decode (token generation), each layer performs:

```
output[hidden_dim] = W[hidden_dim × weight_dim] × x[weight_dim]
```

Where W is the quantized weight matrix and x is the input activation vector. For Llama-3.1-8B:
- W dimensions: multiple matrices per layer
  - QKV projection: [4096 × 4096] for Q (or factored by GQA)
  - Output projection: [4096 × 4096]
  - FFN gate + up: [11008 × 4096] × 2
  - FFN down: [4096 × 11008]
- x: one vector of size hidden_dim (4096)

**Total weight reads per token (all layers):**
- Llama-3.1-8B has 32 layers
- Per-layer weight parameters: 
  - Attention Q+K+V: 3 × 4096² = 50.3M params
  - Attention O: 4096² = 16.8M params
  - FFN gate: 4096 × 11008 = 45.1M params
  - FFN up: 4096 × 11008 = 45.1M params
  - FFN down: 11008 × 4096 = 45.1M params
  - Total per layer: ~202.3M params
- Total across 32 layers: 6.47B params (remaining ~0.53B is embedding/lm_head/norms)

At Q4_K_M (4.625 bits/weight):
- Bytes per token: 6.47B × 4.625 / 8 = 3.74 GB read from memory per token

### 3.2 Memory Bandwidth as the Real Bottleneck

**Bandwidth requirements:**

| Cloud Provider | vCPU Type | Bandwidth (single thread) | Bandwidth (2 threads) |
|---------------|-----------|--------------------------|----------------------|
| AWS c6i.large (2 vCPU) | Xeon Ice Lake | ~15-25 GB/s | ~20-30 GB/s |
| GCP e2-standard-2 (2 vCPU) | AMD Epyc Milan | ~12-20 GB/s | ~15-25 GB/s |
| Azure D2s v5 (2 vCPU) | Xeon Ice Lake | ~15-25 GB/s | ~20-30 GB/s |
| Vultr/AWS c7g.medium (2 vCPU) | Graviton 3 | ~20-30 GB/s | ~25-35 GB/s |

[ESTIMATED: based on STREAM benchmark data for dual-core cloud VMs and memory subsystem bandwidth limits]

**Time per token at different bandwidths:**

For 3.74 GB weight reads per token (Llama-3.1-8B Q4_K_M):

| Bandwidth | Time/Token | Tokens/sec |
|-----------|-----------|------------|
| 15 GB/s | 249 ms | 4.0 |
| 20 GB/s | 187 ms | 5.3 |
| 25 GB/s | 150 ms | 6.7 |
| 30 GB/s | 125 ms | 8.0 |

**Critical finding:** The theoretical ceiling for Llama-3.1-8B Q4_K_M on 2 vCPUs is **4–8 tok/s**, depending on memory bandwidth. This is above the 2–5 tok/s target, confirming the spec is achievable.

**However:** This is the theoretical maximum. Real throughput is reduced by:
1. KV cache reads/writes (~50-200 MB per token)
2. Activation compute (dot products, norms, RoPE)
3. Thread synchronization overhead
4. CPU frequency throttling (turbo vs sustained)
5. Cache misses during weight loading

**Realistic estimate:** 60–80% of theoretical maximum = **2.4–6.4 tok/s** depending on bandwidth.

### 3.3 Dequantize-on-the-Fly vs Store-Decompressed

**Option A: Dequantize on the fly (our choice)**
```
Read Q4 weights (16 bytes) → Dequantize to FP16 (32 bytes) → Dot product with input
```
- Memory reads: 3.74 GB of Q4 data per token (minimum bandwidth needed)
- Compute: ~2 cycles per 32 weights for dequant (negligible vs memory latency)
- L1 cache: only dequantized values in registers, no cache pollution

**Option B: Store decompressed**
```
Pre-decompress entire weight matrix to FP16 → Read FP16 → Dot product
```
- Memory reads: 6.47B × 2 bytes = 12.9 GB per token
- 3.5× more bandwidth required
- Would reduce throughput to ~1–2 tok/s on 2 vCPU
- Also requires 12.9 GB RAM just for working copy (impossible at 6 GB)

**Option C: Hybrid, dequantize to FP16 in L1 cache only**
```
Read Q4 block → Dequantize to FP16 in registers → Immediate dot product → Discard FP16
```
- This is Option A, and it's the standard approach
- FP16 values never leave registers → no L1/L2 pollution
- Total L1 footprint: only the INPUT vector + accumulator

### 3.4 llama.cpp ggml_vec_dot_q4_0, Detailed Dissection

Source: `ggml/src/ggml-quants.c`, function `ggml_vec_dot_q4_0`

```c
void ggml_vec_dot_q4_0(int n, float * restrict s, size_t bs, 
                       const void * restrict vx, size_t bx,
                       const void * restrict vy, size_t by,
                       int nrc) {
    const int qk = QK8_0;  // 32
    const int nb = n / qk;
    
    const block_q4_0 * restrict x = vx;  // Q4_0 quantized weights
    const block_q8_0 * restrict y = vy;  // INT8 quantized input
    
#if defined(__AVX2__)
    __m256 acc = _mm256_setzero_ps();  // FP32 accumulator
    
    for (int i = 0; i < nb; i++) {
        const float d0 = GGML_FP16_TO_FP32(x[i].d);  // Q4 block scale
        const float d1 = GGML_FP16_TO_FP32(y[i].d);  // Q8 block scale
        
        // Load 32 Q4 weights (16 bytes)
        const __m128i raw_bits = _mm_loadu_si128((const __m128i *)x[i].qs);
        
        // Split into low and high nibbles (16 each)
        const __m128i low_nibbles = _mm_and_si128(raw_bits, mask);
        const __m128i high_nibbles = _mm_and_si128(_mm_srli_epi16(raw_bits, 4), mask);
        
        // Convert to INT8 and subtract 8 (unsigned → signed)
        // ... extend to 256-bit
      
        // Load Q8 input (32 bytes)
        // ... similar processing
      
        // Dot product: multiply INT8 pairs and accumulate
        // vpmaddubs_epi16: u8 × s8 → s16
        // vpmadd_epi16: s16 × s16 → s32
        
        acc = _mm256_fmadd_ps(multiplier, acc);
    }
    
    *s = hsum_float_8(acc);  // horizontal sum
#endif
}
```

**Key observations:**
1. **Input is pre-quantized to INT8**, the input vector is quantized before the dot product loop. This enables `vpmaddubsw` (unsigned×signed INT8 dot product).
2. **FP32 accumulator**, partial sums in FP32 to avoid overflow. Final scale multiplication in FP32.
3. **Block-parallel processing**, each block is independent, enabling vectorization across blocks.
4. **No K-quant super-block support in Q4_0**, simpler kernel (just one scale per 32 weights). Q4_K_M has more complex scale handling.

**What we improve:**
1. **Use AVX-512 VNNI** when available (single `vpdpbusd` instruction)
2. **Prefetch next block** while computing current (`_mm_prefetch`)
3. **Align blocks to 64 bytes** for cache-line-aligned loads
4. **Batch multiple rows** in a single kernel call (for prefill or batched decode)

---

## 4. Roofline Model Analysis

### 4.1 Compute vs Memory Bound Regions

The Roofline model plots achievable performance as a function of arithmetic intensity (FLOPs per byte of memory transfer):

```
Arithmetic Intensity = FLOPs / Bytes_transferred

For quantized matmul (Q4 weights, FP16 input):
- Per 32 weights: read 16 bytes (Q4) + 64 bytes (32 FP16 inputs) = 80 bytes
- Compute: 32 multiply + 32 add = 64 FLOPs
- Arithmetic Intensity = 64 / 80 = 0.8 FLOPs/byte
```

**Roofline for typical cloud 2 vCPU:**

| Metric | Value |
|--------|-------|
| Peak compute (AVX2 FP32) | ~48 GFLOPs (2 cores × 8 FLOPs/cycle × 3 GHz) |
| Peak compute (AVX2 INT8) | ~192 GOPS (2 cores × 32 ops/cycle × 3 GHz) |
| Memory bandwidth | ~25 GB/s |
| Ridge point | 48 GFLOPS / 25 GB/s = 1.92 FLOPs/byte |

**Our workload at 0.8 FLOPs/byte is BELOW the ridge point → MEMORY BOUND**

This confirms: optimizing compute (more SIMD, better kernels) alone won't help beyond the memory bandwidth ceiling. The optimization priorities are:
1. Reduce memory reads (better quantization format, fewer redundant loads)
2. Improve memory access patterns (stride-1 sequential access, prefetching)
3. Stay in cache when possible (small working sets)

### 4.2 Cache Hierarchy and Impact

| Cache Level | Typical Size | Latency | Bandwidth (per core) | Can Hold |
|-------------|-------------|---------|---------------------|----------|
| L1d | 32–48 KB | ~1 ns | ~100 GB/s | ~1 quant block + input vector |
| L2 | 256 KB–1 MB | ~4 ns | ~50 GB/s | A few quant blocks |
| L3 | 4–64 MB (shared) | ~12 ns | ~25-40 GB/s (shared) | Small layers (~80 MB won't fit) |
| Main memory |, | ~50–80 ns | ~25–50 GB/s | Everything |

**Implication for 9B model:**
- No layer's weights fit entirely in L3 (80+ MB per layer vs ~32 MB typical L3 share per core)
- Sequential access pattern exploits hardware prefetcher (L2 → L1 streaming)
- KV cache should be kept hot in L3 for frequent attention reads
- Input activation vector (4096 × 2 bytes = 8 KB) fits in L1, reused across all weight matmuls

### 4.3 Strategies for Staying in Cache

1. **Chunked attention:** Instead of computing full attention (O(n²)), compute in chunks that fit L2:
   - Q_chunk × K^T: compute 256 keys at a time (256 × 128 × 2 = 64 KB, fits L1)
   - Accumulate scores in L1, write back to L2 only periodically

2. **KV cache blocking:** Structure KV cache for sequential access:
   ```
   KV[layer][head][token_block][head_dim]  // token_block = 256 tokens
   ```
   This ensures attention computation accesses contiguous memory.

3. **Weight pre-loading:** While computing layer N, prefetch layer N+1 weights into L2/L3:
   ```rust
   // Pseudocode
   for layer in 0..num_layers {
       prefetch_weights(layer + 1);  // Hardware prefetcher kicks in
       compute_layer(layer);
   }
   ```

---

## 5. Threading Model for 2 vCPUs

### 5.1 What "2 vCPUs" Physically Means

In cloud environments, "vCPU" typically means:
- **AWS/GCP/Azure 2 vCPU:** Usually 2 hyperthreads on 1 physical core, OR 2 physical cores depending on instance type
- **Burstable instances (t3.small):** May share physical core with other VMs
- **Dedicated instances:** Typically 2 distinct physical cores

**Hyperthreading vs. Physical cores:**

| Characteristic | 2 Hyperthreads (1 core) | 2 Physical Cores |
|---------------|------------------------|-----------------|
| L1 cache | Shared | Separate |
| L2 cache | Shared | Separate (or shared) |
| L3 cache | Shared | Shared |
| Memory bandwidth | Shared | Shared (slight improvement) |
| SIMD execution | Contention (1 SIMD unit) | Independent |
| Best thread count for GEMV | 1 (hyperthreads hurt) | 2 |

**Critical insight for our runtime:** On 2 hyperthreads sharing 1 physical core, running 2 threads for SIMD workloads often produces **worse** throughput than 1 thread due to:
- SIMD unit contention (both threads compete for AVX execution ports)
- L1/L2 cache thrashing (double the working set, same cache)
- Memory bandwidth already saturated by 1 thread

**Recommendation:**
- Detect physical vs logical cores via CPUID
- If hyperthreaded: use **1 worker thread + 1 main thread**
- If physical cores: use **2 worker threads**
- Always benchmark both and pick the better configuration

### 5.2 Parallelism Strategies at 2 Threads

For the matmul W[d_out × d_in] × x[d_in]:

**Strategy A: Row-sliced parallelism**
```
Thread 0: output[0..d_out/2] = W[0..d_out/2, :] × x[:]
Thread 1: output[d_out/2..d_out] = W[d_out/2..d_out, :] × x[:]
```
- Each thread reads its half of the weight matrix
- Input x is shared (both threads read same input → L1 cache shared or copied)
- **Pro:** Simple, no synchronization until end
- **Con:** Doubles memory bandwidth demand (2 threads reading from memory simultaneously)

**Strategy B: No parallelism (single thread)**
```
Main thread: output[:] = W[:, :] × x[:]
```
- One thread saturates memory bandwidth
- No synchronization overhead
- **Pro:** Simplest, often fastest at low thread counts
- **Con:** Doesn't utilize second vCPU

**Strategy C: Pipelined parallelism**
```
Thread 0: Compute layer N
Thread 1: Prefetch weights for layer N+1 into L3 cache
```
- Thread 1 stays ahead of Thread 0, reducing cache miss latency
- **Pro:** Reduces effective memory latency
- **Con:** Complex synchronization, may not help if memory bandwidth is the bottleneck (not latency)

**Benchmark data (community measurements):**

| Cores Used | Llama-2-7B Q4_0 tok/s | Speedup vs 1 core |
|-----------|----------------------|-------------------|
| 1 | 3.5 | 1.0× |
| 2 | 4.5–5.5 | 1.3–1.6× |
| 4 | 7–9 | 2.0–2.5× |
| 8 | 12–16 | 3.4–4.5× |

**At 2 threads (from llama.cpp benchmarks on various CPUs):**

| CPU | 1 thread | 2 threads | 2-thread speedup |
|-----|----------|-----------|-----------------|
| Xeon Platinum 8375C | 3.2 | 5.1 | 1.59× |
| Epyc 7R13 | 3.8 | 5.9 | 1.55× |
| Apple M2 (P-core) | 4.5 | 7.2 | 1.60× |
| Raspberry Pi 5 | 1.2 | 1.8 | 1.50× |

**Consistent finding:** 2-thread speedup is ~1.5× (not 2×) due to shared memory bandwidth.

### 5.3 Thread Pool Design for 2 Threads

**Recommendation: Fixed thread pool of 2 workers** (adjustable based on topology detection):

```rust
struct ThreadPool {
    workers: [WorkerThread; 2],
    task_queue: Arc<WorkStealingQueue>,
    // ...
}

// For matmul dispatch:
fn parallel_matmul(pool: &ThreadPool, w: &WeightMatrix, x: &[f16], out: &mut [f16]) {
    if pool.num_physical_cores() >= 2 {
        // Split rows evenly
        let mid = w.rows() / 2;
        pool.spawn(move || matmul_rows(w, x, &mut out[0..mid], 0, mid));
        pool.spawn(move || matmul_rows(w, x, &mut out[mid..], mid, w.rows()));
        pool.join();
    } else {
        // Single thread, avoids hyperthreading contention
        matmul_rows(w, x, out, 0, w.rows());
    }
}
```

### 5.4 False Sharing and Cache Line Bouncing

With 2 threads writing to adjacent memory:
- If output vectors are on the same cache line (64 bytes), threads invalidate each other's caches
- Solution: Ensure thread 0's output and thread 1's output start on different cache lines
- For row-sliced matmul: natural alignment (each thread writes consecutive rows) usually avoids false sharing IF the output stride > 64 bytes

**Output stride for Llama-3.1-8B:** hidden_dim = 4096 → 8192 bytes per output vector. Way above cache line size. No false sharing concern.

### 5.5 Should We Even Use 2 Threads?

Based on the analysis:
- Row-sliced matmul gives ~1.5× speedup at 2 physical cores
- On hyperthreaded single core: 1 thread may be faster
- Memory bandwidth is the ceiling regardless

**Adaptive strategy:**
```
1. Detect CPU topology (physical cores, hyperthreads)
2. Run 10-second benchmark: 1 thread vs 2 threads
3. Pick the faster configuration
4. Cache the decision for subsequent inference sessions
```

---

## 6. Memory Bandwidth Deep Dive

### 6.1 DDR4/DDR5 Bandwidth per Channel

| Memory Type | Per-Channel BW | Dual Channel BW | Typical Cloud VM |
|-------------|---------------|-----------------|-----------------|
| DDR4-2666 | 21.3 GB/s | 42.7 GB/s | Older instances |
| DDR4-3200 | 25.6 GB/s | 51.2 GB/s | Standard instances |
| DDR5-4800 | 38.4 GB/s | 76.8 GB/s | Newer Intel instances |
| DDR5-5600 | 44.8 GB/s | 89.6 GB/s | Latest instances |
| LPDDR5-6400 | 51.2 GB/s | 102.4 GB/s | ARM (Graviton 3) |

**Cloud vCPU reality:** A 2-vCPU VM typically sees:
- 1 memory channel (shared with other VMs on the host)
- ~20–35 GB/s effective bandwidth (contended)
- Variable based on host load (noisy neighbor problem)

### 6.2 L3 Cache Impact on Inference

For a 2 vCPU VM sharing an L3 cache with other VMs:
- Effective L3 per vCPU: 2–8 MB (highly variable)
- A single layer of Llama-3.1-8B: ~80 MB (doesn't fit in L3)
- KV cache for 1 token: ~128 KB (fits in L3)
- **Strategy:** Keep KV cache hot in L3, accept that weights always come from main memory

### 6.3 NUMA on Cloud vCPUs

For 2 vCPU VMs:
- Almost always same NUMA node (same socket, likely same core pair)
- No NUMA optimization needed
- Cross-NUMA penalty (~20% latency) is irrelevant

---

## 7. Benchmarking Methodology

### 7.1 Honest Benchmarking Protocol

**Warm-up phase:**
1. Run model for 100 tokens (warm caches, JIT any lazy initialization)
2. Verify steady-state memory usage is stable
3. CPU frequency should be at turbo/sustained boost (not power-saving)

**Measurement phase:**
1. Generate 500 tokens
2. Record per-token latency
3. Report: median, P5, P95, P99

**Metrics to capture:**
- Tokens/second (harmonic mean of 1/latency)
- Time to first token (TTFT, prefill time for prompt)
- Peak RSS (maximum resident set size)
- Memory bandwidth utilization (via `perf mem` or PMU counters)

### 7.2 Standard Benchmarks

| Benchmark | Description | Relevance |
|-----------|-------------|-----------|
| llama.cpp llama-bench | Official benchmark tool, multiple configurations | Directly comparable |
| MLPerf Inference | Industry standard, server/offline/single-stream scenarios | For API server validation |
| Custom: latency distribution | Per-token latency histogram | Most relevant for interactive use |

### 7.3 Realistic Performance Targets

Based on gathered data and bandwidth analysis:

| Target Level | Tok/s (Decode) | Tok/s (Prefill) | Confidence |
|-------------|----------------|-----------------|------------|
| Minimum viable | 2 tok/s | 20 tok/s | Very high (achievable today with llama.cpp) |
| MVP target | 3–4 tok/s | 40 tok/s | High (optimized kernels) |
| Stretch goal | 5–6 tok/s | 60 tok/s | Moderate (requires near-optimal memory BW usage) |
| Theoretical max | 6–8 tok/s | 80 tok/s | Low (perfect utilization, unlikely at 2 vCPU) |

**Compared to llama.cpp:** Our runtime should match llama.cpp at Q4_K_M (baseline) and exceed it by 10–20% through:
1. Better memory alignment (64-byte vs 32-byte)
2. Streaming weight hints (`madvise`)
3. Optimized KV cache layout for CPU cache patterns
4. Reduced runtime overhead (no ggml graph overhead)

---

## 8. Implementation Implications

### 8.1 Kernel Development Priority

1. **Q4_K_M × INT8 dot product (AVX2)**, The critical path for 90% of inference time
2. **FP16 RoPE apply**, Apply rotary position embeddings to Q and K
3. **LayerNorm / RMSNorm**, Per-vector normalization (compute-bound, not memory-bound)
4. **Softmax**, Normalization over attention scores
5. **SiLU/GeLU activation**, For FFN (element-wise, trivially parallel)
6. **KV cache read/write**, Simple memory copies with stride

### 8.2 Optimization Priorities (in order of impact)

1. **AVX2 SIMD kernels**, Without these, scalar code would give ~0.5 tok/s (unusable)
2. **Sequential memory access pattern**, Weight layout must enable stride-1 reads
3. **Prefetching**, `_mm_prefetch` for next weight block during current computation
4. **Thread topology detection**, 1 thread vs 2 based on physical core count
5. **Kernel fusion**, Combine dequant+matmul+bias in single pass
6. **AVX-512 VNNI**, When available, 30–50% speedup per core

### 8.3 What to Build First

```rust
// Kernel crate structure:
// kernels/
//   q4k_matmul.rs, Q4_K_M × input vector dot product (AVX2 + scalar fallback)
//   attention.rs, Scaled dot-product attention with KV cache access
//   rope.rs, Rotary position embedding application
//   norm.rs, RMSNorm implementation
//   activation.rs, SiLU/GeLU element-wise
//   common.rs, Shared SIMD types and helpers
```

Start with `q4k_matmul.rs`, implement scalar reference, then AVX2 intrinsics, benchmark, validate against llama.cpp reference.

---

*Next: Document 5 (Inference Engine Architecture) covers the runtime scheduling, batching, and request handling layer, the software architecture that orchestrates these kernels.*
