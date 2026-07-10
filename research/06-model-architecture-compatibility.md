# Document 6: Target Model Architectures, 9B Class Models in Detail

**Research Program:** CPU-Native LLM Inference Runtime  
**Target Spec:** 9B parameter model, 2 vCPUs, 6 GB RAM, 2–5 tok/s  
**Author:** Research Agent  
**Date:** June 2025  

---

## 1. Introduction

This document provides detailed architectural analysis of candidate 9B-class models for our CPU-native inference runtime. For each model, we cover transformer architecture specifics, memory implications, quantization quality, and compatibility with our target spec (2 vCPU, 6 GB, 2–5 tok/s).

The critical model parameters that affect our runtime:
- **num_layers:** Determines forward pass depth (more layers = more sequential compute)
- **num_kv_heads (GQA ratio):** Determines KV cache size (fewer KV heads = smaller cache)
- **head_dim:** Determines per-head compute and KV size
- **hidden_size / intermediate_size:** Determines matmul dimensions (memory bandwidth)
- **Attention variant:** MHA, GQA, MQA, sliding window, affects implementation

---

## 2. Qwen2.5-7B-Instruct

**Paper:** "Qwen2.5 Technical Report" (arXiv:2412.15115)  
**HuggingFace:** `Qwen/Qwen2.5-7B-Instruct`  
**GGUF availability:** Extensive (Q2_K to Q8_0 on HuggingFace by bartowski, MaziyarPanahi, etc.)

### 2.1 Architecture Specifications

| Parameter | Value | Notes |
|-----------|-------|-------|
| num_layers | 28 | Relatively shallow |
| hidden_size | 3584 | |
| intermediate_size | 18944 | SwiGLU: gate + up = 2 × 18944 |
| num_attention_heads | 28 | |
| num_kv_heads | 4 | **7:1 GQA ratio** |
| head_dim | 128 | (3584 / 28) |
| vocab_size | 151,936 | Large multilingual vocab |
| max_position_embeddings | 32768 | Native context |
| rope_theta | 1,000,000 | High RoPE base |
| activation | SwiGLU (SiLU) | |
| normalization | RMSNorm | |
| rope_type | NTK-aware | |
| tying | true (embed = lm_head shared) | Saves ~1.1 GB memory! |

### 2.2 Memory Implications

**Weight memory at Q4_K_M:**
- Total params: ~7.62B (including 151K vocab embedding)
- Note: embedding/lm_head are tied (shared weights) → only stored once
- Effective unique weights: ~7.07B
- Q4_K_M: ~4.1 GB

**KV cache at various context lengths (FP16):**
```
KV per token = 2 × 28 × 4 × 128 × 2 = 57,344 bytes ≈ 56 KB
```

| Context | KV Cache (FP16) | KV Cache (INT8) |
|---------|----------------|-----------------|
| 2048 | 115 MB | 57 MB |
| 4096 | 229 MB | 115 MB |
| 8192 | 459 MB | 229 MB |
| 32768 | 1,835 MB | 918 MB |

**Total memory at context 4096:**
- Weights: 4.1 GB
- KV cache: 229 MB
- Activation + overhead: ~300 MB
- **Total: ~4.6 GB** ✅ (comfortable 6 GB fit)

### 2.3 GQA Ratio Advantage

The 7:1 GQA ratio (28 query heads, 4 KV heads) means:
- KV cache is 7× smaller than full MHA
- Each KV head serves 7 query heads during attention
- Attention computation: for each query head, broadcast the corresponding KV head → efficient

**Implementation note:** When computing attention, we need to replicate each KV head for 7 query heads:
```rust
// Qwen2.5-7B attention (simplified)
for kv_head in 0..4 {
    let k = load_kv(kv_head);  // [seq_len × head_dim]
    let v = load_kv_v(kv_head);
    for q_head in 0..7 {  // 7 query heads per KV head
        let q = load_query(kv_head * 7 + q_head);  // [head_dim]
        let score = dot(q, k);  // [seq_len]
        attn_out[q_head] = weighted_sum(score, v);
    }
}
```

### 2.4 Quantization Quality

From HuggingFace community benchmarks:

| Quant | PPL (MMLU) | Downstream Acc | Subjective Quality |
|-------|------------|---------------|-------------------|
| FP16 | baseline | 70.3% | Excellent |
| Q5_K_M | +0.05 | ~69.8% | Excellent |
| Q4_K_M | +0.14 | ~69.0% | Very good |
| Q4_K_S | +0.22 | ~68.5% | Good |
| Q3_K_M | +0.65 | ~67.0% | Acceptable |
| Q2_K | +2.7 | ~60% | Poor |

### 2.5 Expected Throughput at 2 vCPU

Based on llama.cpp benchmarks for similar 7B Q4_K_M models:

| CPU Type | 1 Thread | 2 Threads | Notes |
|----------|----------|-----------|-------|
| Xeon Ice Lake (2 cores) | 3.5 | 5.2 | Good bandwidth |
| Epyc Milan (2 cores) | 3.8 | 5.5 | Excellent bandwidth |
| Graviton 3 (2 ARM cores) | 4.0 | 6.0 | ARM-optimized |

**Verdict: Qwen2.5-7B is an EXCELLENT target for our runtime.** Comfortably fits 6 GB, well-supported quantization, high GQA ratio keeps KV cache small.

---

## 3. Gemma-2-9B-IT

**Paper:** "Gemma 2: Improving Open Language Models at a Practical Size" (arXiv:2408.00118)  
**HuggingFace:** `google/gemma-2-9b-it`  
**GGUF:** Available (bartowski, lmstudio-community)

### 3.1 Architecture Specifications

| Parameter | Value | Notes |
|-----------|-------|-------|
| num_layers | 42 | Deep architecture |
| hidden_size | 3584 | |
| intermediate_size | 14336 | SwiGLU |
| num_attention_heads | 16 | |
| num_kv_heads | 8 | **2:1 GQA ratio** (poor for memory) |
| head_dim | 256 | Large head dimension |
| vocab_size | 256,000 | |
| max_position_embeddings | 8192 | Fixed sliding window |
| sliding_window | 4096 | Native sliding window attention |
| attention_type | "sliding_window" | |
| rope_type | Linear | |
| normalization | RMSNorm (post) | Post-attention norm |
| logit_softcap | 30.0 | Softmax logit capping |
| query_pre_attn_scalar | 256 | Pre-attention scaling |
| final_logit_softcap | 30.0 | Final logit capping |

### 3.2 Unique Features

**Sliding Window Attention (4096 tokens):**
- Gemma-2 only attends to the last 4096 tokens regardless of total context
- This BOUNDS the KV cache requirement and attention compute
- Implementation: circular buffer of size 4096 for KV cache

**Post-Normalization:**
- Unlike standard pre-norm (norm → attention → residual), Gemma-2 uses:
  ```
  output = Norm(x + Attention(x))
  ```
- The post-attention RMSNorm means we normalize AFTER the residual connection
- This affects quantization sensitivity (normalized residuals are more stable)

**Logit Softcapping:**
- Softcap function: `c × tanh(logits / c)` where c = 30.0
- Prevents extreme logit values, improves training stability
- Must be applied during sampling (not a standard transformer operation)

**Pre-attention scaling:**
- Q is scaled by `1/sqrt(query_pre_attn_scalar)` = `1/sqrt(256)` = `1/16`
- Applied BEFORE RoPE
- Must be incorporated into the attention kernel

### 3.3 Memory Implications

**Weight memory at Q4_K_M:**
- Total params: ~9.24B
- Q4_K_M: ~5.3 GB

**KV cache (CRITICAL, much larger due to 8 KV heads × head_dim 256):**
```
KV per token = 2 × 42 × 8 × 256 × 2 = 344,064 bytes ≈ 336 KB
```

| Context | KV Cache (FP16) | KV Cache (INT8) |
|---------|----------------|-----------------|
| 2048 | 672 MB | 336 MB |
| 4096 (max window) | 1,344 MB | 672 MB |

**Total at context 4096 (sliding window max):**
- Weights: 5.3 GB
- KV cache: 1,344 MB
- Activation + overhead: ~300 MB
- **Total: ~6.9 GB** ❌ EXCEEDS 6 GB

**With INT8 KV cache:**
- Weights: 5.3 GB + KV: 672 MB + overhead: 300 MB = **~6.3 GB** ❌ Still tight

**With Q4_0 weights (lighter, 5.0 GB) + INT8 KV:**
- Total: 5.0 + 0.65 + 0.3 = **~5.95 GB** ✅ Barely fits

**Verdict: Gemma-2-9B is TIGHT at 6 GB.** Requires INT8 KV cache AND Q4_0 (not Q4_K_M) weights to fit. Quality will be slightly lower but still acceptable. The sliding window is a memory blessing, KV cache never grows beyond 4096 tokens.

### 3.4 Implementation Quirks

1. **Logit softcap must be implemented:** `logit = 30.0 * tanh(logit / 30.0)`
2. **Pre-attention Q scaling:** Must scale Q before RoPE
3. **Post-norm vs pre-norm:** LayerNorm applied after residual, not before
4. **Sliding window:** All layers use sliding window; no "full attention" layers
5. **Interleaved attention:** [UNVERIFIED] Some Gemma-2 variants interleave full and sliding window, check actual config

### 3.5 Expected Throughput

Gemma-2-9B at Q4_0 on 2 vCPU [ESTIMATED]:
- Decode: ~2.5–3.5 tok/s (slower than Llama/Qwen due to 42 layers and larger KV reads)
- The 42 layers mean 42 sequential forward steps per token vs 28-32 for others
- KV cache reads per token: 42 × 8 × 256 × 2 bytes × 4096 tokens = ~672 MB (large)

---

## 4. Llama-3.1-8B-Instruct

**Paper:** "The Llama 3 Herd of Models" (arXiv:2407.21783)  
**HuggingFace:** `meta-llama/Llama-3.1-8B-Instruct`  
**GGUF:** Extensively available

### 4.1 Architecture Specifications

| Parameter | Value | Notes |
|-----------|-------|-------|
| num_layers | 32 | |
| hidden_size | 4096 | |
| intermediate_size | 14336 | SwiGLU |
| num_attention_heads | 32 | |
| num_kv_heads | 8 | **4:1 GQA ratio** |
| head_dim | 128 | (4096 / 32) |
| vocab_size | 128,256 | |
| max_position_embeddings | 131072 | With RoPE scaling |
| rope_theta | 500,000 | |
| normalization | RMSNorm | Pre-norm |
| activation | SwiGLU (SiLU) | |

### 4.2 Memory Implications

**Weight memory at Q4_K_M:**
- Total params: ~8.03B
- Q4_K_M: ~4.9 GB

**KV cache per token:**
```
KV per token = 2 × 32 × 8 × 128 × 2 = 131,072 bytes ≈ 128 KB
```

| Context | KV Cache (FP16) | KV Cache (INT8) |
|---------|----------------|-----------------|
| 2048 | 256 MB | 128 MB |
| 4096 | 512 MB | 256 MB |
| 8192 | 1,024 MB | 512 MB |

**Total at context 4096:**
- Weights: 4.9 GB + KV: 512 MB + overhead: 300 MB = **~5.7 GB** ✅

**Total at context 8192 with INT8 KV:**
- Weights: 4.9 GB + KV: 512 MB + overhead: 300 MB = **~5.7 GB** ✅

**Verdict: Llama-3.1-8B is the OPTIMAL target.** Great balance of quality, memory fit, and KV cache efficiency. Supports context 4096-8192 within 6 GB.

### 4.3 Quantization Quality

| Quant | MMLU Acc | Notes |
|-------|----------|-------|
| FP16 | 66.7% | Meta paper |
| Q8_0 | ~66.5% | Near-lossless |
| Q5_K_M | ~66.0% | Virtually lossless |
| Q4_K_M | ~65.5% | Production quality |
| Q4_K_S | ~65.0% | Slight drop |
| Q3_K_M | ~63.0% | Noticeable degradation |

Llama-3.1-8B is known to quantize well, the model's weights are more uniformly distributed than some competitors, making quantization less lossy.

### 4.4 Expected Throughput

From llama.cpp community benchmarks and our bandwidth analysis:

| Config | 2 vCPU Tok/s | Notes |
|--------|-------------|-------|
| Q4_K_M, context 2048 | 3.5–4.5 | Standard config |
| Q4_K_M, context 4096 | 3.0–4.0 | Slightly slower (more KV reads) |
| Q5_K_M, context 2048 | 3.0–3.5 | Larger weight reads |
| Q4_0, context 2048 | 4.0–5.0 | Simpler format, less scale overhead |

---

## 5. Phi-3.5-mini (3.8B) and Phi-4 (14B)

### 5.1 Phi-3.5-mini (3.8B)

**Paper:** "Phi-3 Technical Report" (arXiv:2404.14219)  
**Architecture:**

| Parameter | Value |
|-----------|-------|
| num_layers | 32 |
| hidden_size | 3072 |
| intermediate_size | 8192 |
| num_attention_heads | 32 |
| num_kv_heads | 32 | **NO GQA (full MHA)** |
| head_dim | 96 |
| vocab_size | 32,064 |
| max_position | 4096 / 128K (SU-RoPE) |
| sliding_window | 2048 |

**Memory at Q4_K_M:**
- Weights: ~2.2 GB
- KV cache (2048 window, FP16): 2 × 32 × 32 × 96 × 2048 × 2 = ~768 MB
  - Wait: NO GQA means 32 KV heads, each 96 dim. KV per token: 2 × 32 × 32 × 96 × 2 = 393,216 bytes = 384 KB
  - At 2048 tokens: 768 MB
  - Total: 2.2 + 0.75 + 0.3 = ~3.25 GB ✅ PLENTY of headroom

**Quality:** Phi-3.5-mini punches above its weight, competitive with Llama-2-7B on many benchmarks despite being 3.8B.

**Throughput at 2 vCPU [ESTIMATED]:** 6-10 tok/s (small model, fast)

**For our runtime:** Excellent backup/fallback option. Fast, high quality for size, leaves plenty of memory headroom. However, not a 9B-class model.

### 5.2 Phi-4 (14B)

**Architecture (estimated based on Phi-3 scaling):**
- num_layers: ~40
- hidden_size: ~5120
- Weight at Q4_K_M: ~8.3 GB ❌ (exceeds 6 GB even without KV cache)

**Verdict: NOT VIABLE at 6 GB.** Too large even at Q4 quantization.

---

## 6. BitNet / Natively Low-Bit Models

### 6.1 Available BitNet Models

| Model | Params | Bits | Weight Size | Status |
|-------|--------|------|-------------|--------|
| BitNet-b1.58-2B-4T | 2B | 1.58 | ~0.4 GB | Released (Microsoft) |
| bitnet_cpp (reference) |, |, |, | Reference impl |
| Community 1-bit attempts | Various | 1-2 | Various | Experimental |

**No 9B-class BitNet model publicly available as of June 2025.**

### 6.2 BitNet b1.58: Implications for CPU Runtime

If a 9B BitNet model were released:

**Weight storage:**
- Ternary weights ({-1, 0, +1}): log₂(3) ≈ 1.585 bits per weight
- 9B × 1.585 / 8 = ~1.78 GB
- Plus: activations are FP16 (needed for residual streams, norms)

**Matmul simplification:**
- No dequantization needed, weights ARE the computation
- Matmul becomes: additions and subtractions only
- CPU: ADD/SUB instructions are 2-4× faster than MUL on most architectures
- SIMD: Can use bitwise operations to select add/sub/skip

**Kernel design for ternary matmul:**
```rust
// BitNet ternary matmul pseudocode
fn ternary_matmul(weights: &TernaryMatrix, x: &[f16]) -> Vec<f16> {
    // weights stored as two bitmaps: is_positive, is_negative
    // weight[i,j] = (is_positive[i,j] as i8) - (is_negative[i,j] as i8)
    
    let mut result = vec![0.0f32; weights.rows()];
    
    // Process 256 weights at a time (AVX2 bitmap)
    for row in 0..weights.rows() {
        let mut acc: f32 = 0.0;
        for chunk in 0..(x.len() / 256) {
            let pos_bits = load_bitmap(row, chunk);  // 256 bits = 32 bytes
            let neg_bits = load_bitmap_neg(row, chunk);
            let x_chunk = &x[chunk*256..(chunk+1)*256];
            
            // For each bit set in pos: add x[j] to accumulator
            // For each bit set in neg: subtract x[j] from accumulator
            // This is a masked sum operation
            acc += masked_sum_pos_neg(pos_bits, neg_bits, x_chunk);
        }
        result[row] = acc;
    }
    result
}
```

**Expected performance advantage:** Ternary matmul requires ~3× less memory reads than Q4 matmul → potentially 2-3× faster on memory-bound workloads. Could achieve 8-15 tok/s on 2 vCPU.

**Runtime support needed:**
- Ternary weight format parser
- Bitmap-based weight storage (not GGUF-compatible)
- Custom kernel: masked-addition-based matmul
- FP16 activation stream (not quantized activations)

### 6.3 Are Any 9B-class BitNet Models on the Horizon?

[UNVERIFIED] Signals:
- Microsoft Research has indicated larger BitNet models in development
- Community fine-tuning efforts for BitNet-7B are active on GitHub
- The BitNet paper demonstrates scaling laws that suggest 9B+ BitNet would be competitive
- Expected: 7-9B BitNet model sometime in 2025-2026

---

## 7. Mixture of Experts (MoE) at 9B Scale

### 7.1 MoE Architecture Overview

MoE models have multiple "expert" FFN modules per layer, but only a subset are activated per token:

```
Standard Transformer:
  Attention → FFN (always full size)

MoE Transformer:
  Attention → Router → TopK Experts (only K of N FFNs activated)
```

### 7.2 Llama-4 Scout (109B, MoE)

**Architecture:**
- 16 experts per layer, 1 active (top-1 routing)
- Active params per token: ~17B (of 109B total)
- Total weight size: 109B × 4.625/8 ≈ 63 GB at Q4_K_M

**Obviously NOT viable at 6 GB** due to total weight size.

### 7.3 Small MoE Models

**Mixtral 8x7B (46.7B total, ~12.9B active):** Too large.

**Qwen2-57B-A14B:** Too large.

**DeepSeekMoE-16B (16B total, 2.8B active):** 
- Total weights: ~9.3 GB at Q4_K_M ❌ (exceeds 6 GB even though only 2.8B are active per token)

**Key MoE insight for memory-constrained deployment:**
- Even though only some experts are activated per token, ALL expert weights must be loaded (or accessible)
- On CPU with mmap, inactive experts can stay on disk (only active experts page-faulted in)
- But the total file size still matters for mmap region and disk space
- **No MoE model under 6 GB total weight size currently exists at production quality**

### 7.4 MoE CPU Inference Potential

If a suitable small MoE model existed:
- **Advantage:** Per-token compute is reduced (only active experts computed)
- **On memory-bound CPU:** Weight reads per token are dramatically reduced (only active expert weights read)
- **Example:** If total model is 5 GB but only 20% of experts (1 GB) are active per token, effective weight reads = 1 GB/token → ~4× faster than dense model

**Future potential:** If a ~5 GB MoE model with ~1 GB active params becomes available, it could achieve 15-20 tok/s on 2 vCPU. **This would be transformative for our runtime.**

---

## 8. Compatibility Matrix

### 8.1 Complete Model Fitness Table

| Model | Params | Q4_K_M Size | KV at 2048 (FP16) | KV at 4096 (FP16) | Total @ 4096 | Fits 6GB? | Quality | Throughput Est. |
|-------|--------|-------------|-------------------|-------------------|-------------|-----------|---------|-----------------|
| Qwen2.5-7B | 7.62B | 4.1 GB | 115 MB | 229 MB | ~4.6 GB | ✅ Yes | ⭐⭐⭐⭐⭐ | 5+ tok/s |
| Llama-3.1-8B | 8.03B | 4.9 GB | 256 MB | 512 MB | ~5.7 GB | ✅ Yes | ⭐⭐⭐⭐ | 3-4 tok/s |
| Gemma-2-9B | 9.24B | 5.3 GB | 672 MB | 1344 MB | ~6.9 GB | ❌ (Q4_0+INT8 OK) | ⭐⭐⭐⭐⭐ | 2.5-3.5 tok/s |
| Phi-3.5-mini | 3.8B | 2.2 GB | 768 MB (2K win) |, | ~3.3 GB | ✅ Plenty | ⭐⭐⭐⭐ | 6-10 tok/s |
| Phi-4 (14B) | 14B | 8.3 GB |, |, |, | ❌ Too big | N/A | N/A |

### 8.2 Quantization Format Compatibility

| Model | GGUF Q4_K_M | GGUF Q5_K_M | GPTQ-4bit | AWQ-4bit | OpenVINO INT4 |
|-------|-------------|-------------|-----------|----------|---------------|
| Qwen2.5-7B | ✅ | ✅ | ✅ | ✅ | ✅ |
| Llama-3.1-8B | ✅ | ✅ | ✅ | ✅ | ✅ |
| Gemma-2-9B | ✅ | ✅ | ⚠️ (softcap) | ⚠️ | ✅ |
| Phi-3.5-mini | ✅ | ✅ | ✅ | ✅ | ✅ |

### 8.3 Runtime Implementation Requirements Per Model

| Model | Special Handling Needed |
|-------|----------------------|
| Qwen2.5-7B | Standard. NTK-aware RoPE. Tied embeddings. |
| Llama-3.1-8B | Standard. RoPE with frequency scaling (for >8K context). |
| Gemma-2-9B | Logit softcap. Pre-attention Q scaling. Sliding window. Post-norm. |
| Phi-3.5-mini | SU-RoPE (scaling). Sliding window. No GQA (full MHA). |

### 8.4 Ranking for Target Spec

**Best to worst for 6GB/2vCPU target:**

1. **Qwen2.5-7B**, Best overall: fits easily, fast, high GQA, excellent community support
2. **Llama-3.1-8B**, Close second: fits at context 4096, excellent quality, well-known
3. **Gemma-2-9B**, Tight fit: needs INT8 KV + Q4_0, but highest quality at 9B scale; sliding window is a memory gift
4. **Phi-3.5-mini**, Fallback: fast and light, but only 3.8B params (lower capability ceiling)

---

## 9. Architecture-Specific Implementation Notes

### 9.1 SwiGLU (All Models)

All target models use SwiGLU for the feed-forward network:

```
FFN(x) = (SiLU(W_gate × x) ⊙ (W_up × x)) × W_down
```

Where:
- `W_gate`: [intermediate_size × hidden_size]
- `W_up`: [intermediate_size × hidden_size]
- `W_down`: [hidden_size × intermediate_size]
- `SiLU(silu) = x × σ(x)` (SiLU = Swish activation)
- `⊙` is element-wise multiplication

**Memory for FFN (Llama-3.1-8B):**
- W_gate + W_up: 2 × 14336 × 4096 × 4.625/8 = ~68 MB
- W_down: 4096 × 14336 × 4.625/8 = ~34 MB
- Total FFN weights per layer: ~102 MB
- Across 32 layers: ~3.26 GB (bulk of model!)

**Computation:**
1. Two matmuls (gate and up) in parallel or sequential, can fuse into one wider matmul
2. Element-wise SiLU + multiply (trivial compute)
3. One matmul (down)

**Optimization:** Fuse gate+up into a single `[2*intermediate × hidden]` matmul, reducing memory read overhead.

### 9.2 RoPE (Rotary Position Embedding)

All target models use RoPE with different configurations:

```rust
fn apply_rope(x: &[f16], position: usize, dim: usize, theta: f32) -> Vec<f16> {
    let mut result = x.to_vec();
    for i in (0..dim).step_by(2) {
        let freq = 1.0 / (theta.powf(i as f32 / dim as f32));
        let angle = position as f32 * freq;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        let x0 = result[i];
        let x1 = result[i + 1];
        result[i] = x0 * cos_a - x1 * sin_a;     // Rotation
        result[i + 1] = x0 * sin_a + x1 * cos_a;  // Rotation
    }
    result
}
```

**Performance:** RoPE is lightweight, O(head_dim) per head per token. Not a bottleneck.

**SIMD optimization:** Process pairs of elements using AVX2 multiply-add instructions. For head_dim=128, that's 64 rotation pairs per head.

### 9.3 RMSNorm

```rust
fn rms_norm(x: &[f16], weight: &[f16], eps: f32) -> Vec<f16> {
    let n = x.len() as f32;
    let rms: f32 = x.iter().map(|v| (*v as f32).powi(2)).sum::<f32>() / n;
    let inv_rms = 1.0 / (rms + eps).sqrt();
    
    x.iter().zip(weight.iter())
        .map(|(xi, wi)| *xi as f32 * inv_rms * *wi as f32)
        .map(|v| v as f16)
        .collect()
}
```

**Performance:** O(hidden_dim) with two passes (sum of squares, then normalize). Trivial compute.

---

## 10. Implementation Implications

### 10.1 Primary Target: Llama-3.1-8B

1. **Best overall fit for 6 GB** at Q4_K_M with context 4096
2. **Standard architecture**, no exotic features (no softcap, no post-norm quirks)
3. **Massive community support**, GGUF models widely available
4. **4:1 GQA**, Reasonable KV cache size
5. **32 layers**, Good balance of quality vs sequential compute depth

### 10.2 Secondary Target: Qwen2.5-7B

1. **Best memory margin**, 4.1 GB weights leaves 1.9 GB for everything else
2. **7:1 GQA**, Smallest KV cache of any target
3. **Tied embeddings**, Saves ~200 MB (embed = lm_head)
4. **28 layers**, Fewer sequential compute steps = faster per token
5. **Excellent multilingual**, Good for international deployment

### 10.3 Stretch Target: Gemma-2-9B

1. **Highest quality per parameter**, Best 9B-class model on reasoning/challenge benchmarks
2. **Requires INT8 KV cache**, Implement as a runtime option
3. **Sliding window simplifies**, Fixed 4096-token context window (no context growth management)
4. **Special implementation needed**, softcap, pre-attention scaling, post-norm

### 10.4 Runtime Architecture Configuration

```rust
enum ModelArchitecture {
    Llama {
        config: LlamaConfig,
        rope_type: RopeType,
    },
    Qwen2 {
        config: Qwen2Config,
        tied_embeddings: bool,
    },
    Gemma2 {
        config: Gemma2Config,
        sliding_window: usize,
        logit_softcap: f32,
        pre_attn_scale: f32,
    },
}
```

The executor dispatches to the architecture-specific layer implementations based on the loaded model's type.

---

*The next document (Document 7: Benchmarks and Baselines) provides realistic performance targets and the complete benchmarking methodology for validating our runtime.*
