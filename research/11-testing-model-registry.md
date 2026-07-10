# Document 11: Testing Model Registry, From Tiny to Maximum

**Research Program:** CPU-Native LLM Inference Runtime  
**Date:** June 2025

---

## Philosophy: Progressive Testing

As we implement the runtime, we test against increasingly larger models. Each tier validates the previous work and stretches the next capability:

> **Start tiny, prove correctness, optimize, then scale.**
> A bug found on a 135M model takes 2 seconds to reproduce.
> The same bug on a 32B model takes 10 minutes.

---

## Tier 0: Bring-Up (135M, 360M params)

*Purpose: Prove the architecture compiles and produces a single correct token. Instant iteration loop.*

| Model | Params | Layers | Hidden | Heads | KV Heads | GQA Ratio | Head Dim | Intermediate | Vocab | Arch | Weight Sizes |
|-------|--------|--------|--------|-------|----------|-----------|----------|-------------|-------|------|-------------|
| **SmolLM2-135M-Instruct** | 135M | 30 | 576 | 9 | 3 | 3:1 | 64 | 1,536 | 49,152 | Llama | FP16: **270 MB** · Q4_K_M: **85 MB** |
| **SmolLM2-360M-Instruct** | 360M | 32 | 960 | 15 | 5 | 3:1 | 64 | 2,560 | 49,152 | Llama | FP16: **720 MB** · Q4_K_M: **220 MB** |

**Source:** `HuggingFaceTB/SmolLM2-135M-Instruct`, `HuggingFaceTB/SmolLM2-360M-Instruct` (open, Apache 2.0)

**Why these models:**
- Load instantly on any machine, no mmap streaming needed
- Forward pass in microseconds, test iteration loop is instant
- Small enough to hand-debug: dump every intermediate tensor in a fraction of a second
- Llama architecture, same code path as 8B Llama models
- GQA ratio 3:1, tests the grouped query attention path from day one
- KV cache per token: ~0.5 KB (negligible)

**Success criteria for Tier 0:**
- [ ] GGUF parser loads model successfully
- [ ] Forward pass produces output (even garbage is OK, we're testing plumbing)
- [ ] Token output matches llama.cpp reference output
- [ ] Streaming mode works (even if all weights in RAM trivially)

---

## Tier 1: Small Models (0.5B, 1.7B params)

*Purpose: Validate SIMD kernels at real matrix sizes. First meaningful throughput measurements.*

| Model | Params | Layers | Hidden | Heads | KV Heads | GQA | Head Dim | Intermediate | Vocab | Arch | FP16 | Q4_K_M | KV @2K |
|-------|--------|--------|--------|-------|----------|-----|----------|-------------|-------|------|------|--------|--------|
| **Qwen2.5-0.5B-Instruct** | 0.49B | 24 | 896 | 14 | 2 | 7:1 | 64 | 4,864 | 151,936 | Qwen2 | 980 MB | **310 MB** | 6 MB |
| **Llama-3.2-1B-Instruct** | 1.24B | 16 | 2048 | 32 | 8 | 4:1 | 64 | 8,192 | 128,256 | Llama | 2.5 GB | **780 MB** | 32 MB |
| **Qwen2.5-1.5B-Instruct** | 1.54B | 28 | 1536 | 12 | 2 | 6:1 | 128 | 8,960 | 151,936 | Qwen2 | 3.1 GB | **970 MB** | 28 MB |
| **SmolLM2-1.7B-Instruct** | 1.7B | 24 | 2048 | 32 | 32 | 1:1 | 64 | 8,192 | 49,152 | Llama | 3.4 GB | **1.1 GB** | 96 MB |

**Sources:** `Qwen/Qwen2.5-0.5B-Instruct`, `meta-llama/Llama-3.2-1B-Instruct`, `Qwen/Qwen2.5-1.5B-Instruct`, `HuggingFaceTB/SmolLM2-1.7B-Instruct`

**Key testing notes:**
- **SmolLM2-1.7B has NO GQA** (32 kv heads = 32 attention heads = full MHA). This is a critical test to ensure our attention kernel handles the no-grouping case correctly. Large KV cache (96 MB at context 2K) relative to model size.
- **Llama-3.2-1B** tests the Llama architecture family directly, validates we can run both the small and large Llama variants with the same code.
- **Qwen2.5-0.5B** tests extreme GQA (7:1 ratio), our attention kernel must handle broadcasting a single KV head to 7 query heads efficiently.
- **Qwen2.5-1.5B** tests head_dim=128 (vs 64 in the smaller models), validates our RoPE and attention kernels at the same head dimension used by 7B+ models.

**Expected throughput at 2 vCPU:** 10-30 tok/s (fast enough for comfortable interactive testing)

**Success criteria for Tier 1:**
- [ ] AVX2 kernels match scalar reference output within 0.1% relative error
- [ ] Throughput ≥ 10 tok/s on 2 vCPU (Q4_K_M)
- [ ] GQA broadcasting works correctly for ratios 1:1, 3:1, 4:1, 6:1, 7:1
- [ ] RoPE handles both head_dim=64 and head_dim=128
- [ ] KV cache grows/shrinks correctly across turns

---

## Tier 2: Medium-Small (3B, 4B params)

*Purpose: First models that feel "somewhat intelligent." Test memory management begins to matter.*

| Model | Params | Layers | Hidden | Heads | KV Heads | GQA | Head Dim | Intermediate | Vocab | Arch | FP16 | Q4_K_M | KV @4K |
|-------|--------|--------|--------|-------|----------|-----|----------|-------------|-------|------|------|--------|--------|
| **Qwen2.5-3B-Instruct** | 3.09B | 36 | 2048 | 16 | 2 | 8:1 | 128 | 11,008 | 151,936 | Qwen2 | 6.2 GB | **1.9 GB** | 115 MB |
| **Llama-3.2-3B-Instruct** | 3.21B | 28 | 3072 | 24 | 8 | 3:1 | 128 | 8,192 | 128,256 | Llama | 6.4 GB | **2.0 GB** | 192 MB |
| **SmolLM3-3B** | 3.0B | 36 | 2048 | 16 | 4 | 4:1 | 128 | 11,008 | 128,256 | SmolLM3 | 6.0 GB | **1.9 GB** | 172 MB |
| **Phi-3.5-mini-instruct** | 3.8B | 32 | 3072 | 32 | 32 | 1:1 | 96 | 8,192 | 32,064 | Phi3 | 7.6 GB | **2.4 GB** | 576 MB |
| **Phi-4-mini-instruct** | 3.8B | 32 | 3072 | 24 | 8 | 3:1 | 128 | 8,192 | 200,064 | Phi3 | 7.6 GB | **2.4 GB** | 256 MB |

**Sources:** `Qwen/Qwen2.5-3B-Instruct`, `meta-llama/Llama-3.2-3B-Instruct`, `HuggingFaceTB/SmolLM3-3B`, `microsoft/Phi-3.5-mini-instruct`, `microsoft/Phi-4-mini-instruct`

**Key testing notes:**
- **Phi-3.5-mini has NO GQA** (full MHA) and uses the **Phi3 architecture**, our first non-Llama/non-Qwen architecture. Must implement Phi-specific attention (uses rotary embeddings differently) and the Phi attention variant.
- **Phi-3.5 KV cache is ENORMOUS**, 576 MB at context 4K due to 32 KV heads × head_dim 96. This is the first model where KV cache memory management matters.
- **Qwen2.5-3B** at Q4_K_M (1.9 GB + 115 MB KV) fits comfortably on any machine. Excellent for daily development testing.
- **Phi-4-mini** has the Phi3 architecture + large vocab (200K tokens), tests that our tokenizer handles large vocabularies efficiently.

**Expected throughput at 2 vCPU:** 5-15 tok/s (Q4_K_M)

**Success criteria for Tier 2:**
- [ ] Multiple architectures supported: Llama, Qwen2, Phi3
- [ ] Phi3 architecture correctly implements its attention variant
- [ ] KV cache manager handles large KV models (Phi-3.5: 576 MB)
- [ ] Memory budget monitoring activates (>1 GB KV cache)
- [ ] Output quality subjectively "usable" for simple conversations

---

## Tier 3: Medium, Primary Target (7B, 9B params)

*Purpose: This is where the runtime proves its value. Must run at 3-5 tok/s on 2 vCPU / 6 GB.*

| Model | Params | Layers | Hidden | Heads | KV Heads | GQA | Head Dim | Intermediate | Vocab | Arch | FP16 | Q4_K_M | KV @4K |
|-------|--------|--------|--------|-------|----------|-----|----------|-------------|-------|------|------|--------|--------|
| **Qwen2.5-7B-Instruct** | 7.62B | 28 | 3584 | 28 | 4 | 7:1 | 128 | 18,944 | 152,064 | Qwen2 | 15.2 GB | **4.1 GB** | 229 MB |
| **Llama-3.1-8B-Instruct** | 8.03B | 32 | 4096 | 32 | 8 | 4:1 | 128 | 14,336 | 128,256 | Llama | 16.1 GB | **4.9 GB** | 512 MB |
| **Mistral-7B-v0.3-Instruct** | 7.25B | 32 | 4096 | 32 | 8 | 4:1 | 128 | 14,336 | 32,768 | Mistral | 14.5 GB | **4.5 GB** | 512 MB |
| **Gemma-2-9B-IT** | 9.24B | 42 | 3584 | 16 | 8 | 2:1 | *256* | 14,336 | 256,000 | Gemma2 | 18.5 GB | **5.3 GB** | *1,344 MB* |

**Sources:** `Qwen/Qwen2.5-7B-Instruct`, `meta-llama/Llama-3.1-8B-Instruct`, `mistralai/Mistral-7B-Instruct-v0.3`, `google/gemma-2-9b-it`

**Key testing notes:**

**Qwen2.5-7B, best overall test target:**
- Fits 6 GB at Q4_K_M (4.1 GB weights + 229 MB KV @ 4K + 300 MB overhead = 4.6 GB)
- 7:1 GQA ratio → smallest KV cache of any 7B+ model
- Tied embeddings (embed = lm_head shared) → saves ~200 MB weight memory
- Fastest decode of any target model (fewest layers: 28, smallest KV reads)

**Llama-3.1-8B, our reference benchmark:**
- Fits 6 GB at Q4_K_M (4.9 GB weights + 512 MB KV @ 4K = 5.7 GB)
- 32 layers → heavier sequential compute
- Most community benchmarks available for comparison

**Mistral-7B-v0.3, different family, similar arch:**
- Mistral architecture variant (mostly compatible with Llama)
- Smaller vocab (32K) → faster LM head matmul, smaller embedding
- Sliding window attention option (4096 tokens)

**Gemma-2-9B, the stress test:**
- **head_dim = 256** (double all others!), tests our attention kernel at 2× width
- **42 layers**, deepest model, most sequential compute
- **Low GQA ratio (2:1)**, massive KV cache: 1,344 MB at context 4K FP16
- Requires INT8 KV cache to fit in 6 GB
- Sliding window attention (4096 fixed), tests circular KV cache
- Post-normalization + logit softcapping, unique Gemma-2 quirks
- **This model will be last to pass all tests, it pushes every limit**

**Expected throughput at 2 vCPU (Q4_K_M):**
- Qwen2.5-7B: 4-6 tok/s
- Llama-3.1-8B: 3-5 tok/s
- Mistral-7B: 3-5 tok/s
- Gemma-2-9B: 2-3 tok/s

**Streaming FP16 (unquantized, from disk, 5+ GB RAM):**
- All models: 0.2-0.5 tok/s (Phase 6 milestone)

**Success criteria for Tier 3:**
- [ ] Llama-3.1-8B achieves ≥3 tok/s decode on 2 vCPU / 6 GB
- [ ] Qwen2.5-7B achieves ≥4 tok/s decode on 2 vCPU / 6 GB
- [ ] Gemma-2-9B works with INT8 KV cache (context 2048)
- [ ] Streaming FP16 mode: any model generates tokens on 5 GB RAM
- [ ] Gemma-2 quirks work: softcapping, post-norm, pre-attention scaling
- [ ] Memory budget enforced: no OOM at Q4_K_M + context 4096

---

## Tier 4: Large (14B, 32B params)

*Purpose: Push streaming architecture to its limits. Test the runtime can handle models 3-4× RAM size.*

| Model | Params | Layers | Hidden | Heads | KV Heads | GQA | Head Dim | Intermediate | Vocab | Arch | FP16 | Q4_K_M | KV @4K |
|-------|--------|--------|--------|-------|----------|-----|----------|-------------|-------|------|------|--------|--------|
| **Qwen2.5-14B-Instruct** | 14.77B | 48 | 5120 | 40 | 8 | 5:1 | 128 | 13,824 | 152,064 | Qwen2 | 29.5 GB | **8.2 GB** | 640 MB |
| **Phi-4** | 14.7B | 40 | 5120 | 40 | 10 | 4:1 | 128 | 17,920 | 100,352 | Phi3 | 29.4 GB | **8.2 GB** | 800 MB |
| **Qwen2.5-32B-Instruct** | 32.8B | 64 | 5120 | 40 | 8 | 5:1 | 128 | 27,648 | 152,064 | Qwen2 | 65.6 GB | **17.8 GB** | 640 MB |

**Sources:** `Qwen/Qwen2.5-14B-Instruct`, `microsoft/phi-4`, `Qwen/Qwen2.5-32B-Instruct`

**Key testing notes:**

**Qwen2.5-14B:**
- Too large for Q4_K_M in 6 GB (8.2 GB weights alone)
- Viable at Q3_K_M (~6.8 GB) or INT2 (~4.3 GB)
- Streaming FP16: 29.5 GB on NVMe → ~10 seconds/token → 0.1 tok/s
- Tests streaming architecture for models that DON'T fit in RAM

**Phi-4 (14B):**
- Phi3 architecture at scale, tests that our Phi implementation generalizes
- 10 KV heads (4:1 GQA), intermediate KV cache size
- Q4_K_M: 8.2 GB → must use streaming

**Qwen2.5-32B:**
- 64 layers, doubles the streaming read volume vs 8B models
- At Q4_K_M: 17.8 GB → streaming requires ~6 seconds/token on NVMe
- Tests memory management under extreme pressure
- If the streaming architecture works at 32B on 8 GB, it works anywhere

**Expected throughput (streaming FP16, 8 GB RAM, NVMe):**
- Qwen2.5-14B: ~0.1 tok/s
- Qwen2.5-32B: ~0.05 tok/s

**Success criteria for Tier 4:**
- [ ] Qwen2.5-14B generates tokens via streaming (any speed)
- [ ] No OOM during streaming of 32B model on 8 GB RAM
- [ ] Streaming works on SATA SSD (slower disk, adjusted prefetch)
- [ ] Graceful degradation: system remains responsive during heavy streaming

---

## Summary: The Testing Progression

| Tier | Models | Goal | When to Start |
|------|--------|------|--------------|
| **T0: Bring-Up** | SmolLM2-135M, SmolLM2-360M | Architecture compiles, produces 1 token | Phase 1, Day 1 |
| **T1: Small** | Qwen2.5-0.5B, Llama-1B, Qwen2.5-1.5B, SmolLM2-1.7B | SIMD kernels correct, 10+ tok/s | Phase 2 |
| **T2: Medium-Small** | Qwen2.5-3B, Llama-3B, SmolLM3-3B, Phi-3.5-mini, Phi-4-mini | Multi-arch support, memory management | Phase 3-4 |
| **T3: Target** | Qwen2.5-7B, Llama-8B, Mistral-7B, Gemma-2-9B | Production quality, 3-5 tok/s | Phase 5-6 |
| **T4: Large** | Qwen2.5-14B, Phi-4, Qwen2.5-32B | Streaming at scale, stretch goals | Phase 6-7 |

---

## Model Architecture Variants to Support

| Architecture | Models Using It | Key Differences |
|-------------|----------------|-----------------|
| **Llama** | SmolLM2, Llama-3.x, TinyLlama | Standard: pre-norm, SwiGLU, RoPE |
| **Qwen2** | Qwen2.5 all sizes, SmolLM3 | Similar to Llama, NTK-aware RoPE, tied embeddings |
| **Mistral** | Mistral-7B | Sliding window attention option |
| **Gemma2** | Gemma-2 family | Post-norm, logit softcap, pre-attention Q scaling, sliding window, head_dim=256 |
| **Phi3** | Phi-3.5-mini, Phi-4, Phi-4-mini | Different attention pattern, small vocab, MHA or GQA |

**Implementation order:** Llama → Qwen2 → Mistral → Phi3 → Gemma2 (each adds complexity)

---

## Special Model Categories

### Draft Models (for Speculative Decoding)

| Model | Params | Use As Draft For |
|-------|--------|-----------------|
| Qwen2.5-0.5B | 0.49B | Qwen2.5-3B or 7B |
| Llama-3.2-1B | 1.24B | Llama-3.1-8B |
| Phi-3.5-mini | 3.8B | Phi-4 (14B) |

Draft model must share tokenizer with target model.

### BitNet Candidates (Future)

| Model | Params | Notes |
|-------|--------|-------|
| BitNet-b1.58-2B-4T | 2B | Only public BitNet model (Microsoft, Oct 2024) |
| Community fine-tunes | 1-3B | Community 1-bit models via `bitnet.cpp` |

When 7B+ BitNet models appear, they become top-priority test targets (1.58-bit weights, no dequantization, potential 10+ tok/s).

---

## Memory Map: What Fits Where

| RAM | Q4_K_M | Q4_K_M + INT8 KV | Streaming FP16 |
|-----|--------|-------------------|---------------|
| **4 GB** | SmolLM2 (all), Qwen2.5-0.5B, Qwen2.5-1.5B | + Llama-3.2-1B | SmolLM2-1.7B, Qwen2.5-0.5B |
| **5 GB** | + Qwen2.5-3B, Llama-3.2-3B, Phi-3.5-mini | + Qwen2.5-3B context 8K | Qwen2.5-1.5B, Llama-1B |
| **6 GB** | + Qwen2.5-7B, Llama-8B, Mistral-7B | + Llama-8B context 8K, Gemma-2-9B ctx 2K | Qwen2.5-3B, Llama-3B |
| **8 GB** | + Phi-4 (tight) | + Gemma-2-9B context 4K | Qwen2.5-7B, Llama-8B, Mistral-7B |
| **16 GB** | + Qwen2.5-14B, Phi-4 comfortably | + Qwen2.5-32B at Q4_K_M | Qwen2.5-14B, Phi-4 |
| **32 GB** | + Qwen2.5-32B, Gemma-2-27B | All models full | Qwen2.5-32B |

---

## Download Commands for Test Suite

```bash
# Tier 0: Bring-up (instant download)
huggingface-cli download HuggingFaceTB/SmolLM2-135M-Instruct --include "*Q4_K_M*" --local-dir models/smol-135m
huggingface-cli download HuggingFaceTB/SmolLM2-360M-Instruct --include "*Q4_K_M*" --local-dir models/smol-360m

# Tier 1: Small models (~2-4 GB each)
huggingface-cli download Qwen/Qwen2.5-0.5B-Instruct-GGUF --include "*Q4_K_M*" --local-dir models/qwen-0.5b
huggingface-cli download bartowski/Llama-3.2-1B-Instruct-GGUF --include "*Q4_K_M*" --local-dir models/llama-1b
huggingface-cli download Qwen/Qwen2.5-1.5B-Instruct-GGUF --include "*Q4_K_M*" --local-dir models/qwen-1.5b

# Tier 2: Medium-small (~5-10 GB each)
huggingface-cli download Qwen/Qwen2.5-3B-Instruct-GGUF --include "*Q4_K_M*" --local-dir models/qwen-3b
huggingface-cli download bartowski/Llama-3.2-3B-Instruct-GGUF --include "*Q4_K_M*" --local-dir models/llama-3b
huggingface-cli download bartowski/Phi-3.5-mini-instruct-GGUF --include "*Q4_K_M*" --local-dir models/phi3.5-mini
huggingface-cli download bartowski/Phi-4-mini-instruct-GGUF --include "*Q4_K_M*" --local-dir models/phi4-mini

# Tier 3: Primary targets (~10-15 GB each)
huggingface-cli download Qwen/Qwen2.5-7B-Instruct-GGUF --include "*Q4_K_M*" --local-dir models/qwen-7b
huggingface-cli download bartowski/Llama-3.1-8B-Instruct-GGUF --include "*Q4_K_M*" --local-dir models/llama-8b
huggingface-cli download bartowski/Mistral-7B-Instruct-v0.3-GGUF --include "*Q4_K_M*" --local-dir models/mistral-7b
huggingface-cli download bartowski/gemma-2-9b-it-GGUF --include "*Q4_K_M*" --local-dir models/gemma-9b

# Tier 3 FP16 (for streaming tests)
huggingface-cli download Qwen/Qwen2.5-7B-Instruct --include "*.safetensors" --local-dir models/qwen-7b-fp16
huggingface-cli download meta-llama/Llama-3.1-8B-Instruct --include "*.safetensors" --local-dir models/llama-8b-fp16

# Tier 4: Large models (streaming only)
huggingface-cli download Qwen/Qwen2.5-14B-Instruct --include "*.safetensors" --local-dir models/qwen-14b-fp16
huggingface-cli download microsoft/phi-4 --include "*.safetensors" --local-dir models/phi4-fp16
huggingface-cli download Qwen/Qwen2.5-32B-Instruct --include "*.safetensors" --local-dir models/qwen-32b-fp16
```

---

*This testing registry ensures every phase of development has concrete models to validate against, from a 135M warm-up model that runs in microseconds to a 32B streaming stress test.*
