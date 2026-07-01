# OpenAI SmolLM EOS Stop Proof

## Scope

This run proves tokenizer EOS termination through Ferrite's OpenAI-compatible
HTTP surface for SmolLM2 1.7B Q4_K_M. It uses the known EOS-sensitive prompt
from the CLI proof, `The capital of France is`, and verifies that natural EOS
returns OpenAI `finish_reason: "stop"` before the requested token budget is
exhausted.

This is EOS-specific evidence for one local Tier 1 model. It does not prove EOS
behavior across all required Tier 1 models, x86_64, or longer steady-state
serving.

## Environment

- Date: 2026-06-30
- Commit: `3d1dce7`
- Host: local macOS development machine
- Server port: `127.0.0.1:18104`
- Server PID: `66453`
- Model: `SmolLM2-1.7B-Instruct-Q4_K_M`
- Model path: `target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf`
- API key: `local-secret`
- Raw log: `target/proof/smollm-openai-eos-probe.log`

## Server Command

```sh
target/release/ferrite-server \
  --bind 127.0.0.1:18104 \
  --model target/models/SmolLM2-1.7B-Instruct-Q4_K_M.gguf \
  --model-id SmolLM2-1.7B-Instruct-Q4_K_M \
  --api-key local-secret \
  --default-max-tokens 16 \
  --hard-max-tokens 64
```

Health check response:

```json
{"status":"ok","ready":true,"model":"SmolLM2-1.7B-Instruct-Q4_K_M"}
```

## Probe Command

```sh
python3 - <<'PY' > target/proof/smollm-openai-eos-probe.log 2>&1
import json, urllib.request

def post(path, body):
    req = urllib.request.Request(
        f'http://127.0.0.1:18104{path}',
        data=json.dumps(body).encode(),
        headers={'Authorization':'Bearer local-secret','Content-Type':'application/json'},
    )
    with urllib.request.urlopen(req, timeout=180) as resp:
        print(f'HTTP {resp.status} {path} content-type={resp.headers.get("content-type")}')
        print(resp.read().decode())

post('/v1/completions', {
    'model': 'SmolLM2-1.7B-Instruct-Q4_K_M',
    'prompt': 'The capital of France is',
    'max_tokens': 6,
    'stream': True,
    'stream_options': {'include_usage': True},
})
post('/v1/completions', {
    'model': 'SmolLM2-1.7B-Instruct-Q4_K_M',
    'prompt': 'The capital of France is',
    'max_tokens': 6,
    'stream': False,
})
post('/v1/chat/completions', {
    'model': 'SmolLM2-1.7B-Instruct-Q4_K_M',
    'messages': [{'role': 'user', 'content': 'The capital of France is'}],
    'max_tokens': 16,
    'stream': True,
    'stream_options': {'include_usage': True},
})
PY
```

## Results

Streaming legacy completions returned HTTP `200` with `text/event-stream` and
emitted:

```text
data: ... "text":" Paris" ... "finish_reason":null
data: ... "text":"." ... "finish_reason":null
data: ... "text":"<|im_end|>" ... "finish_reason":null
data: ... "text":"" ... "finish_reason":"stop"
data: ... "usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8,...}
data: [DONE]
```

The non-streaming legacy completion returned:

```json
{"choices":[{"text":" Paris.<|im_end|>","finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8}}
```

Streaming chat completions returned HTTP `200` with `text/event-stream` and
emitted generated content through the tokenizer EOS marker:

```text
data: ... "delta":{"content":" Paris"} ... "finish_reason":null
data: ... "delta":{"content":"."} ... "finish_reason":null
data: ... "delta":{"content":"<|im_end|>"} ... "finish_reason":null
data: ... "delta":{} ... "finish_reason":"stop"
data: ... "usage":{"prompt_tokens":12,"completion_tokens":9,"total_tokens":21,...}
data: [DONE]
```

After stopping the server, `lsof -nP -iTCP:18104 -sTCP:LISTEN` returned no
listener.

## Interpretation

Ferrite's OpenAI-compatible legacy completion and chat streaming surfaces can
terminate on tokenizer EOS and report OpenAI `finish_reason: "stop"` without an
explicit `stop` request parameter. Usage accounting confirms generation ended
before the requested token budgets:

- legacy completions: 3 completion tokens for `max_tokens: 6`;
- streaming chat completions: 9 completion tokens for `max_tokens: 16`.

The EOS token is still emitted as visible `<|im_end|>` content today. That
matches the current runtime behavior already documented by the CLI EOS proof,
but future API polish may choose to suppress tokenizer control text at the HTTP
boundary.
