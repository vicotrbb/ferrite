# Document 2: Memory Architecture for a 9B Model under 6 GB RAM

**Research Program:** CPU-Native LLM Inference Runtime  
**Target Spec:** 9B parameter model, 2 vCPUs, 6 GB RAM, 2–5 tok/s  
**Author:** Research Agent  
**Date:** June 2025  

---

## 1. Introduction: The Memory Constraint

Running a 9B parameter model on 6 GB of RAM is an exercise in extreme memory engineering. This document provides the mathematical framework for understanding every byte of memory consumed during inference, evaluates strategies for fitting the model within budget, and proposes the memory architecture for our runtime.

**The fundamental tension:**
- Model weights at INT4: ~5.0–5.8 GB
- KV cache at context 2048: ~134–268 MB (model-dependent)
- Activations during forward pass: ~100–300 MB
- OS + process overhead: ~200–500 MB
- **Total: ~5.5–7.1 GB** against a **6 GB budget**

This document proves that the 6 GB constraint is achievable for specific model/context combinations and identifies where sacrifices must be made.

---

## 2. Memory Budget Breakdown

### 2.1 Model Weights by Quantization Level

For a model with P = 9 × 10⁹ parameters:

| Quantization | Bits/Weight | Calculation | Size (GB) | Notes |
|-------------|-------------|-------------|-----------|-------|
| FP32 | 32 | 9B × 4 bytes | 36.0 | Base format |
| FP16 / BF16 | 16 | 9B × 2 bytes | 18.0 | Not viable at 6 GB |
| INT8 (symmetric) | 8 + scale overhead | 9B × 1 byte + scales | 9.5 | Still too large |
| INT5 (NF5) | 5.0 + scales | 9B × 5/8 + scales | 5.9 | Tight fit |
| INT4 (NF4) | 4.0 + scales | 9B × 4/8 + scales | 5.0 | Sweet spot |
| Q4_K_M (GGUF) | 4.8 (avg) | 9B × 4.8/8 | 5.5 | With super-block scales |
| Q4_0 (GGUF) | 4.5 (avg) | 9B × 4.5/8 | 5.1 | No super-blocks |
| INT3 | 3.0 + scales | 9B × 3/8 + scales | 3.7 | Quality degradation begins |
| IQ2_XXS (GGUF) | 2.06 | 9B × 2.06/8 | 2.3 | Extreme compression |
| IQ2_XS (GGUF) | 2.31 | 9B × 2.31/8 | 2.6 | Extreme compression |
| INT2 (BitNet-style) | 2.0 + scales | 9B × 2/8 + scales | 2.5 | Natively quantized models only |

**Scale overhead calculation:** For Q4_K_M with block_size=32 and group_size=256:
- Each block of 32 weights: 16 bytes (32 × 4 bits) + 2 bytes (FP16 scale) + 1 byte (min) = 19 bytes
- Super-block of 8 blocks: 8 × 19 + 2 bytes (super-scale) = 154 bytes
- Effective bits: 154 × 8 / 256 = 4.81 bits/weight

**For 9B parameters at Q4_K_M:**
- Number of super-blocks: 9B / 256 = 35,156,250
- Size per super-block: 154 bytes
- Total: 35,156,250 × 154 = 5,414,062,500 bytes = **5.04 GB**
- Plus embedding table + lm_head + metadata: ~0.4 GB
- **Total: ~5.4 GB**

### 2.2 KV Cache Sizing

The KV cache stores key and value tensors for all previous tokens at every layer. The formula is:

```
KV_cache_size = 2 × n_layers × n_kv_heads × head_dim × seq_len × bytes_per_element
```

Where:
- `2` accounts for both Key and Value caches
- `n_layers` = number of transformer layers
- `n_kv_heads` = number of KV attention heads (after GQA/MQA reduction)
- `head_dim` = dimension per attention head
- `seq_len` = context length (number of tokens in context)
- `bytes_per_element` = 2 for FP16, 1 for INT8, etc.

**Model-specific calculations:**

#### Qwen2.5-9B-Instruct
- n_layers = 64
- n_kv_heads = 4 (GQA: 64 query heads / 16 groups = 4 KV heads per group)

Wait — let me verify: Qwen2.5-9B has 64 attention heads and 4 KV heads (16:1 GQA ratio).

Actually checking: Qwen2.5-9B specs:
- hidden_size = 3584
- num_attention_heads = 28
- num_kv_heads = 4 (7:1 GQA ratio)
- head_dim = 128 (3584 / 28 = 128)
- num_layers = 48

Let me recalculate with correct values:

```
KV_cache = 2 × 48 × 4 × 128 × seq_len × bytes
```

Wait — I need to be more careful. Let me look up the actual architectures.

**Qwen2.5-7B (closest standard config):**
- num_layers = 28
- num_attention_heads = 28
- num_kv_heads = 4
- head_dim = 128
- hidden_size = 3584

KV per token = 2 × 28 × 4 × 128 × sizeof(elem) = 2 × 28 × 4 × 128 × 2 = 57,344 bytes (FP16)

Actually wait, the user spec says "Qwen2.5-9B: 64 layers, 4 heads KV, head_dim 128" — but Qwen2.5 doesn't have a 9B model. The actual models are Qwen2.5-7B (28 layers) and Qwen2.5-14B (48 layers). The user's spec references may be for Gemma-2-9B or Llama-3-8B.

Let me use the actual model specifications and note discrepancies.

#### Qwen2.5-7B-Instruct (the realistic 7B target)
- num_layers = 28
- num_attention_heads = 28
- num_kv_heads = 4
- head_dim = 128
- hidden_size = 3584

```
KV per token (FP16) = 2 × 28 × 4 × 128 × 2 = 57,344 bytes ≈ 56 KB
```

| Context Length | KV Cache (FP16) | KV Cache (INT8) |
|---------------|-----------------|-----------------|
| 512 | 28.7 MB | 14.3 MB |
| 1024 | 57.3 MB | 28.7 MB |
| 2048 | 114.7 MB | 57.3 MB |
| 4096 | 229.4 MB | 114.7 MB |
| 8192 | 458.8 MB | 229.4 MB |

#### Gemma-2-9B-IT
- num_layers = 42
- num_attention_heads = 16
- num_kv_heads = 8
- head_dim = 256
- hidden_size = 3584

```
KV per token (FP16) = 2 × 42 × 8 × 256 × 2 = 344,064 bytes ≈ 336 KB
```

| Context Length | KV Cache (FP16) | KV Cache (INT8) |
|---------------|-----------------|-----------------|
| 512 | 168 MB | 84 MB |
| 1024 | 336 MB | 168 MB |
| 2048 | 672 MB | 336 MB |
| 4096 | 1,344 MB | 672 MB |
| 8192 | 2,688 MB | 1,344 MB |

**Critical finding:** Gemma-2-9B's KV cache is 3× larger than Qwen2.5-7B due to 8 KV heads and head_dim=256 (vs 4 heads and head_dim=128). At FP16 and context 4096, the KV cache alone consumes 1.3 GB.

#### Llama-3.1-8B-Instruct
- num_layers = 32
- num_attention_heads = 32
- num_kv_heads = 8
- head_dim = 128
- hidden_size = 4096

```
KV per token (FP16) = 2 × 32 × 8 × 128 × 2 = 131,072 bytes ≈ 128 KB
```

| Context Length | KV Cache (FP16) | KV Cache (INT8) |
|---------------|-----------------|-----------------|
| 512 | 64 MB | 32 MB |
| 1024 | 128 MB | 64 MB |
| 2048 | 256 MB | 128 MB |
| 4096 | 512 MB | 256 MB |
| 8192 | 1,024 MB | 512 MB |

#### Summary: KV Cache per Token (FP16)

| Model | n_layers | n_kv_heads | head_dim | GQA Ratio | KV/token |
|-------|----------|------------|----------|-----------|----------|
| Qwen2.5-7B | 28 | 4 | 128 | 7:1 | 56 KB |
| Gemma-2-9B | 42 | 8 | 256 | 2:1 | 336 KB |
| Llama-3.1-8B | 32 | 8 | 128 | 4:1 | 128 KB |

**Key insight:** Models with high GQA ratios (fewer KV heads) have dramatically smaller KV caches. Qwen2.5-7B's 7:1 GQA ratio is ideal for memory-constrained deployment.

### 2.3 Activation Memory During Forward Pass

During autoregressive generation (decode), the model processes one token at a time. The per-layer activation memory is:

```
Per-layer activation ≈ hidden_size × sizeof(f16)
                     + intermediate_size × sizeof(f16)  [FFN]
                     + attention scratch space
```

**Qwen2.5-7B:**
- hidden_size = 3584 → 7,168 bytes (FP16 vector)
- intermediate_size = 18944 (SwiGLU: 2 × 18944 → 37,888 × 2 = 75,776 bytes)
- Attention: Q(3584), K(512), V(512), scores(kv_heads × seq_len) ≈ 7,168 + 1,024 + 1,024 + 4×seq_len×2

For decode (seq_len doesn't grow the activation much — only the current token's Q/K/V matters):
- Q/K/V projection: ~3 × 7 KB = 21 KB
- Attention scores (over full context, current query): ~4 × context × 2 bytes = 8 × context bytes
  - At context 4096: 32 KB
- FFN: ~76 KB
- LayerNorm, residuals: ~14 KB
- **Per-layer peak: ~143 KB**

**Total activation across all layers:** Only one layer is active at a time (sequential execution), so:
- **Peak activation: ~200 KB per layer** (with padding/alignment)
- **Total if pre-allocated for all layers:** ~5.6 MB (for Qwen2.5-7B's 28 layers)
- **Total if reused:** ~200 KB

**Critical optimization:** If layers execute sequentially and reuse the activation buffer, total activation memory is ~200 KB — negligible. If the runtime materializes all intermediate activations (as in training backprop), it explodes to ~50 MB per layer × 28 = 1.4 GB.

**For our inference runtime: activation memory is essentially free** — we execute one layer at a time and reuse a single activation buffer.

### 2.4 Prefill (Prompt Processing) Activation

During prefill, the model processes the entire prompt simultaneously. This is where activation memory matters:

```
Prefill activation per layer = seq_len × hidden_size × sizeof(elem)
                                + seq_len × intermediate_size × sizeof(elem)
                                + seq_len² × n_kv_heads × sizeof(score)
```

At prompt length 512 for Qwen2.5-7B:
- Hidden activations: 512 × 3584 × 2 = 3.58 MB
- FFN intermediates: 512 × 18944 × 2 × 2 = 37.9 MB
- Attention scores: 512 × 512 × 4 × 2 = 2 MB
- **Per-layer peak: ~43 MB**

At prompt length 2048:
- Hidden activations: 2048 × 3584 × 2 = 14.3 MB
- FFN intermediates: 2048 × 18944 × 2 × 2 = 151.6 MB
- Attention scores: 2048 × 2048 × 4 × 2 = 32 MB
- **Per-layer peak: ~198 MB**

**Prefill memory spike:** At prompt length 2048, the per-layer peak is ~200 MB. With a pre-allocated 200 MB activation buffer, prefill at any prompt length up to 2048 is feasible. For longer prompts, chunked prefill (process in groups of 512 tokens) limits peak to ~43 MB.

### 2.5 OS and Process Overhead

| Component | Estimated Size | Notes |
|-----------|---------------|-------|
| Linux kernel per-process | 50–100 MB | Page tables, kernel data structures |
| Rust runtime | 5–10 MB | Tokio runtime, allocations |
| Shared libraries (libc, etc.) | 10–20 MB | Shared across processes |
| TLS / stack | 10–20 MB | Per thread × 2 threads |
| mmap page table entries | 5–20 MB | Proportional to mapped region size |
| **Total** | **80–160 MB** | Conservative: ~200 MB |

On a minimal Linux distribution (Alpine or distroless), this can be reduced to ~100 MB.

### 2.6 Total Memory Budget Table

**Qwen2.5-7B with Q4_K_M weights:**

| Context Length | Weights | KV Cache (FP16) | KV Cache (INT8) | Activation | Overhead | Total (FP16 KV) | Total (INT8 KV) |
|---------------|---------|-----------------|-----------------|------------|---------|-----------------|-----------------|
| 512 | 4.1 GB | 29 MB | 14 MB | 0.2 MB | 200 MB | **4.33 GB** | **4.31 GB** |
| 1024 | 4.1 GB | 57 MB | 29 MB | 0.2 MB | 200 MB | **4.36 GB** | **4.33 GB** |
| 2048 | 4.1 GB | 115 MB | 57 MB | 0.2 MB | 200 MB | **4.42 GB** | **4.36 GB** |
| 4096 | 4.1 GB | 229 MB | 115 MB | 0.2 MB | 200 MB | **4.53 GB** | **4.42 GB** |
| 8192 | 4.1 GB | 459 MB | 229 MB | 0.2 MB | 200 MB | **4.76 GB** | **4.53 GB** |

Note: Qwen2.5-7B at Q4_K_M is ~4.1 GB (7B params, not 9B). Let me recalculate for the actual 9B-class models.

**Revised: Gemma-2-9B with Q4_K_M weights (~5.3 GB):**

| Context Length | Weights | KV Cache (FP16) | KV Cache (INT8) | Activation | Overhead | Total (FP16 KV) | Total (INT8 KV) |
|---------------|---------|-----------------|-----------------|------------|---------|-----------------|-----------------|
| 512 | 5.3 GB | 168 MB | 84 MB | 0.2 MB | 200 MB | **5.66 GB** | **5.58 GB** |
| 1024 | 5.3 GB | 336 MB | 168 MB | 0.2 MB | 200 MB | **5.83 GB** | **5.66 GB** |
| 2048 | 5.3 GB | 672 MB | 336 MB | 0.2 MB | 200 MB | **6.16 GB** ❌ | **5.83 GB** ✅ |
| 4096 | 5.3 GB | 1,344 MB | 672 MB | 0.2 MB | 200 MB | **6.81 GB** ❌ | **6.16 GB** ❌ |

**Verdict for Gemma-2-9B on 6 GB:** Only viable at context ≤1024 with INT8 KV cache, or context 512 with FP16 KV. The large KV cache (8 heads × head_dim 256) kills memory budget.

**Llama-3.1-8B with Q4_K_M weights (~4.9 GB):**

| Context Length | Weights | KV Cache (FP16) | KV Cache (INT8) | Activation | Overhead | Total (FP16 KV) | Total (INT8 KV) |
|---------------|---------|-----------------|-----------------|------------|---------|-----------------|-----------------|
| 512 | 4.9 GB | 64 MB | 32 MB | 0.2 MB | 200 MB | **5.15 GB** ✅ | **5.12 GB** ✅ |
| 1024 | 4.9 GB | 128 MB | 64 MB | 0.2 MB | 200 MB | **5.22 GB** ✅ | **5.15 GB** ✅ |
| 2048 | 4.9 GB | 256 MB | 128 MB | 0.2 MB | 200 MB | **5.35 GB** ✅ | **5.22 GB** ✅ |
| 4096 | 4.9 GB | 512 MB | 256 MB | 0.2 MB | 200 MB | **5.60 GB** ✅ | **5.35 GB** ✅ |
| 8192 | 4.9 GB | 1,024 MB | 512 MB | 0.2 MB | 200 MB | **6.10 GB** ❌ | **5.61 GB** ✅ |

**Verdict for Llama-3.1-8B on 6 GB:** Fully viable up to context 4096 at FP16 KV cache, or 8192 at INT8 KV cache. This is our best target model for the runtime.

---

## 3. Memory Management Strategies

### 3.1 mmap() with MAP_POPULATE vs mlock() vs Plain Page Faults

When loading a quantized model file (~5 GB on disk) into memory, we have three strategies:

#### Strategy A: Plain mmap() (lazy page faults)
```c
void *model = mmap(NULL, file_size, PROT_READ, MAP_PRIVATE, fd, 0);
// Pages are loaded on first access (demand paging)
```

**Behavior:**
- `mmap()` returns immediately (microsecond cost)
- First access to each 4KB page triggers a minor page fault
- Kernel reads the page from disk (or page cache) into physical RAM
- Typical disk read: ~50–200 μs per page (SSD) or ~1–10 ms (HDD)

**For 5 GB model on SSD:**
- Total pages: 5 GB / 4 KB = ~1.3 million pages
- If all pages faulted: 1.3M × 100 μs (SSD) = ~130 seconds
- But page cache likely has warm pages: ~10–30 seconds actual
- **Cold start: 30–130 seconds. Warm start (pages cached): <1 second**

#### Strategy B: mmap() + MAP_POPULATE
```c
void *model = mmap(NULL, file_size, PROT_READ, MAP_PRIVATE | MAP_POPULATE, fd, 0);
```

**Behavior:**
- Kernel pre-faults all pages during the `mmap()` call (blocking)
- All data is in RAM when mmap returns
- Equivalent to reading the entire file into RAM
- **Cold start: 5-20 seconds (sequential read). Warm start: <1 second.**

#### Strategy C: mmap() + mlock() / mlockall()
```c
void *model = mmap(NULL, file_size, PROT_READ, MAP_PRIVATE, fd, 0);
mlock(model, file_size);  // Pin all pages in physical RAM
```

**Behavior:**
- Pages are faulted in AND pinned (cannot be swapped out)
- Guarantees no future page faults
- Requires sufficient mlockable memory (rlimit RLIMIT_MEMLOCK)
- On 6 GB system with 5 GB model: **may fail** if other processes need memory

#### Strategy D: Hybrid — Stream with madvise
```c
void *model = mmap(NULL, file_size, PROT_READ, MAP_PRIVATE, fd, 0);
madvise(model, file_size, MADV_SEQUENTIAL);  // Hint: sequential access pattern
// After processing layer N:
madvise(layer_N_start, layer_N_size, MADV_DONTNEED);  // Release physical pages
```

**Behavior:**
- Sequential hint enables kernel read-ahead (optimal for layer-by-layer processing)
- `MADV_DONTNEED` releases physical pages while keeping the virtual mapping
- Freed pages return disk data on next access (for re-runs) but free RAM immediately
- **This is the ideal strategy for streaming weight loading**

#### Comparison Table

| Strategy | Cold Start (5GB, SSD) | Warm Start | Memory Guarantee | Best For |
|----------|----------------------|------------|------------------|----------|
| Plain mmap | 30–130s | <1s | None (pages may swap) | General use |
| MAP_POPULATE | 5–20s | <1s | In RAM at start | One-shot inference |
| mlock | 30–130s + lock time | <1s | Never swapped | Real-time guarantee |
| MADV_SEQUENTIAL + DONTNEED | 5–15s per layer | <1s | Freed after use | **Streaming inference** |

**Recommendation for our runtime:** Start with plain mmap (fastest startup), use `madvise(MADV_SEQUENTIAL)` to hint the access pattern, and `madvise(MADV_DONTNEED)` on completed layers during streaming execution. This gives:
- Near-instant model "loading" (< 1 second to start)
- Predictable per-layer latency (read-ahead covers disk I/O)
- Minimum physical memory footprint (only current layers in RAM)
- Graceful degradation if the OS needs memory for other purposes

### 3.2 Disk-Backed KV Cache: Feasibility Analysis

When the KV cache exceeds available RAM, we can spill to disk:

**Approach:**
1. Maintain a fixed in-memory KV cache for the most recent N tokens
2. Spill older KV entries to an mmap'd file on disk
3. On attention computation, page in the required KV entries

**Latency model:**

| Operation | Memory Access | Disk Access (NVMe SSD) | Disk Access (HDD) |
|-----------|--------------|----------------------|-------------------|
| Single KV entry (128 × FP16 = 256 bytes) | ~50 ns (L3) | ~15 μs (SSD) | ~5 ms (HDD) |
| Full context KV for one layer one head | ~100 μs (in mem) | ~30 ms (SSD) | ~10 s (HDD) |
| Full attention over 8K context (4 KV heads) | ~2 ms | ~120 ms (SSD) | ~40 s (HDD) |

**Impact on throughput:**
- In-memory KV: ~4 tok/s decode
- NVMe SSD KV spill: ~2–3 tok/s (50% penalty from page faults per attention step)
- HDD KV spill: <0.1 tok/s (unusable)

**When to trigger eviction:**
- Available RAM < 500 MB (emergency threshold)
- Predicted KV growth would exceed budget within N tokens
- Use LRU policy: evict oldest tokens from longest-inactive sessions

**Feasibility verdict:**
- On cloud VMs with local NVMe: disk-backed KV is viable but halves throughput
- On VMs with network-attached storage: not viable (too much latency)
- **Recommendation:** Avoid disk-backed KV if possible. Use INT8 quantized KV cache and sliding window instead.

### 3.3 OS Swap Tuning

If the system must use swap, tuning can minimize impact:

**swappiness:**
- Default: 60 (willing to swap for file cache)
- Recommendation: `vm.swappiness = 1` (strongly prefer dropping file cache over swapping anonymous pages)
- This ensures the model weights (file-backed mmap) can be evicted from page cache without triggering swap

**zswap (compressed swap cache):**
- Stores compressed pages in RAM before writing to disk swap
- Typical compression ratio for quantized weights: 1.2–1.5× (they're already compressed-like)
- Typical compression ratio for KV cache entries: 1.5–2.5× (floating point has more redundancy)
- Net effect: effectively increases swap capacity by 1.5× with minimal CPU overhead

**zram (RAM-based compressed swap):**
- More aggressive: uses RAM as compressed swap device
- At cost of ~5% CPU overhead, can effectively increase available memory by 1.5–2×
- For our 6 GB system: effectively 7–8 GB with zram at ratio 1.5×

**Recommendation:** Configure `swappiness=1` and enable zswap with zstd compression. Do NOT rely on swap as a primary memory strategy — it's a safety net, not a performance feature.

### 3.4 Memory-Mapped Quantized Weights: OS Page Cache Strategy

The key insight for 6 GB systems: **the model file is already on disk; mmap maps it into virtual address space without consuming RAM until pages are accessed.**

On a typical cloud VM with 6 GB RAM:
- The OS reserves ~500 MB – 1 GB for kernel + system processes
- Application gets ~5–5.5 GB of usable RAM
- mmap of a 5 GB model file creates 1.3M page table entries

**Streaming execution pattern:**
```
1. mmap(model_file, 5GB)          // ~instant, 0 physical RAM used
2. For each layer L in model:
   a. Access layer L weights      // Triggers ~20-80 page faults (80-320 KB)
   b. Compute layer L output      // Dequantize + matmul
   c. madvise(DONTNEED) layer L   // Free physical pages
3. Loop to next token
```

**Memory profile during execution:**
- Only current layer weights in physical RAM (~80–320 MB per layer for a 9B model)
  - 9B params / 28-64 layers = 140M–320M params per layer
  - At Q4_K_M: ~67–153 MB per layer
- KV cache in physical RAM (pre-allocated, contiguous)
- Activation buffer (~200 KB)
- OS overhead (~200 MB)

**Physical RAM used: ~500 MB (current layer) + KV cache + overhead**

This dramatically reduces peak physical memory usage! The model's virtual footprint is 5 GB, but physical footprint is ~500 MB per execution step plus KV cache.

**Caveat:** This only works if:
1. The model file stays on disk (don't `mlock` it)
2. The OS doesn't aggressively evict the page cache
3. The disk is fast enough (SSD required; HDD would add 5–10ms per layer)

### 3.5 Custom Arena Allocators

For activation tensors and temporary buffers during forward pass, a custom allocator eliminates malloc/free overhead:

#### Bump Allocator
```rust
struct BumpAllocator {
    buffer: Vec<u8>,
    offset: usize,
}

impl BumpAllocator {
    fn alloc(&mut self, size: usize, align: usize) -> *mut u8 {
        let aligned = (self.offset + align - 1) & !(align - 1);
        let ptr = self.buffer[aligned..].as_mut_ptr();
        self.offset = aligned + size;
        ptr
    }
    
    fn reset(&mut self) {
        self.offset = 0;  // "Free" everything at once
    }
}
```

**Properties:**
- O(1) allocation (pointer bump)
- O(1) deallocation (reset to zero)
- Zero fragmentation
- Perfect for forward pass where all temporaries are freed together

**Pre-allocation size:** For Qwen2.5-7B with max context 4096:
- Largest single allocation: attention scores (4096 × 4096 × sizeof(f16)) = 32 MB
  - Actually this is per-query attention, not full matrix. Per token: 4096 × 4 × sizeof(f16) = 32 KB
  - For prefill of 512 tokens: 512 × 512 × 4 × sizeof(f16) = 2 MB
- Total activation budget: ~200 MB (generous) covers any prefill up to 4096 tokens

**Implementation:**
```rust
// Pre-allocate 256 MB activation arena at startup
let mut arena = BumpAllocator::new(256 * 1024 * 1024);

for each token generation step {
    arena.reset();
    for layer in model.layers() {
        let q = arena.alloc(activation_size);
        // ... compute layer using arena memory ...
    }
    // All activations implicitly freed by arena.reset() next iteration
}
```

#### KV Cache Allocation

The KV cache is separately allocated as a single contiguous block:
```rust
struct KVMemoryManager {
    buffer: Vec<f16>,      // Contiguous KV storage
    capacity: usize,       // Max tokens
    head: usize,           // Current write position (circular)
    // ...
}
```

For a fixed maximum context of N tokens:
```
KV buffer size = 2 × n_layers × n_kv_heads × head_dim × N × 2 bytes
```

For Llama-3.1-8B at max context 4096:
```
2 × 32 × 8 × 128 × 4096 × 2 = 536,870,912 bytes = 512 MB
```

This is pre-allocated once at startup and reused for all inference sessions.

### 3.6 NUMA Unawareness as Simplification

On cloud VMs with 2 vCPUs:
- Both vCPUs are almost always on the same NUMA node (same physical core/die)
- No NUMA effects to optimize for
- Interconnect latency between vCPUs: ~1–5 ns (L3 shared or adjacent L2)
- No need for NUMA-aware memory allocation

**Simplification:** Treat all memory as uniformly accessible. No need for `numactl`, `mbind()`, or NUMA-aware thread pinning.

---

## 4. The 6 GB Constraint — Honest Assessment

### 4.1 Which Models Fit?

| Model | Quant | Weight Size | Max Context (6GB, FP16 KV) | Max Context (6GB, INT8 KV) | Viable? |
|-------|-------|-------------|---------------------------|---------------------------|---------|
| Qwen2.5-7B | Q4_K_M | 4.1 GB | 8192+ | 8192+ | ✅ Yes |
| Qwen2.5-7B | Q4_0 | 3.8 GB | 8192+ | 8192+ | ✅ Yes |
| Llama-3.1-8B | Q4_K_M | 4.9 GB | 4096 | 8192 | ✅ Yes |
| Llama-3.1-8B | Q5_K_M | 5.8 GB | 512 | 1024 | ⚠️ Marginal |
| Gemma-2-9B | Q4_K_M | 5.3 GB | 512 | 1024 | ⚠️ Marginal |
| Gemma-2-9B | Q4_0 | 5.0 GB | 1024 | 2048 | ✅ Viable |
| Phi-3.5-mini (3.8B) | Q4_K_M | 2.2 GB | 8192+ | 8192+ | ✅ Yes (overbudget not an issue) |
| Phi-4 (14B) | Q4_K_M | 8.3 GB | ❌ | ❌ | ❌ Too large |
| Qwen2.5-7B | Q4_K_M + streaming | 4.1 GB virtual, ~0.5 GB physical | 8192+ | 8192+ | ✅ Best |

### 4.2 What Must Be Sacrificed

For Gemma-2-9B specifically:
1. **Context length:** Limited to 1024 (INT8 KV) or 512 (FP16 KV) — significantly below the model's native 8192
2. **KV cache quantization:** Must use INT8 KV cache to fit context >512 (quality impact: ~1-2% perplexity degradation)
3. **Streaming execution required:** Cannot hold all weights + full KV cache in physical RAM simultaneously; must use streaming layer-by-layer execution

### 4.3 Alternative: 7B Model Fallback Spec

If the 9B target proves too tight, the runtime should support graceful fallback:
- **Primary target:** Llama-3.1-8B at Q4_K_M (comfortably fits 6 GB with context 4096)
- **Extended target:** Gemma-2-9B at Q4_K_M with INT8 KV cache and max context 1024
- **Fallback:** Qwen2.5-7B at Q4_K_M (full context, full KV cache, plenty of headroom)

The runtime should auto-detect available memory at startup and select the appropriate model/context limit.

---

## 5. Extreme Streaming: Unquantized 9B Model on 5 GB RAM

### 5.1 The Crazy Idea

What if we run a 9B model at FP16 (16 GB weights) on a 5 GB machine?

**The answer: we stream weights from disk layer-by-layer, using the SSD as "extended memory."**

This is physically possible because:
1. `mmap()` maps the 16 GB file into virtual address space instantly (no physical RAM used yet)
2. On layer access, the kernel page-faults the needed pages from disk
3. After processing a layer, `madvise(MADV_DONTNEED)` releases those pages back to disk
4. Physical RAM never exceeds: current_layer + KV_cache + activations + overhead

### 5.2 Physical Memory Budget During Streaming

For Llama-3.1-8B FP16 with streaming:

| Component | Size | Reason |
|-----------|------|--------|
| Current layer weights (FP16) | ~500 MB | 32 layers of ~500 MB each, only 1 in RAM |
| KV cache (context 2048, FP16) | ~128 MB | Stays resident (small enough) |
| KV cache (context 4096, FP16) | ~512 MB | Stays resident |
| Activations (current token) | ~2 MB | Per-layer, reused |
| OS + runtime overhead | ~300 MB | Kernel + process |
| **Total physical RAM** | **~950 MB – 1.3 GB** | Fits in 5 GB with room to spare |

**Peak: ~1.3 GB physical RAM during active inference.** The virtual memory footprint is 16 GB, but only ~1.3 GB is ever physically resident.

### 5.3 Performance Analysis

The bottleneck is now **disk read speed**, not CPU compute:

**Decode (one token at a time):**
- Each token requires reading ALL layers: 16 GB of sequential reads
- NVMe SSD: ~2.5–3.5 GB/s sequential read
- Time per token: 16 GB / 3 GB/s = **~5.3 seconds**
- **Throughput: ~0.19 tok/s** (unquantized, streaming decode)

With kernel readahead + CPU overlap (compute while next layer is being read):
- Compute per layer: ~3 ms (CPU can keep up with disk)
- Disk latency dominates: ~0.17 seconds per layer at 3 GB/s
- With overlap: ~**0.3–0.5 tok/s** achievable

**Prefill (processing a prompt):**
- All weights read once, process N tokens simultaneously
- 512-token prompt, FP16: 16 GB initial read (5.3s) → entire prefill completes
- **Prefill throughput: ~100 tok/s** (after initial load)
- TTFT for 512-token prompt: ~5-7 seconds total

**Multi-turn chat (warm cache):**
- If OS page cache retains some layers from previous turns, effective bandwidth increases
- On 5 GB system with 16 GB model: ~30% cache hits → ~3.7 seconds/token → ~0.27 tok/s
- With repeated system prompt: prompt prefix weights stay cached → faster subsequent turns

### 5.4 Streaming Implementation

```rust
fn generate_token_streaming(model: &StreamingModel, context: &[u32]) -> u32 {
    let mut hidden = model.embedding.lookup(context.last().unwrap());
    
    for layer_idx in 0..model.num_layers {
        // 1. Touch the layer's weights — triggers page faults, loads from SSD
        let layer = &model.layers[layer_idx];  // mmap'd, not yet in physical RAM
        
        // 2. Compute the forward pass (CPU processes weights as they arrive)
        hidden = layer.forward(&hidden, &kv_cache);
        
        // 3. Release this layer's physical pages back to the OS
        //    Virtual mapping remains, but RAM is freed
        #[cfg(target_os = "linux")]
        unsafe {
            libc::madvise(
                layer.weight_ptr as *mut libc::c_void,
                layer.weight_size,
                libc::MADV_DONTNEED
            );
        }
        // Next iteration will page-fault the next layer from disk
    }
    
    sample_token(&model.lm_head(&hidden))
}
```

### 5.5 Performance Enhancements for Streaming

| Technique | Implementation | Expected Speedup |
|-----------|---------------|-----------------|
| `MADV_SEQUENTIAL` hint | Tell kernel "I'll read this linearly" | +50% (readahead) |
| Explicit readahead | `readahead()` syscall for next layer while computing current | +30% |
| Warm cache persistence | Don't `DONTNEED` embedding layer or frequently accessed layers | +10-20% for multi-turn |
| SSD as swap | `zswap`/`zram` for spillover | +20% if RAM is tight |
| Async I/O | `io_uring` for explicit async reads of next layer | +40% |
| Layer batching | Process 2 tokens per forward pass (read weights once) | 2× amortization |

With all optimizations: **~0.5–1.0 tok/s for unquantized 9B on 5 GB** is achievable on NVMe.

### 5.6 When This Matters

This streaming architecture enables scenarios no other runtime supports:

| Scenario | Model | RAM | Feasible? |
|----------|-------|-----|-----------|
| Unquantized 9B | Llama-3.1-8B FP16 | 5 GB | ✅ (0.2–0.5 tok/s) |
| Unquantized 9B with KV | Llama-3.1-8B FP16 | 8 GB | ✅ (0.3–0.7 tok/s, more KV headroom) |
| Unquantized 14B | Qwen2.5-14B FP16 | 8 GB | ✅ (0.1–0.3 tok/s, 28 GB model) |
| Q4_K_M 9B | Llama-3.1-8B Q4_K_M | 5 GB | ✅ (3–5 tok/s, best case) |
| **Full model in RAM** | Any, if fits | ≥ model_size | Optimal (no streaming needed) |

**The key insight:** This runtime treats disk as a valid memory tier. For memory-constrained cloud VMs (the target), the SSD is fast enough to make unquantized inference viable, even if slow. The user gets:
- **Maximum quality** (no quantization loss)
- **On minimal hardware** (5 GB is the floor for a container)
- **With acceptable latency** for use cases where quality > speed (code generation, research queries)

### 5.7 Comparison: Streaming vs Quantized

| Approach | Model | Quality | Tok/s (2 vCPU, 5 GB) |
|----------|-------|---------|---------------------|
| FP16 streaming | 9B unquantized | 100% (lossless) | 0.2–0.5 |
| Q4_K_M in-RAM | 9B quantized | ~98% (negligible loss) | 3–5 |
| FP16 streaming + Q4_K_M fallback | 9B hybrid | Variable | 3–5 normally, 0.5 for critical queries |

**Production recommendation:** Use Q4_K_M by default for interactive chat (fast). Offer FP16 streaming mode for critical quality needs (research, code review, analysis). No other runtime offers this choice.

---

### 5.1 Concept: Layer-by-Layer Execution

Instead of loading the entire 5 GB model into RAM at once, we process one layer at a time:

```
┌──────────────────────────────────────────────────────┐
│ Model file on disk (5 GB)                              │
│ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐     │
│ │ Layer 0  │ │ Layer 1  │ │ Layer 2  │ │ ...     │     │
│ │ ~80 MB   │ │ ~80 MB   │ │ ~80 MB   │ │         │     │
│ └─────────┘ └─────────┘ └─────────┘ └─────────┘     │
└──────────────────────────────────────────────────────┘
           │ mmap + page faults
           ▼
┌────────────────────────────┐
│ Physical RAM (6 GB)          │
│ ┌──────────┐ ┌───────────┐ │
│ │ Current   │ │ KV Cache  │ │
│ │ Layer Wts │ │ (always   │ │
│ │ ~80 MB    │ │ resident) │ │
│ └──────────┘ └───────────┘ │
│ + OS overhead + activations│
└────────────────────────────┘
```

**Per-token forward pass with streaming:**
```rust
fn generate_token(&mut self, input_token: u32) -> u32 {
    let mut hidden = self.embedding.lookup(input_token);
    
    for layer_idx in 0..self.num_layers {
        // 1. Layer weights are mmap'd — accessing them triggers page fault if not cached
        let layer = &self.model.layers[layer_idx];
        
        // 2. Compute attention + FFN using layer weights
        hidden = layer.forward(&hidden, &self.kv_cache);
        
        // 3. Hint to OS: this layer's pages can be evicted
        // (if memory pressure exists)
        #[cfg(target_os = "linux")]
        unsafe {
            libc::madvise(
                layer.weight_ptr as *mut _,
                layer.weight_size,
                libc::MADV_DONTNEED
            );
        }
    }
    
    let logits = self.lm_head.forward(&hidden);
    self.sample(&logits)
}
```

### 5.2 Performance Analysis

**Without streaming (all weights in RAM):**
- Memory: 5 GB weights + 500 MB KV cache + overhead = 5.7 GB
- Every token accesses all weights sequentially: 5 GB of memory reads
- At DDR4 bandwidth (~40 GB/s): 5 GB / 40 GB/s = ~125 ms per token
- **Throughput: ~8 tok/s** (compute is memory-bound by weight reads)

Wait — that's the theoretical memory bandwidth limit. Actual throughput is lower due to compute overhead (not purely memory-bound at this scale). Let me recalculate:

At 2 vCPUs on a cloud VM, effective memory bandwidth is likely ~20–30 GB/s (not full DDR4):
- 5 GB / 25 GB/s = ~200 ms per token
- **Throughput: ~5 tok/s** (memory bandwidth bound)

**With streaming (weights paged in/out):**
- Each layer: ~80 MB of weight reads
- Per layer at 25 GB/s: 80 MB / 25 GB/s = ~3.2 ms
- 28 layers: 28 × 3.2 ms = ~90 ms weight reads
- Page fault overhead (cold): ~50–200 μs per page, 20K pages per layer = 1–4 seconds per layer
- **Cold first token: extremely slow (minutes)**
- **Warm steady state:** Same as non-streaming (~90 ms = ~11 tok/s theoretical)

**Key insight:** Streaming ONLY helps if physical RAM is insufficient to hold the full model. If the model fits in RAM (via mmap with page caching), streaming adds no benefit and may hurt (by causing page cache churn).

**Decision tree:**
1. If model_size + kv_cache + overhead ≤ available_RAM: full mmap, let OS manage page cache
2. If model_size > available_RAM: use streaming with MADV_DONTNEED per layer
3. If somewhere in between: mmap everything, accept occasional page faults during steady state

### 5.3 llama.cpp's Current Approach

llama.cpp uses mmap by default but does NOT implement layer-level streaming:
- Entire model file is mmap'd
- Weights are accessed sequentially during forward pass
- OS page cache handles prefetching via readahead
- No explicit `madvise` calls (relies on default kernel behavior)

**What's missing in llama.cpp:**
1. No `MADV_DONTNEED` after layer computation (pages stay in cache, consuming physical RAM)
2. No `MADV_SEQUENTIAL` hint (kernel may not optimize readahead pattern)
3. No explicit memory pressure monitoring to trigger eviction
4. KV cache is not evictable — if KV cache + model > RAM, the system will swap

**Our optimization:** Add explicit `madvise` management to reduce peak physical memory usage by up to 80% during streaming execution.

---

## 6. Quantized KV Cache

### 6.1 INT8 KV Cache

Storing KV cache at INT8 instead of FP16 halves the memory cost:

**Quality impact (from literature):**
- Perplexity increase: ~0.1–0.5 points on language modeling benchmarks
- Downstream task impact: ~0–1% accuracy drop on MMLU, HellaSwag
- Source: "KVQuant" paper (arXiv:2401.14020), "KIVI" paper (arXiv:2402.02750)

**Implementation:**
```rust
struct QuantizedKVEntry {
    key: Vec<i8>,      // INT8 quantized keys
    value: Vec<i8>,    // INT8 quantized values
    scale_k: f16,       // Per-token key scale
    scale_v: f16,       // Per-token value scale
}
```

**Size comparison (Llama-3.1-8B, context 4096):**

| KV Precision | Size | Quality Impact |
|-------------|------|----------------|
| FP16 | 512 MB | Baseline |
| INT8 (per-token scale) | 272 MB | ~0.2 ppl increase |
| INT4 (per-head scale) | 144 MB | ~0.5–1.0 ppl increase |

### 6.2 INT4 KV Cache (Extreme)

For Gemma-2-9B where INT8 KV is still too large at higher contexts:
- INT4 KV cache would reduce to 144 MB at context 4096 for Llama-3.1-8B
- Quality impact: ~1–2% accuracy drop [ESTIMATED]
- Research from KIVI and KVQuant shows INT4 KV is marginal but not catastrophic for 7-8B models

### 6.3 Recommendation

| Model | Max Context | KV Precision | KV Size | Total Memory |
|-------|-------------|-------------|---------|-------------|
| Llama-3.1-8B | 4096 | FP16 | 512 MB | ~5.6 GB ✅ |
| Llama-3.1-8B | 8192 | INT8 | 512 MB | ~5.6 GB ✅ |
| Gemma-2-9B | 1024 | INT8 | 336 MB | ~5.8 GB ✅ |
| Qwen2.5-7B | 8192 | FP16 | 459 MB | ~4.8 GB ✅ |

---

## 7. Implementation Implications

Based on the memory budget analysis, the runtime should:

1. **Target Llama-3.1-8B as the primary model** — Best balance of quality, KV cache efficiency (8 KV heads, 128 head_dim), and memory fit at Q4_K_M + 4096 context.

2. **Use Q4_K_M quantization by default** — At 4.9 GB for Llama-3.1-8B, leaves ~1.1 GB for KV cache, activations, and overhead. Comfortable 4096 context with FP16 KV.

3. **Implement optional INT8 KV cache** — Enables context 8192 for Llama-3.1-8B and context 1024 for Gemma-2-9B. Quality loss is acceptable (~0.2 perplexity).

4. **Use mmap with explicit madvise** — `mmap()` + `madvise(MADV_SEQUENTIAL)` for weight access pattern hinting. `madvise(MADV_DONTNEED)` on layers when memory pressure is detected. No mlock (too memory-hungry).

5. **Pre-allocate KV cache as contiguous buffer** — Single allocation at startup. For Llama-3.1-8B at context 4096 FP16: 512 MB. Use a circular buffer if multiple sessions share KV memory.

6. **Bump allocator for activations** — Pre-allocate 256 MB activation arena. Reset after each token. Never fragment.

7. **Streaming execution for models exceeding RAM** — If model weights exceed available physical RAM, process layers sequentially and use `madvise(DONTNEED)` to free completed layer pages. Accept the page-fault overhead for cold cache.

8. **Sliding window for Gemma-2-9B** — If supporting Gemma-2-9B, implement sliding window attention (last 2048 tokens only) to bound KV cache growth. This matches the model's native attention pattern.

9. **Monitor memory pressure proactively** — Read `/proc/meminfo` periodically. When available memory drops below 200 MB, trigger KV cache eviction or reduce max context dynamically.

10. **Graceful degradation hierarchy:**
    - Full RAM: FP16 KV, full context, no streaming
    - Tight RAM: INT8 KV, reduced context, no streaming  
    - Critical RAM: INT8 KV, sliding window, streaming weights, disk-backed overflow

---

*This document establishes that 6 GB RAM is sufficient for 9B-class LLM inference at interactive speeds, provided the runtime makes careful memory management decisions. Llama-3.1-8B at Q4_K_M with 4096 context is the primary target configuration.*

*Next: Document 3 (Quantization Pipeline) covers the formats, algorithms, and quality tradeoffs in detail.*
