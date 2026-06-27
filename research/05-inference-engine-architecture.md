# Document 5: Runtime Architecture — Scheduling, Batching, and Request Handling

**Research Program:** CPU-Native LLM Inference Runtime  
**Target Spec:** 9B parameter model, 2 vCPUs, 6 GB RAM, 2–5 tok/s  
**Author:** Research Agent  
**Date:** June 2025  

---

## 1. Introduction

This document proposes the complete runtime architecture for our CPU-native LLM inference engine. It covers component boundaries, the Rust crate ecosystem decisions, continuous batching applicability, KV cache management, speculative decoding, prompt caching, and the HTTP API layer.

**Zero-dependency philosophy: everything is built from scratch.** No third-party crates except `libc` for syscall wrappers (`mmap`, `madvise`, `sched_getaffinity`). Every component — HTTP server, tokenizer, thread pool, memory allocator, SIMD kernels, JSON parser — is custom-implemented. This gives:
1. Total control over every allocation and cache line
2. Zero supply chain risk
3. Maximum optimization surface (no opaque crate behavior)
4. Full auditability of the entire codebase
5. Binary size <10 MB (no dependency bloat)

---

## 2. Overall Runtime Architecture

### 2.1 Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    API Server (axum)                         │
│  OpenAI-compatible HTTP + SSE streaming + WebSocket          │
├─────────────────────────────────────────────────────────────┤
│                    Request Queue                             │
│  Priority queue with backpressure, timeout handling          │
├──────────────┬──────────────┬───────────────┬───────────────┤
│  Scheduler   │ KV Cache     │  Tokenizer    │  Sampler      │
│  (FIFO +     │  Manager     │  (HF tok.     │  (temperature, │
│   priority   │  (contiguous,│   crate)      │   top-p, etc.)│
│   + preempt) │   sliding,   │               │               │
│              │   eviction)  │               │               │
├──────────────┴──────────────┴───────────────┴───────────────┤
│                   Executor (Core Loop)                       │
│  Layer-by-layer forward pass, SIMD kernel dispatch           │
├─────────────────────────────────────────────────────────────┤
│                  Kernel Layer (Rust + SIMD)                  │
│  Q4_K_M matmul, attention, RoPE, RMSNorm, activations       │
├─────────────────────────────────────────────────────────────┤
│                 Memory Manager                               │
│  mmap weights, bump allocator, KV pre-allocation             │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Crate Organization (Cargo Workspace)

```
cpu-llm-runtime/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── runtime/            # Main binary (API server + orchestration)
│   │   └── src/
│   │       ├── main.rs
│   │       ├── server.rs   # axum HTTP API
│   │       ├── scheduler.rs
│   │       └── config.rs
│   ├── executor/           # Core inference loop
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── forward.rs  # Layer-by-layer execution
│   │       ├── attention.rs
│   │       ├── sampling.rs
│   │       └── kv_cache.rs
│   ├── kernels/            # SIMD-optimized operations
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── q4k_matmul.rs  # Q4_K_M dot product (AVX2/AVX-512)
│   │       ├── matmul_f16.rs  # FP16 matmul (for small ops)
│   │       ├── attention.rs
│   │       ├── rope.rs
│   │       ├── norm.rs
│   │       └── activation.rs
│   ├── model/              # Model loading + architecture
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── gguf.rs     # GGUF parser
│   │       ├── arch.rs     # Architecture configs (Llama, Qwen2, etc.)
│   │       ├── loader.rs   # mmap-based model loading
│   │       └── tokenizer.rs
│   └── memory/             # Memory management
│       └── src/
│           ├── lib.rs
│           ├── mmap.rs     # mmap wrapper with madvise
│           ├── arena.rs    # Bump allocator
│           └── budget.rs   # Memory budget tracking
```

---

## 3. Core Execution Loop

### 3.1 Token Generation Cycle

The core loop executes once per generated token:

```rust
/// Generate one token given current context
fn generate_step(
    model: &Model,
    kv_cache: &mut KVCache,
    context: &[u32],  // token IDs in context
    last_token: u32,
) -> u32 {
    let mut hidden = model.embedding.lookup(last_token);  // [hidden_dim]
    
    // Apply layer-by-layer with RoPE position encoding
    let position = context.len();
    
    for (layer_idx, layer) in model.layers.iter().enumerate() {
        // 1. RMSNorm (pre-attention)
        let normed = rms_norm(&hidden, &layer.input_norm);
        
        // 2. Multi-head attention with KV cache
        let attn_out = attention_forward(
            &normed,
            &layer.attn_weights,  // Q, K, V, O projections
            kv_cache,
            layer_idx,
            position,
            &model.config,
        );
        
        // 3. Residual connection
        hidden.add_inplace(&attn_out);
        
        // 4. RMSNorm (pre-FFN)  
        let normed = rms_norm(&hidden, &layer.post_attn_norm);
        
        // 5. Feed-forward network (SwiGLU for most models)
        let ffn_out = ffn_forward(&normed, &layer.ffn_weights);
        
        // 6. Residual connection
        hidden.add_inplace(&ffn_out);
    }
    
    // Final norm + LM head (logits)
    let normed = rms_norm(&hidden, &model.final_norm);
    let logits = matmul(&model.lm_head, &normed);  // [vocab_size]
    
    // Sample next token
    sample_token(&logits, &sampler_config)
}
```

### 3.2 Memory Access Pattern During One Token

For Llama-3.1-8B Q4_K_M, one token generation involves:

| Phase | Memory Reads | Memory Writes | Notes |
|-------|-------------|---------------|-------|
| Embedding lookup | 8 KB | 8 KB | 4096 × FP16 |
| Per-layer (×32): RMSNorm | 8 KB | 8 KB | In-place |
| Per-layer: Q projection | 8 MB | 8 KB | 4096² × 4.625/8 |
| Per-layer: K projection | 1 MB | 32 B | GQA: 8 KV heads |
| Per-layer: V projection | 1 MB | 32 B | GQA: 8 KV heads |
| Per-layer: KV cache update | — | 64 B | Append K, V for current token |
| Per-layer: Attention | 512 KB | 8 KB | Read full K,V from cache |
| Per-layer: O projection | 8 MB | 8 KB | |
| Per-layer: FFN gate+up | 45 MB | 22 KB | 11008 × 4096 × 2 |
| Per-layer: FFN down | 22 MB | 8 KB | 4096 × 11008 |
| LM head | 0.5 MB | 300 KB | 128K vocab × 4096 |
| **TOTAL per token** | **~3.7 GB** | **~2 MB** | |

This confirms the memory-bandwidth-bound nature: ~3.7 GB reads per token.

---

## 4. Language Decision: Rust Ecosystem

### 4.1 Zero-Dependency Inference Engine Core

**The inference engine core is built from scratch. Infrastructure crates are allowed.**

| Component | Approach | Why |
|-----------|----------|-----|
| SIMD matmul kernels | **FROM SCRATCH** (core::arch) | The hot path — maximum control, cache-optimal |
| Quantization formats | **FROM SCRATCH** | Custom CQR-4 format optimized for streaming |
| KV cache manager | **FROM SCRATCH** | Novel streaming + eviction design |
| Weight memory manager | **FROM SCRATCH** | mmap streaming with madvise — no crate does this |
| Model execution loop | **FROM SCRATCH** | Layer-by-layer streaming design |
| Sampling / decoding | **FROM SCRATCH** | Tightly integrated with execution |
| BPE tokenizer | **FROM SCRATCH** or `tokenizers` crate | Either works, not a bottleneck |
| F16/BF16 types | **FROM SCRATCH** or `half` crate | Either works, simple type |
| Arena allocator | **FROM SCRATCH** (80 lines) | Trivial to build, full control |
| HTTP server | `axum` + `tokio` | Production-grade, not inference-related |
| JSON serialization | `serde` + `serde_json` | Standard, not inference-related |
| Config parsing | `toml` or `clap` | Standard, not inference-related |
| Logging | `tracing` | Standard, not inference-related |

**The principle:** Everything that touches model weights, KV cache, attention computation, or memory bandwidth is from scratch. Everything that's standard infrastructure uses the best available crate.

### 4.2 FFI Boundaries

**Call oneDNN from Rust?**

Option: Use `onednn-sys` (bindgen-generated bindings) to call oneDNN matmul kernels.

| Factor | FFI to oneDNN | Pure Rust kernels |
|--------|--------------|-------------------|
| Performance on Intel | ⭐⭐⭐⭐⭐ (optimal) | ⭐⭐⭐⭐ (close) |
| Performance on AMD | ⭐⭐⭐ (not AMD-optimized) | ⭐⭐⭐⭐ (AVX2-tuned) |
| Build complexity | High (C++ build, linker issues) | Low (pure Cargo) |
| Debuggability | Hard (C++ symbols, opaque) | Easy (Rust backtraces) |
| Portability | Linux x86_64 only | Cross-platform |
| Maintenance | External dep version tracking | Self-contained |

**Decision: Pure Rust kernels.** The marginal perf gain from oneDNN on Intel doesn't justify the build complexity and portability cost. Rust `core::arch` provides equivalent AVX2 throughput for the specific patterns we need (quantized matmul).

**Call OpenBLAS?** For FP32/FP16 matmul (embedding layer, LM head): `openblas-src` or `intel-mkl-src` could help. However, these are only needed for the small number of non-quantized operations. **Decision:** Use a simple Rust FP16 dot product for small matrices, avoid BLAS dependency.

---

## 5. Continuous Batching on CPU

### 5.1 What Continuous Batching Achieves (on GPU)

On GPU, continuous batching (in-flight batching, from Orca/vLLM) allows:
- Multiple requests to share the same batch at different decode stages
- New requests inserted into ongoing batch without waiting for current batch to complete
- Maximizes GPU utilization (fill idle SMs with concurrent work)

### 5.2 Why Continuous Battering Matters Less on CPU at 2 Threads

**Analysis for 2-vCPU:**

| Factor | GPU (continuous batching) | CPU 2-thread |
|--------|--------------------------|-------------|
| Parallelism available | 100+ SMs | 2 cores |
| Batch size sweet spot | 8–64 | 1–2 |
| Scheduling overhead | Amortized over 100s of threads | ~5-15% of total time |
| Memory overhead per request | KV cache only | KV cache + activation buffers |
| Implementation complexity | High | Still high, lower payoff |

**At batch size 1:** Single-threaded decode. No batching needed. Maximum per-request throughput.

**At batch size 2:** Two requests processed simultaneously.
- Memory: 2× KV cache (e.g., 2 × 512 MB = 1 GB for Llama-3.1-8B at context 4096)
- Compute: Split 2 threads → each thread handles one batch element
- Throughput: ~1.5× total throughput (not 2× due to shared bandwidth)
- Per-request latency: same as batch-1 (no benefit to requester)

**Verdict:** At 2 threads with 6 GB RAM, continuous batching is **not worth the complexity.** The memory overhead of maintaining multiple KV caches and the scheduling logic is not justified by the minimal throughput gain.

### 5.3 Recommended Approach: Micro-batch Size 1 with FIFO Queue

```
Request Queue (FIFO, priority-aware)
    │
    ▼
Take 1 request → Process fully (prefill → decode stream) → Return response
    │
    ▼ (if next request waiting)
Take next request → Process → Return
```

**Simplification:** One request at a time. Sequential processing. The API server queues requests and returns them as they complete. This eliminates:
- Batch tensor dimension management
- Per-request KV cache sizing complexity
- Scheduling fairness logic
- Attention masking for heterogeneous sequences

**When to add batching:** Only if profiling shows the API is frequently backlogged (multiple concurrent users exceeding sequential processing rate). At 3–4 tok/s per user and typical chat patterns (10–30 second thinking time between messages), sequential processing supports ~6-12 concurrent casual users.

---

## 6. KV Cache Management

### 6.1 Contiguous Allocation (Not Paged)

Based on Document 2's analysis, we reject PagedAttention for CPU:

```rust
pub struct KVCache {
    // Single contiguous allocation per model
    keys: Vec<f16>,     // Shape: [num_layers × max_context × n_kv_heads × head_dim]
    values: Vec<f16>,   // Same shape
    
    num_layers: usize,
    n_kv_heads: usize,
    head_dim: usize,
    max_context: usize,
    
    current_len: usize,  // Number of tokens in current context
}

impl KVCache {
    pub fn new(config: &ModelConfig, max_context: usize) -> Self {
        let per_layer_size = max_context * config.n_kv_heads * config.head_dim;
        let total = config.num_layers * per_layer_size;
        
        Self {
            keys: vec![0.0; total],
            values: vec![0.0; total],
            num_layers: config.num_layers,
            n_kv_heads: config.n_kv_heads,
            head_dim: config.head_dim,
            max_context,
            current_len: 0,
        }
    }
    
    /// Append K, V for the current token at the given layer
    pub fn append(&mut self, layer: usize, k: &[f16], v: &[f16]) {
        let offset = layer * self.max_context * self.n_kv_heads * self.head_dim
                   + self.current_len * self.n_kv_heads * self.head_dim;
        self.keys[offset..offset + k.len()].copy_from_slice(k);
        self.values[offset..offset + v.len()].copy_from_slice(v);
    }
}
```

**Advantages for CPU:**
- Sequential memory access during attention (stride-1 within a head)
- No page table lookups (direct pointer arithmetic)
- Prefetcher-friendly (hardware stride detection works on contiguous data)
- Simple implementation, no memory fragmentation

**Disadvantages:**
- Must pre-allocate for max_context (wastes memory if context is short)
- Cannot share KV between requests (no paging/sharing)
- Fixed maximum context (cannot grow beyond allocation)

### 6.2 Sliding Window Attention (Mistral-style)

For models like Gemma-2-9B with native sliding window attention:

```rust
impl KVCache {
    /// Get attention key slice for the sliding window
    pub fn get_keys_window(&self, layer: usize, window_size: usize) -> &[f16] {
        let start = if self.current_len > window_size {
            self.current_len - window_size
        } else {
            0
        };
        let offset = layer * self.max_context * self.n_kv_heads * self.head_dim;
        let start_offset = offset + start * self.n_kv_heads * self.head_dim;
        let end_offset = offset + self.current_len * self.n_kv_heads * self.head_dim;
        &self.keys[start_offset..end_offset]
    }
}
```

**Memory benefit:** With sliding window of 4096, the KV cache only needs to store the last 4096 tokens' K and V — regardless of total conversation length. This saves ~50% memory for long conversations.

**Circular buffer implementation for sliding window:**
```rust
impl KVCache {
    fn append_circular(&mut self, layer: usize, k: &[f16], v: &[f16]) {
        let write_pos = self.current_len % self.max_context;  // Circular write position
        let offset = layer * self.max_context * self.stride_per_token
                   + write_pos * self.stride_per_token;
        self.keys[offset..offset + k.len()].copy_from_slice(k);
        self.values[offset..offset + v.len()].copy_from_slice(v);
    }
}
```

### 6.3 KV Cache Eviction Policy

For multi-session support (future), eviction policies:

| Policy | Description | Overhead | Effectiveness |
|--------|-------------|----------|---------------|
| LRU (time-based) | Evict oldest session's KV | O(1) | Good for bursty traffic |
| LFU (token-count) | Evict session with fewest tokens | O(1) | Good for skewed usage |
| Context-length-based | Evict longest context (frees most memory) | O(1) | Emergency memory recovery |
| Attention-score-based | Evict tokens with lowest cumulative attention | O(n) per eviction | Best quality preservation |

**For single-session deployment (our primary case):** No eviction needed. Just pre-allocate for the session's expected max context.

**For multi-session (stretch goal):** LRU eviction. When memory pressure exceeds threshold, drop the oldest session's KV cache and reallocate.

---

## 7. Speculative Decoding on CPU

### 7.1 Concept

Speculative decoding uses a small "draft" model to generate K candidate tokens cheaply, then verifies them against the large "target" model in a single batched forward pass:

```
1. Draft model (0.5B) generates K tokens: [t1, t2, ..., tk]
2. Target model (9B) verifies all K tokens in ONE forward pass (batched)
3. Accept tokens until first rejection; resume from there
4. Net speedup: K × (draft_time / target_time) if acceptance rate high
```

**Speedup formula (Leviathan et al., arXiv:2211.17192):**
```
speedup = (1 - α^(K+1)) / ((1-α) × target_time_ratio)
where α = acceptance probability per token
```

### 7.2 CPU-Specific Analysis

**Draft model candidates (for 9B target):**
- Qwen2.5-0.5B at Q4_K_M: ~0.3 GB weights, ~30 tok/s decode on 2 vCPU [ESTIMATED]
- Phi-3.5-mini at Q4_K_M: ~2.2 GB weights, ~8 tok/s decode

**Timing analysis:**

For draft K=4 tokens with Qwen2.5-0.5B:
- Draft time: 4 × 33 ms = 132 ms (sequential on 1 thread)
- Target verification: 1 forward pass for 4 tokens (batched) = ~400 ms
  - NOT 4× target token time because: batched verification is faster than sequential decode (matrix × 4 vectors is more efficient than 4 × matrix × 1 vector on vectorizable ops)
  - Actually on CPU with 2 threads: batch-4 forward ≈ 3.5× single-token time = ~700 ms
  
Hmm. Let me reconsider. On CPU, batched forward pass for K tokens:
- Prefill-like: process K tokens through all layers simultaneously
- Memory reads: same weight reads (3.7 GB) regardless of batch size
- Compute: K× more MACs
- On memory-bound workload: batch-4 costs ~same time as batch-1 (memory BW dominates)

**Key insight for CPU:** Speculative decoding is MORE effective on CPU than GPU because:
1. The target model's forward pass is memory-bound — batch-1 and batch-4 take similar wall time
2. The draft model runs at ~30 tok/s (fast)
3. Acceptance rate: ~70-85% for aligned draft/target models (same architecture family)

**Estimated speedup:**
- Base decode: 4 tok/s
- With speculative (K=4, 80% acceptance): effective speedup ~1.5-2.5×
- **Result: 6-10 tok/s effective**

### 7.3 Self-Speculation (Medusa-like)

Medusa (arXiv:2401.10774) adds extra "heads" to the target model that predict multiple future tokens simultaneously, without a separate draft model:

**For our runtime:**
- Requires additional trained heads (not available for standard models)
- Memory overhead: extra parameters
- Not viable without pre-trained Medusa variants of our target models

**Verdict:** Skip Medusa. Use standard speculative decoding with a small draft model if throughput is below target.

### 7.4 When Speculative Decoding is Worth It

| Scenario | Worth It? |
|----------|-----------|
| Base decode ≥ 5 tok/s | ❌ Not needed |
| Base decode 2-4 tok/s | ✅ Try if draft model fits in memory |
| Memory headroom available (≥ 1 GB) | ✅ Can afford draft model |
| Tight memory (< 500 MB free) | ❌ No room for draft model |

**Recommendation:** Implement speculative decoding as an optional feature. Enable when:
1. Base throughput < 3 tok/s
2. A compatible draft model is available (same tokenizer)
3. Memory budget allows (target + draft < 6 GB)

---

## 8. Prompt Caching / Prefix Sharing

### 8.1 Concept

Multi-turn chat always reuses a system prompt. If we cache the KV entries for the system prompt, we avoid recomputing them on every turn.

```
Turn 1: System prompt (500 tokens) + User message (50 tokens)  → Compute KV for all 550 tokens
Turn 2: System prompt (500 tokens) + User msg 1 (50) + Asst reply (200) + User msg 2 (30)
        └─ KV for first 500 tokens cached ─┘  Only compute new 280 tokens
```

**Savings:** At 500-token system prompt, saves ~500 × prefill_time / layer = significant time for multi-turn.

### 8.2 Implementation for 6 GB RAM

**Challenge:** The KV cache for the system prompt (~500 tokens at 4 KV heads × 128 dim × FP16) takes:
```
2 × 32 × 500 × 8 × 128 × 2 = 32,768,000 bytes = ~31 MB
```

This is modest. We can maintain a separate "prefix KV cache" that persists across sessions.

**Design:**
```rust
pub struct PrefixCache {
    prefix_tokens: Vec<u32>,       // The cached prefix token IDs
    prefix_kv: Vec<f16>,           // KV cache for prefix (same layout as main KV)
    prefix_len: usize,             // Number of cached tokens
}

impl PrefixCache {
    /// Check if current input starts with cached prefix
    pub fn match_prefix(&self, input: &[u32]) -> usize {
        input.iter().zip(self.prefix_tokens.iter())
            .take_while(|(a, b)| a == b)
            .count()
    }
    
    /// Copy cached KV into the main KV cache
    pub fn restore_to(&self, layer: usize, kv_cache: &mut KVCache) {
        // Copy prefix_kv for this layer into kv_cache
    }
}
```

**Memory cost:** ~31 MB for a 500-token system prompt (Llama-3.1-8B). Negligible in 6 GB budget.

**Recommendation:** Implement prefix caching for the system prompt. This is a simple, high-value optimization for multi-turn chat use cases.

---

## 9. Tokenizer (Custom Built)

### 9.1 Custom BPE Tokenizer

We build our own BPE (Byte-Pair Encoding) tokenizer rather than depending on the `tokenizers` crate. This is straightforward — BPE is a well-understood algorithm (~500 lines of Rust).

**Design:**
```rust
pub struct BpeTokenizer {
    vocab: HashMap<Vec<u8>, u32>,      // byte sequence → token ID
    merges: HashMap<(u32, u32), u32>,  // (token_a, token_b) → merged_token
    id_to_bytes: Vec<Vec<u8>>,         // token ID → byte sequence
    special_tokens: HashMap<String, u32>,
}

impl BpeTokenizer {
    /// Load from tokenizer.json (HuggingFace format) — parsed by our custom JSON parser
    pub fn load(tokenizer_json: &[u8]) -> Self { ... }

    /// Encode text → token IDs
    pub fn encode(&self, text: &str) -> Vec<u32> {
        // 1. UTF-8 → bytes
        // 2. Apply byte-level pre-tokenization (split on whitespace/punctuation)
        // 3. For each pre-token: greedy BPE merge using merges map
        // 4. Return concatenated token IDs
    }

    /// Decode token IDs → text (for streaming output)
    pub fn decode(&self, ids: &[u32]) -> String { ... }

    /// Decode single token incrementally (for SSE streaming)
    pub fn decode_incremental(&mut self, token_id: u32) -> &str { ... }
}
```

**Vocabulary loading:** We parse HuggingFace `tokenizer.json` files using our custom JSON parser. No Python dependency, no pickle, no unsafe deserialization of external formats.

### 9.2 Performance

| Operation | Custom BPE | HuggingFace `tokenizers` crate |
|-----------|-----------|-------------------------------|
| Encode 500 tokens | ~5–8 ms | ~5 ms |
| Decode 1 token | <1 μs | <1 μs |
| Vocab load time | ~50 ms | ~20 ms (with C parsing) |

**At 2 vCPU:** Tokenization is never the bottleneck even with a custom implementation. Encoding a 500-token prompt takes ~5 ms against ~1 second for prefill compute.

---

## 10. HTTP API Server Design

### 10.1 OpenAI-Compatible API

**Endpoints:**
```
POST /v1/chat/completions      # Chat completion (main endpoint)
POST /v1/completions           # Text completion  
GET  /v1/models                # List available models
GET  /health                   # Health check
GET  /v1/memory                # Memory usage stats
```

**Chat Completions Request:**
```json
{
  "model": "llama-3.1-8b-q4km",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "temperature": 0.7,
  "max_tokens": 512,
  "stream": true
}
```

### 10.2 Streaming Output via SSE

```rust
use axum::{
    response::sse::{Event, Sse},
    extract::State, Json, Router,
};
use tokio_stream::wrappers::ReceiverStream;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default = "default_max_tokens")]
    max_tokens: usize,
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct ChatChunk {
    id: String,
    object: &'static str,
    choices: Vec<ChunkChoice>,
}

async fn chat_completions_stream(
    State(runtime): State<Arc<Runtime>>,
    Json(request): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(32);
    
    // Spawn inference on the dedicated compute thread pool (not tokio workers)
    runtime.submit_inference(move |engine| {
        let mut generator = engine.create_generator(&request);
        while let Some(token) = generator.next_token() {
            let chunk = ChatChunk::from_token(token);
            let json = serde_json::to_string(&chunk).unwrap();
            if tx.blocking_send(Ok(Event::default().data(json))).is_err() {
                break;  // Client disconnected
            }
        }
        let _ = tx.blocking_send(Ok(Event::default().data("[DONE]")));
    });
    
    Sse::new(ReceiverStream::new(rx))
}

fn build_router(runtime: Arc<Runtime>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/health", get(health_check))
        .with_state(runtime)
}
```

**Key design choice:** Inference runs on a dedicated thread pool (`spawn_blocking` or custom), completely isolated from tokio's async worker threads. This ensures the compute-heavy forward pass never blocks HTTP connections or other concurrent requests.

### 10.3 Backpressure Handling

When the CPU can't keep up (e.g., long prompt prefill + slow decode):
1. **Queue depth limit:** Reject requests if queue > N (return 429 Too Many Requests)
2. **Timeout:** Cancel inference if single request takes > T seconds
3. **Token streaming:** Send tokens as soon as they're generated (no buffering)
4. **Graceful degradation:** If memory pressure detected, reduce max_context for new requests

---

## 11. Configuration and Startup

### 11.1 Configuration File (TOML)

```toml
[model]
path = "/models/llama-3.1-8b-instruct.Q4_K_M.gguf"
max_context = 4096
kv_cache_precision = "fp16"  # or "int8"

[runtime]
threads = 0  # 0 = auto-detect
memory_budget_mb = 5900  # Leave 100MB for OS

[server]
host = "0.0.0.0"
port = 8080
max_queue_size = 4
request_timeout_secs = 300

[sampling]
default_temperature = 0.7
default_top_p = 0.9
repetition_penalty = 1.1

[optimization]
streaming_weights = true   # Use madvise hints
prefix_cache = true        # Cache system prompt KV
speculative_decoding = false
speculative_draft_model = ""
```

### 11.2 Startup Sequence

```rust
async fn main() {
    // 1. Parse config
    let config = Config::load("config.toml");
    
    // 2. Detect CPU topology
    let topology = detect_topology();  // Physical cores, SIMD capabilities
    let num_threads = if config.threads == 0 {
        topology.physical_cores.min(2)
    } else {
        config.threads
    };
    
    // 3. Load model (mmap, zero-copy)
    let model = Model::load_mmap(&config.model.path)?;
    
    // 4. Pre-allocate KV cache
    let kv_cache = KVCache::new(&model.config, config.model.max_context);
    
    // 5. Pre-allocate activation arena
    let arena = BumpAllocator::new(256 * MB);
    
    // 6. Initialize tokenizer
    let tokenizer = Tokenizer::from_file(&config.model.tokenizer_path)?;
    
    // 7. Start API server
    let runtime = Runtime::new(model, kv_cache, arena, tokenizer, num_threads);
    start_server(runtime, &config.server).await;
}
```

**Startup time:** <1 second (mmap is instantaneous, pre-allocation is fast).

---

## 12. Implementation Implications

### 12.1 Architecture Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Batching | Batch size 1 (sequential) | Memory + thread constraints |
| KV cache layout | Contiguous, pre-allocated | CPU cache-friendly |
| KV cache precision | FP16 (default), INT8 (optional) | Quality vs memory tradeoff |
| Sliding window | Yes (for applicable models) | Bounds KV cache growth |
| Prompt caching | Yes (system prompt KV) | Multi-turn chat optimization |
| Speculative decoding | Optional, draft model based | Enable if throughput < 3 tok/s |
| API framework | axum + tokio | Production-grade, async |
| Thread model | 1-2 based on toplogy detection | Avoid hyperthreading contention |
| Memory allocator | Bump arena for activations | No fragmentation, O(1) alloc/free |

### 12.2 Development Phases

| Phase | Scope | Duration |
|-------|-------|----------|
| Phase 1 | GGUF loader + scalar forward pass + sampling | 2 weeks |
| Phase 2 | AVX2 kernels (Q4_K_M matmul, attention) | 3 weeks |
| Phase 3 | mmap streaming + KV cache + memory manager | 2 weeks |
| Phase 4 | API server (axum) + streaming SSE | 1 week |
| Phase 5 | Multi-model support + prefix caching | 2 weeks |
| Phase 6 | Speculative decoding + AVX-512 kernels | 2 weeks |

**Total: ~12 weeks** for a single engineer to reach production-ready MVP.

---

*Next: Document 6 covers target model architectures in detail — Qwen2.5, Gemma-2, Llama-3.1, Phi, and BitNet models with compatibility matrices.*
