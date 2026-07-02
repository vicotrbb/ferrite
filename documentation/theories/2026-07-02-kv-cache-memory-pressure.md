# Theory: KV Cache Memory Pressure

Date: 2026-07-02

Status: Hypothesis

## Hypothesis

Ferrite's current long-chat RSS profile is acceptable for one request at a time,
but KV cache allocation and retention policy will become a limiting factor for
6Gi-fit targets, longer steady-state serving, and future multi-client queues.
Explicit KV cache accounting, reservation, and eviction should improve memory
predictability before larger model tiers.

## Mechanism

The x86_64 Qwen2.5-1.5B Q8_0 generated-context proofs ran with 8Gi pod limits,
not because the final 1024 run exceeded 6Gi, but because previous Q8_0 long-chat
work came close to the earlier 6Gi cap. The latest 1024 generated-context run
observed:

- server RSS after model load: `1875744` KiB;
- pod cgroup memory current after health: `2028441600` bytes;
- pod cgroup memory peak after build, model load, and proof: `3928395776`
  bytes;
- turn-level RSS before/after values from roughly `1987674112` to
  `2060681216` bytes.

This single-run peak is below 6Gi, but it does not prove steady-state memory
behavior, multi-session safety, or failure behavior under memory pressure. The
system needs a direct way to measure and bound KV cache bytes per request,
queued request, and idle server state.

## Expected Measurement

This theory is worth pursuing if a memory-focused probe can attribute RSS or
cgroup growth to model weights, active K/V cache, request buffers, and retained
server state with enough precision to set safe limits.

The first useful result would show:

- predicted K/V bytes for each model/configuration;
- observed RSS and cgroup current before health, after health, per request,
  after idle, and after server shutdown;
- behavior under repeated sessions without restarting the process;
- explicit failure behavior when a configured KV budget would be exceeded.

## Falsification Experiment

Run repeated 1024-token generated-context sessions in one server process with
RSS/cgroup sampling before and after every session. Compare:

- one session;
- three sequential sessions;
- five sequential sessions;
- same server after a fixed idle period.

The theory is falsified for the current milestone if repeated sessions show no
meaningful retained growth, cgroup current returns to the same idle baseline,
and existing memory accounting already predicts observed growth closely enough
to set a safe 6Gi operating policy.

## Risks

- RSS and cgroup current include allocator behavior, page cache, and runtime
  effects that are not direct KV-cache bytes.
- Per-request memory accounting can become misleading if it ignores allocator
  reuse.
- Aggressive eviction may reduce latency gains from prefix reuse.
- Memory limits that are safe for Qwen2.5-1.5B may not transfer to SmolLM2 or
  future 3B-4B tiers.

## Next Step

Add a memory-focused benchmark note from existing long-chat proof data, then
run a repeated-session probe in a bounded pod. If retained memory grows or
cannot be attributed, add explicit KV/cache byte accounting before any prefix
reuse implementation.
