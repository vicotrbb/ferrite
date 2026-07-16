# ADR 0020: Rust 2024 and bounded test harnesses

Date: 2026-07-16

Status: Accepted

## Context

Ferrite pins Rust 1.96 and no longer needs the Rust 2021 edition for compiler
compatibility. A full `rust-2024-compatibility` audit found destructor-order
warnings in the continuous scheduler and OpenAI routes. The affected values
included channel permits, channel senders, scheduled jobs, and mapped Locus KV
sessions, so the migration required explicit review rather than lint
suppression.

The server package also exposed 43 Cargo targets, including 39 integration-test
binaries. Most of those binaries were real-model cases that are ignored when a
local GGUF artifact is absent. Cargo still compiled and linked the shared server
and test support for every binary during routine all-target validation.

## Decision

The workspace uses the Rust 2024 edition and Cargo resolver 3. Routing branches
that obtain an optional scheduler use `match`, retaining the prior sender
lifetime. Scheduler receive, reservation, admission, and activation results are
terminated explicitly so destructor timing is visible at review.

The server disables automatic integration-test discovery and declares seven
test harnesses. Fixture-backed client cases share `openai_clients`. All
artifact-gated client and HTTP cases share `real_models`, with one module per
model and behavior. Help, long-chat, fixture HTTP, throughput, and tool-metadata
tests remain separate because they exercise distinct executable or operational
surfaces.

## Consequences

Resolver 3 prefers dependency versions compatible with the declared MSRV.
Rustfmt uses the canonical Rust 2024 style. The scheduler and routes preserve
the reviewed resource-release contract while gaining the current edition.

The server now has 11 Cargo targets, seven of which are integration-test
harnesses. All prior test functions remain present. Routine checks reuse one
compiled server and support module for the real-model inventory, reducing
repeated compiler and linker work. A focused real-model gate now selects a
module inside `--test real_models` instead of naming a standalone test target.

This is build and repository overhead reduction. It does not claim inference
throughput, latency, memory, or numerical improvement.

## Alternatives Considered

- Remain on Rust 2021. Rejected because the pinned compiler fully supports Rust
  2024 and resolver 3, and the compatibility audit provided a bounded migration
  surface.
- Suppress the migration lints. Rejected because channel and mapped-session
  destructors participate in cancellation and resource-release behavior.
- Delete ignored real-model tests. Rejected because those tests are required
  compatibility and release evidence.
- Put every server test in one binary. Rejected because help, long-chat, HTTP,
  and throughput suites have distinct dependencies and operational intent.

## Evidence

- `RUSTFLAGS="-Dwarnings -Wrust-2024-compatibility" cargo check --workspace
  --all-targets --all-features --locked` identified the reviewed scope changes.
- `Cargo.toml` selects edition 2024 and resolver 3.
- `crates/ferrite-server/Cargo.toml`, `tests/openai_clients.rs`, and
  `tests/real_models.rs` define the bounded harness surface.
- `cargo metadata --no-deps --format-version 1` reports 11 server targets after
  the change, down from 43 before it.
- Default and all-feature validation, strict Clippy, rustdoc, and the eval
  harness remain required before closeout.
