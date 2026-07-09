# Dev note: deterministic live disconnect permit test

- Date: 2026-07-09
- Scope: test-only HTTP stream lifecycle proof

## Problem

The release workspace suite intermittently failed
`live_http_server_releases_inference_permit_after_tcp_disconnect_before_generated_content`.
The TCP helper reads up to 1024 bytes at a time and closes the connection as
soon as the initial assistant-role marker is present. HTTP headers, the role
event, and one or more generated events may legally be coalesced into that
single read, so asserting that the returned buffer contains no generated event
made the test depend on packet and scheduler timing.

## Fix

The test is now named for the event it actually controls:
`live_http_server_releases_inference_permit_after_initial_role_chunk_disconnect`.
It still aborts immediately when the role marker becomes observable and proves
that the server releases its inference permit. The impossible negative
assertion about other bytes already coalesced into the same TCP read was
removed. Product code and stream behavior are unchanged.

## Validation

The complete release `openai_http` integration suite passed three consecutive
serial runs after the change. Before the change, the target test passed alone
but failed when run in the suite, reproducing the timing dependency.
