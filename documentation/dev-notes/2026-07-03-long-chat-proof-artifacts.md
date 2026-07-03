# Long Chat Proof Artifacts

Date: 2026-07-03 UTC, 2026-07-02 local time

## Change

Added durable proof artifact options to `ferrite-openai-long-chat-gate`:

```text
--proof-log PATH
--proof-exit-code PATH
```

When configured, the gate mirrors every machine-readable stdout proof line into
the proof log and writes the final process result to the exit-code file. Parent
directories are created automatically.

This is intentionally scoped to the proof harness. It does not change Ferrite's
OpenAI-compatible HTTP API or inference runtime.

## Why

A real long-chat proof lost its Kubernetes exec stream with:

```text
read tcp ...: read: connection reset by peer
```

The gate process survived inside the pod, but the client-side output stream was
lost. For long 256/512/1024-token gates, the proof artifact must survive the
transport used to launch or observe it.

## Validation

Red test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate proof_artifacts -- --nocapture
error[E0432]: unresolved import `ferrite_server::long_chat_gate::LongChatProofArtifacts`
error[E0599]: no method named `proof_log_path` found for struct `LongChatGateConfig`
error[E0599]: no method named `proof_exit_code_path` found for struct `LongChatGateConfig`
```

Green test evidence:

```text
cargo test -p ferrite-server --test long_chat_gate proof_artifacts -- --nocapture
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 47 filtered out

cargo test -p ferrite-server --test long_chat_gate formats_long_chat_gate_plan_with_proof_artifact_paths -- --nocapture
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 47 filtered out

cargo test -p ferrite-server --test long_chat_gate parses_custom_long_chat_token_lengths_turns_and_models -- --nocapture
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 47 filtered out

cargo test -p ferrite-server --test long_chat_gate -- --nocapture
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Binary smoke:

```text
cargo run -p ferrite-server --bin ferrite-openai-long-chat-gate -- \
  --models fixture-model \
  --token-lengths 256 \
  --turns 4 \
  --proof-log target/proof/long-chat-artifact-smoke.log \
  --proof-exit-code target/proof/long-chat-artifact-smoke.exit
```

The command exited `0`. The proof log contained the same plan and scenario lines
as stdout, and the exit-code file contained:

```text
0
```

## Next Proof Use

Run the next long-chat proof inside the homelab pod with the gate launched in
the background and these artifact paths set. Poll the files from separate
short-lived `kubectl exec` calls instead of relying on one long-lived exec
stream.
