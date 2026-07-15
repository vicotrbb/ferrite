# Phi-3 Vec versus Locus KV soak diagnostic, 2026-07-14

Status: diagnostic only. This artifact does not promote a performance or
steady-state memory claim.

The clean-host acceptance child
`2026-07-14-092611-qwen2.5-1.5b-instruct-q8_0-multi.json` first found the
failure. Qwen2.5 1.5B Q8 and SmolLM2 1.7B Q4 passed exact route parity and both
16 MiB soak gates. Phi-3 3.8B Q4 also preserved exact default-versus-batched
tokens, but its continuous-batched Vec route retained 394,346,496 bytes and
failed the memory gate.

## Focused method

Both focused servers used the exact Phi-3 artifact with SHA-256
`8a83c7fb9049a9b2e92266fa7ad04933bb53aa1e85136b7b30f1b8000ff2edef`,
four requests at concurrency four, a 64-token budget, automatic kernels, and
the experimental four-stream scheduler. RSS was sampled after two idle
seconds. The Vec server used the existing default release binary. The Locus
server was built from the same Rust source with `--features locus-kv`, then
configured for 16-token blocks and a 128-token per-session capacity.

Every one of the twelve focused cohorts reproduced the same exact 64-token
trace. The Vec idle samples were:

```text
2679373824 2566950912 2548940800 2679242752 2671280128 2622619648
```

Their full range was 130,433,024 bytes and the last-three range was 56,623,104
bytes. The series was non-monotonic and ended 56,754,176 bytes below its first
sample, so it did not demonstrate a per-round leak. It did demonstrate an RSS
envelope too wide for the fixed 16 MiB acceptance gate.

The Locus idle samples were:

```text
2401894400 2400550912 2402222080 2401697792 2402123776 2401026048
```

Their complete six-round range was 1,671,168 bytes and the last-three range was
1,097,728 bytes. The final sample was 868,352 bytes below the first. This is a
strong bounded-backend hypothesis, not accepted evidence, because the focused
runs were not clean-host screened and did not exercise both server routes.

A later normal Locus suite child tested the shorter 500 ms idle boundary. Its
Phi batched samples were 2,401,320,960, 2,381,316,096, and 2,795,732,992 bytes.
The 414,416,896-byte tail range failed the same 16 MiB gate while preserving
exact route parity. OrbStack also contaminated postflight at 56.8 percent, but
the child had already failed its internal RSS gate. This separates backend
capacity from the time at which macOS reports reclaimed resident pages. A
two-second boundary still requires a normal clean repeated suite before any
claim can be promoted.

Both diagnostic servers and all throughput clients were stopped before this
artifact was written. The next gate is a normal three-repetition clean-host
`eval_suite.py` run with the exact Locus build and runtime configuration.
