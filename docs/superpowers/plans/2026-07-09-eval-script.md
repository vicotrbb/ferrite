# Ferrite Evaluation Script Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A one-command evaluation harness (`scripts/eval.py`) that measures Ferrite's tokens/second, TTFT, memory, and CPU via the CLI and the OpenAI server, writing JSON + Markdown records to `scripts/evals/`.

**Architecture:** A single stdlib-only Python 3 script orchestrates Ferrite's already-instrumented Rust binaries (`ferrite`, `ferrite-server`, `ferrite-openai-throughput`). Python's only measurements are wall-clock timestamps on stdout lines and a `ps`-based RSS/cputime sampler thread; everything else is parsed from the binaries' `key=value` output. Pure computation functions are unit-tested in `scripts/eval_test.py`; subprocess orchestration is verified end-to-end in the final task.

**Tech Stack:** Python 3 stdlib (`subprocess`, `threading`, `urllib.request`, `argparse`, `json`), `ps`, cargo release builds.

**Spec:** `docs/superpowers/specs/2026-07-09-eval-script-design.md`

## Global Constraints

- Python stdlib only — no pip installs, no third-party imports.
- All inference measurement stays in Ferrite's Rust binaries; Python takes only line timestamps and `ps` samples.
- Output records go to `scripts/evals/<YYYY-MM-DD-HHMMSS>-<model-stem>.json` + `.md`, `schema_version: 1`, and must include the exact command lines run.
- Auto-download offers only Qwen2.5-0.5B-Instruct-Q4_K_M (~400 MB); bigger models are placed manually.
- Exit code 2 when no model is available and download is declined/forbidden.
- The server child process must always be terminated, including on exceptions (finally block).
- Works on macOS (primary); Linux fallbacks for env capture are best-effort guarded.

## Reference: instrumentation this plan consumes

- `target/release/ferrite` prints `key=value` lines. Relevant: `sleep_after_load_ms=` (printed + flushed *before* the post-load sleep), `prompt_token_ids=` (first line printed *after* prefill), `stream_token_id=`/`stream_text=` per generated token, `benchmark_total_ns=`/`benchmark_avg_ns=` (in-process decode timing), `model_file_bytes=`, `scalar_weight_bytes=`, `kv_cache_bytes=`, `generated_stopped_on_eos=`. Rust stdout is line-buffered even when piped, so line timestamps are honest.
- `target/release/ferrite-server` flags: `--model`, `--model-id`, `--bind`, `--api-key`, `--default-max-tokens`, `--hard-max-tokens` (must be ≥ default), `--inference-wait-ms`. `GET /health` returns 200 when ready.
- `target/release/ferrite-openai-throughput` (built by `-p ferrite-server`) flags: `--addr host:port`, `--model <id>`, `--endpoint chat-completions`, `--prompt`, `--requests N`, `--max-tokens N`, `--stream`, `--stream-usage`, `--api-key`. Prints `key=value` metrics including `streaming_time_to_first_token_ms`, `streaming_tokens_per_second`, `streaming_token_latency_p50_ms`/`p95_ms`, `requests_per_second`, `streaming_usage_*`.
- `ps -o rss=,cputime= -p PID` works on macOS and Linux; `rss` is KiB, `cputime` is `[DD-]HH:MM:SS.ss` or `MM:SS.ss`. CPU% is computed from cputime deltas (true utilization), not `ps -o %cpu` (decaying average).

---

### Task 1: Scaffold + pure parsers (`parse_kv_lines`, `percentile`, `parse_cputime`)

**Files:**
- Create: `scripts/eval.py`
- Create: `scripts/eval_test.py`

**Interfaces:**
- Consumes: nothing.
- Produces: `parse_kv_lines(lines: Iterable[str]) -> dict[str, str]` (last occurrence wins, non-kv lines skipped); `percentile(values: list[float], pct: float) -> float` (nearest-rank, raises `ValueError` on empty); `parse_cputime(text: str) -> float` seconds. Module constants `SCHEMA_VERSION`, `REPO_ROOT`, `MODELS_DIR`, `EVALS_DIR`, `DEFAULT_PROMPT`, `DEFAULT_MODEL_URL`, `API_KEY`, and namedtuple `Sample`.

- [ ] **Step 1: Write the failing tests**

Create `scripts/eval_test.py`:

```python
#!/usr/bin/env python3
"""Unit tests for the pure logic in scripts/eval.py (stdlib unittest)."""

import importlib
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
ev = importlib.import_module("eval")


class ParseKvLinesTest(unittest.TestCase):
    def test_last_occurrence_wins_and_noise_is_skipped(self):
        lines = [
            "next_token_id=42",
            "stream_text=hello world",
            "stream_text=again",
            "not a kv line",
            "benchmark_avg_ns=1000000",
        ]
        kv = ev.parse_kv_lines(lines)
        self.assertEqual(kv["next_token_id"], "42")
        self.assertEqual(kv["stream_text"], "again")
        self.assertEqual(kv["benchmark_avg_ns"], "1000000")
        self.assertEqual(len(kv), 3)

    def test_value_may_contain_equals(self):
        kv = ev.parse_kv_lines(["generated_text=a=b"])
        self.assertEqual(kv["generated_text"], "a=b")


class PercentileTest(unittest.TestCase):
    def test_nearest_rank(self):
        values = [40.0, 10.0, 30.0, 20.0]
        self.assertEqual(ev.percentile(values, 50), 20.0)
        self.assertEqual(ev.percentile(values, 95), 40.0)
        self.assertEqual(ev.percentile([5.0], 50), 5.0)

    def test_empty_raises(self):
        with self.assertRaises(ValueError):
            ev.percentile([], 50)


class ParseCputimeTest(unittest.TestCase):
    def test_minutes_seconds(self):
        self.assertAlmostEqual(ev.parse_cputime("0:01.25"), 1.25)
        self.assertAlmostEqual(ev.parse_cputime("2:03.50"), 123.5)

    def test_hours_and_days(self):
        self.assertAlmostEqual(ev.parse_cputime("1:02:03"), 3723.0)
        self.assertAlmostEqual(ev.parse_cputime("1-01:02:03"), 86400 + 3723.0)


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 scripts/eval_test.py`
Expected: FAIL with `ModuleNotFoundError: No module named 'eval'`

- [ ] **Step 3: Write the scaffold and parsers**

Create `scripts/eval.py`:

```python
#!/usr/bin/env python3
"""Ferrite evaluation harness.

Measures tokens/second, time-to-first-token, memory, and CPU for the
ferrite CLI and the OpenAI-compatible server, then writes a JSON record
and a Markdown report into scripts/evals/.

Stdlib only. All inference measurement happens inside Ferrite's
instrumented binaries; this script orchestrates, timestamps stdout
lines, samples `ps`, and aggregates.
"""

import argparse
import json
import math
import os
import platform
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
from collections import namedtuple
from datetime import datetime, timezone
from pathlib import Path

SCHEMA_VERSION = 1
REPO_ROOT = Path(__file__).resolve().parent.parent
MODELS_DIR = REPO_ROOT / "target" / "models"
EVALS_DIR = REPO_ROOT / "scripts" / "evals"
DEFAULT_PROMPT = "Write a short story about a rusty robot who learns to sail."
DEFAULT_MODEL_URL = (
    "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/"
    "qwen2.5-0.5b-instruct-q4_k_m.gguf"
)
API_KEY = "ferrite-eval"

Sample = namedtuple("Sample", ["t", "rss_bytes", "cpu_seconds"])


def parse_kv_lines(lines):
    """Parse ferrite's key=value stdout lines; last occurrence wins."""
    result = {}
    for line in lines:
        key, sep, value = line.partition("=")
        if sep and key and " " not in key:
            result[key] = value
    return result


def percentile(values, pct):
    """Nearest-rank percentile; values need not be sorted."""
    if not values:
        raise ValueError("percentile of empty list")
    ordered = sorted(values)
    rank = max(1, math.ceil(pct / 100 * len(ordered)))
    return ordered[rank - 1]


def parse_cputime(text):
    """Parse ps cputime ([DD-]HH:MM:SS.ss or MM:SS.ss) into seconds."""
    text = text.strip()
    days = 0.0
    if "-" in text:
        day_part, text = text.split("-", 1)
        days = float(day_part)
    seconds = 0.0
    for part in text.split(":"):
        seconds = seconds * 60 + float(part)
    return days * 86400 + seconds
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python3 scripts/eval_test.py`
Expected: `OK` (6 tests)

- [ ] **Step 5: Make the script executable and commit**

```bash
chmod +x scripts/eval.py
git add scripts/eval.py scripts/eval_test.py
git commit -m "feat(eval): scaffold eval harness with kv/percentile/cputime parsers"
```

---

### Task 2: Process sampler + window aggregation

**Files:**
- Modify: `scripts/eval.py` (append after `parse_cputime`)
- Modify: `scripts/eval_test.py` (append test class)

**Interfaces:**
- Consumes: `parse_cputime`, `Sample` from Task 1.
- Produces: `ProcessSampler(pid, interval_s=0.1)` — a daemon `threading.Thread`; `.start()`, `.stop() -> list[Sample]`, `.samples: list[Sample]` (timestamps are `time.monotonic()`); `aggregate_samples(samples, t_start=None, t_end=None) -> dict` with keys `sample_count`, and when samples exist `rss_peak_bytes`, `rss_mean_bytes`, plus `cpu_mean_percent`/`cpu_peak_percent` when ≥2 samples span time (from cputime deltas, rounded to 0.1).

- [ ] **Step 1: Write the failing test**

Append to `scripts/eval_test.py` (before the `__main__` block):

```python
class AggregateSamplesTest(unittest.TestCase):
    SAMPLES = [
        ev.Sample(t=0.0, rss_bytes=100 << 20, cpu_seconds=0.0),
        ev.Sample(t=1.0, rss_bytes=200 << 20, cpu_seconds=0.5),
        ev.Sample(t=2.0, rss_bytes=150 << 20, cpu_seconds=1.5),
    ]

    def test_whole_run_aggregation(self):
        agg = ev.aggregate_samples(self.SAMPLES)
        self.assertEqual(agg["sample_count"], 3)
        self.assertEqual(agg["rss_peak_bytes"], 200 << 20)
        self.assertEqual(agg["rss_mean_bytes"], (450 << 20) // 3)
        self.assertAlmostEqual(agg["cpu_mean_percent"], 75.0)
        self.assertAlmostEqual(agg["cpu_peak_percent"], 100.0)

    def test_windowed_aggregation(self):
        agg = ev.aggregate_samples(self.SAMPLES, t_start=1.0, t_end=2.0)
        self.assertEqual(agg["sample_count"], 2)
        self.assertAlmostEqual(agg["cpu_mean_percent"], 100.0)

    def test_empty_window(self):
        agg = ev.aggregate_samples(self.SAMPLES, t_start=5.0)
        self.assertEqual(agg, {"sample_count": 0})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 scripts/eval_test.py AggregateSamplesTest -v`
Expected: FAIL/ERROR with `AttributeError: module 'eval' has no attribute 'aggregate_samples'`

- [ ] **Step 3: Implement sampler and aggregation**

Append to `scripts/eval.py`:

```python
class ProcessSampler(threading.Thread):
    """Samples RSS and accumulated CPU time of a pid via ps every interval."""

    def __init__(self, pid, interval_s=0.1):
        super().__init__(daemon=True)
        self.pid = pid
        self.interval_s = interval_s
        self.samples = []
        self._stop_event = threading.Event()

    def run(self):
        while not self._stop_event.is_set():
            proc = subprocess.run(
                ["ps", "-o", "rss=,cputime=", "-p", str(self.pid)],
                capture_output=True,
                text=True,
            )
            fields = proc.stdout.split()
            if proc.returncode == 0 and len(fields) >= 2:
                try:
                    self.samples.append(
                        Sample(
                            t=time.monotonic(),
                            rss_bytes=int(fields[0]) * 1024,
                            cpu_seconds=parse_cputime(fields[1]),
                        )
                    )
                except ValueError:
                    pass
            self._stop_event.wait(self.interval_s)

    def stop(self):
        self._stop_event.set()
        self.join()
        return self.samples


def aggregate_samples(samples, t_start=None, t_end=None):
    """Aggregate sampler output over an optional [t_start, t_end] window."""
    window = [
        s
        for s in samples
        if (t_start is None or s.t >= t_start) and (t_end is None or s.t <= t_end)
    ]
    if not window:
        return {"sample_count": 0}
    result = {
        "sample_count": len(window),
        "rss_peak_bytes": max(s.rss_bytes for s in window),
        "rss_mean_bytes": sum(s.rss_bytes for s in window) // len(window),
    }
    if len(window) >= 2 and window[-1].t > window[0].t:
        result["cpu_mean_percent"] = round(
            (window[-1].cpu_seconds - window[0].cpu_seconds)
            / (window[-1].t - window[0].t)
            * 100,
            1,
        )
        deltas = [
            (cur.cpu_seconds - prev.cpu_seconds) / (cur.t - prev.t) * 100
            for prev, cur in zip(window, window[1:])
            if cur.t > prev.t
        ]
        if deltas:
            result["cpu_peak_percent"] = round(max(deltas), 1)
    return result
```

- [ ] **Step 4: Run all tests to verify they pass**

Run: `python3 scripts/eval_test.py`
Expected: `OK` (9 tests)

- [ ] **Step 5: Commit**

```bash
git add scripts/eval.py scripts/eval_test.py
git commit -m "feat(eval): ps-based RSS/cputime sampler with windowed aggregation"
```

---

### Task 3: Timestamped runner + CLI metric computation

**Files:**
- Modify: `scripts/eval.py` (append)
- Modify: `scripts/eval_test.py` (append test class)

**Interfaces:**
- Consumes: `ProcessSampler`, `percentile`.
- Produces: `RunResult = namedtuple("RunResult", ["t_spawn", "lines", "returncode", "stderr", "samples"])` where `lines` is `list[(t_monotonic, str)]`; `run_timestamped(cmd, timeout_s=1800) -> RunResult`; `compute_cli_metrics(t_spawn, lines, sleep_ms) -> dict` with keys `load_seconds`, `ttft_prefill_seconds`, `stream_token_count`, `decode_tokens_per_second_streamed`, `token_latency_ms_mean`/`_p50`/`_p95`, and internal window bounds `t_sleep`, `t_gen_start`, `t_last_stream` (popped by the caller before serialization).

- [ ] **Step 1: Write the failing test**

Append to `scripts/eval_test.py`:

```python
class ComputeCliMetricsTest(unittest.TestCase):
    def test_synthetic_generation_timeline(self):
        lines = [
            (1.0, "sleep_after_load_ms=2000"),
            (3.5, "prompt_token_ids=1,2,3"),
            (3.55, "next_token_id=7"),
            (3.6, "stream_token_id=7"),
            (3.6, "stream_text=a"),
            (3.7, "stream_token_id=8"),
            (3.8, "stream_token_id=9"),
            (3.9, "stream_token_id=10"),
        ]
        metrics = ev.compute_cli_metrics(t_spawn=0.0, lines=lines, sleep_ms=2000)
        self.assertAlmostEqual(metrics["load_seconds"], 1.0)
        self.assertAlmostEqual(metrics["ttft_prefill_seconds"], 0.5)
        self.assertEqual(metrics["stream_token_count"], 4)
        self.assertAlmostEqual(metrics["decode_tokens_per_second_streamed"], 10.0)
        self.assertAlmostEqual(metrics["token_latency_ms_p50"], 100.0)
        self.assertAlmostEqual(metrics["t_sleep"], 1.0)
        self.assertAlmostEqual(metrics["t_gen_start"], 3.0)
        self.assertAlmostEqual(metrics["t_last_stream"], 3.9)

    def test_missing_sleep_marker_returns_empty(self):
        self.assertEqual(ev.compute_cli_metrics(0.0, [(1.0, "next_token_id=5")], 2000), {})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 scripts/eval_test.py ComputeCliMetricsTest -v`
Expected: ERROR with `AttributeError: module 'eval' has no attribute 'compute_cli_metrics'`

- [ ] **Step 3: Implement runner and metric computation**

Append to `scripts/eval.py`:

```python
RunResult = namedtuple("RunResult", ["t_spawn", "lines", "returncode", "stderr", "samples"])


def run_timestamped(cmd, timeout_s=1800):
    """Run cmd, timestamping each stdout line while sampling the process."""
    with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as stderr_file:
        t_spawn = time.monotonic()
        proc = subprocess.Popen(
            cmd, stdout=subprocess.PIPE, stderr=stderr_file, text=True
        )
        sampler = ProcessSampler(proc.pid)
        sampler.start()
        lines = []
        try:
            deadline = t_spawn + timeout_s
            for line in proc.stdout:
                lines.append((time.monotonic(), line.rstrip("\n")))
                if time.monotonic() > deadline:
                    proc.kill()
                    raise TimeoutError(f"{cmd[0]} exceeded {timeout_s}s")
            returncode = proc.wait(timeout=60)
        finally:
            sampler.stop()
            if proc.poll() is None:
                proc.kill()
        stderr_file.seek(0)
        stderr = stderr_file.read()
    return RunResult(t_spawn, lines, returncode, stderr, sampler.samples)


def compute_cli_metrics(t_spawn, lines, sleep_ms):
    """Timing metrics from a timestamped `--stream` generation run.

    Relies on ferrite's output ordering: `sleep_after_load_ms=` is printed
    and flushed after weight load but before the sleep; `prompt_token_ids=`
    is the first line printed after prefill; each generated token prints a
    `stream_token_id=` line as it is produced.
    """

    def first_ts(prefix):
        for t, line in lines:
            if line.startswith(prefix):
                return t
        return None

    metrics = {}
    t_sleep = first_ts("sleep_after_load_ms=")
    if t_sleep is None:
        return metrics
    t_gen_start = t_sleep + sleep_ms / 1000
    metrics["load_seconds"] = round(t_sleep - t_spawn, 3)
    metrics["t_sleep"] = t_sleep
    metrics["t_gen_start"] = t_gen_start
    t_prefill_done = first_ts("prompt_token_ids=")
    if t_prefill_done is not None:
        metrics["ttft_prefill_seconds"] = round(t_prefill_done - t_gen_start, 3)
    stream_ts = [t for t, line in lines if line.startswith("stream_token_id=")]
    if len(stream_ts) >= 2:
        deltas_ms = [(b - a) * 1000 for a, b in zip(stream_ts, stream_ts[1:])]
        metrics["stream_token_count"] = len(stream_ts)
        metrics["decode_tokens_per_second_streamed"] = round(
            (len(stream_ts) - 1) / (stream_ts[-1] - stream_ts[0]), 2
        )
        metrics["token_latency_ms_mean"] = round(sum(deltas_ms) / len(deltas_ms), 1)
        metrics["token_latency_ms_p50"] = round(percentile(deltas_ms, 50), 1)
        metrics["token_latency_ms_p95"] = round(percentile(deltas_ms, 95), 1)
        metrics["t_last_stream"] = stream_ts[-1]
    return metrics
```

- [ ] **Step 4: Run all tests to verify they pass**

Run: `python3 scripts/eval_test.py`
Expected: `OK` (11 tests)

- [ ] **Step 5: Commit**

```bash
git add scripts/eval.py scripts/eval_test.py
git commit -m "feat(eval): timestamped subprocess runner and CLI timing metrics"
```

---

### Task 4: CLI phase orchestration

**Files:**
- Modify: `scripts/eval.py` (append)

**Interfaces:**
- Consumes: `run_timestamped`, `compute_cli_metrics`, `parse_kv_lines`, `aggregate_samples`.
- Produces: `EvalConfig = namedtuple("EvalConfig", ["prompt", "generate_tokens", "benchmark_runs", "sleep_ms", "requests"])`; `run_cli_phase(ferrite_bin: Path, model_path: Path, cfg: EvalConfig) -> dict` — keys: `status` ("ok"/"failed"), `generation_command`, `benchmark_command`, all `compute_cli_metrics` keys (window bounds popped), `model_file_bytes`, `scalar_weight_bytes`, `kv_cache_bytes`, `generated_stopped_on_eos`, `rss_post_load_bytes`, `rss_peak_bytes`, `cpu_mean_percent`, `cpu_peak_percent`, `benchmark_avg_ns`, `decode_tokens_per_second_precise`; on failure: `stderr` excerpt.

No unit test (subprocess orchestration); verified end-to-end in Task 8. Syntax gate: `python3 -m py_compile` + full unit suite still green.

- [ ] **Step 1: Implement the CLI phase**

Append to `scripts/eval.py`:

```python
EvalConfig = namedtuple(
    "EvalConfig", ["prompt", "generate_tokens", "benchmark_runs", "sleep_ms", "requests"]
)


def run_cli_phase(ferrite_bin, model_path, cfg):
    """Generation run (wall-clock + ps sampling), then precise benchmark run."""
    phase = {"status": "ok"}
    gen_cmd = [
        str(ferrite_bin),
        "--model", str(model_path),
        "--prompt", cfg.prompt,
        "--sleep-after-load-ms", str(cfg.sleep_ms),
        "--generate-tokens", str(cfg.generate_tokens),
        "--stream",
    ]
    phase["generation_command"] = " ".join(gen_cmd)
    run = run_timestamped(gen_cmd)
    if run.returncode != 0:
        phase["status"] = "failed"
        phase["stderr"] = run.stderr[-2000:]
        return phase

    metrics = compute_cli_metrics(run.t_spawn, run.lines, cfg.sleep_ms)
    t_sleep = metrics.pop("t_sleep", None)
    t_gen_start = metrics.pop("t_gen_start", None)
    t_last_stream = metrics.pop("t_last_stream", None)
    phase.update(metrics)

    kv = parse_kv_lines(line for _, line in run.lines)
    for key in (
        "model_file_bytes",
        "scalar_weight_bytes",
        "kv_cache_bytes",
        "generated_stopped_on_eos",
    ):
        if key in kv:
            phase[key] = kv[key]

    if t_sleep is not None:
        load_window = aggregate_samples(run.samples, t_sleep, t_gen_start)
        if "rss_peak_bytes" in load_window:
            phase["rss_post_load_bytes"] = load_window["rss_peak_bytes"]
    whole_run = aggregate_samples(run.samples)
    if "rss_peak_bytes" in whole_run:
        phase["rss_peak_bytes"] = whole_run["rss_peak_bytes"]
    if t_gen_start is not None and t_last_stream is not None:
        gen_window = aggregate_samples(run.samples, t_gen_start, t_last_stream)
        for key in ("cpu_mean_percent", "cpu_peak_percent"):
            if key in gen_window:
                phase[key] = gen_window[key]

    bench_cmd = [
        str(ferrite_bin),
        "--model", str(model_path),
        "--prompt", cfg.prompt,
        "--benchmark-runs", str(cfg.benchmark_runs),
    ]
    phase["benchmark_command"] = " ".join(bench_cmd)
    bench = run_timestamped(bench_cmd)
    if bench.returncode == 0:
        bench_kv = parse_kv_lines(line for _, line in bench.lines)
        avg_ns = int(bench_kv.get("benchmark_avg_ns", "0"))
        if avg_ns > 0:
            phase["benchmark_avg_ns"] = avg_ns
            phase["decode_tokens_per_second_precise"] = round(1e9 / avg_ns, 2)
    else:
        phase["benchmark_error"] = bench.stderr[-500:]
    return phase
```

- [ ] **Step 2: Syntax gate + unit suite still green**

Run: `python3 -m py_compile scripts/eval.py && python3 scripts/eval_test.py`
Expected: `OK` (11 tests)

- [ ] **Step 3: Commit**

```bash
git add scripts/eval.py
git commit -m "feat(eval): CLI phase orchestration (generation + precise benchmark)"
```

---

### Task 5: Server phase orchestration

**Files:**
- Modify: `scripts/eval.py` (append)

**Interfaces:**
- Consumes: `ProcessSampler`, `aggregate_samples`, `parse_kv_lines`, `EvalConfig`, `API_KEY`.
- Produces: `find_free_port() -> int`; `wait_for_health(port: int, proc: subprocess.Popen, timeout_s=300) -> bool`; `run_server_phase(server_bin: Path, throughput_bin: Path, model_path: Path, cfg: EvalConfig) -> dict` — keys: `status`, `server_command`, `client_command`, throughput-client metrics (`streaming_time_to_first_token_ms`, `streaming_tokens_per_second`, `streaming_token_latency_p50_ms`, `streaming_token_latency_p95_ms`, `streaming_token_events`, `requests_per_second`, `elapsed_ms`, `streaming_usage_prompt_tokens`, `streaming_usage_completion_tokens`), `server_rss_peak_bytes`, `server_cpu_mean_percent`, `server_cpu_peak_percent`; on failure `error` (+ `server_log` excerpt).

No unit test; verified end-to-end in Task 8.

- [ ] **Step 1: Implement the server phase**

Append to `scripts/eval.py`:

```python
def find_free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_for_health(port, proc, timeout_s=300):
    """Poll GET /health until 200, the process dies, or timeout."""
    deadline = time.monotonic() + timeout_s
    url = f"http://127.0.0.1:{port}/health"
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            return False
        try:
            with urllib.request.urlopen(url, timeout=2) as response:
                if response.status == 200:
                    return True
        except OSError:
            pass
        time.sleep(0.5)
    return False


def run_server_phase(server_bin, throughput_bin, model_path, cfg):
    """Start ferrite-server, drive it with ferrite-openai-throughput, tear down."""
    phase = {"status": "ok"}
    port = find_free_port()
    server_cmd = [
        str(server_bin),
        "--model", str(model_path),
        "--model-id", "eval",
        "--bind", f"127.0.0.1:{port}",
        "--api-key", API_KEY,
        "--default-max-tokens", str(cfg.generate_tokens),
        "--hard-max-tokens", str(cfg.generate_tokens),
        "--inference-wait-ms", "120000",
    ]
    phase["server_command"] = " ".join(server_cmd)
    with tempfile.TemporaryFile(mode="w+", encoding="utf-8") as server_log:
        server = subprocess.Popen(server_cmd, stdout=server_log, stderr=subprocess.STDOUT)
        sampler = ProcessSampler(server.pid)
        sampler.start()
        try:
            if not wait_for_health(port, server):
                server_log.seek(0)
                phase["status"] = "failed"
                phase["error"] = "server never became healthy"
                phase["server_log"] = server_log.read()[-2000:]
                return phase
            client_cmd = [
                str(throughput_bin),
                "--addr", f"127.0.0.1:{port}",
                "--model", "eval",
                "--endpoint", "chat-completions",
                "--prompt", cfg.prompt,
                "--requests", str(cfg.requests),
                "--max-tokens", str(cfg.generate_tokens),
                "--stream",
                "--stream-usage",
                "--api-key", API_KEY,
            ]
            phase["client_command"] = " ".join(client_cmd)
            t_requests_start = time.monotonic()
            client = subprocess.run(client_cmd, capture_output=True, text=True, timeout=1800)
            t_requests_end = time.monotonic()
            if client.returncode != 0:
                phase["status"] = "failed"
                phase["error"] = (client.stderr or client.stdout)[-2000:]
                return phase
            kv = parse_kv_lines(client.stdout.splitlines())
            for key in (
                "streaming_time_to_first_token_ms",
                "streaming_tokens_per_second",
                "streaming_token_latency_p50_ms",
                "streaming_token_latency_p95_ms",
                "streaming_token_events",
                "requests_per_second",
                "elapsed_ms",
                "streaming_usage_prompt_tokens",
                "streaming_usage_completion_tokens",
            ):
                if key in kv:
                    phase[key] = kv[key]
            request_window = aggregate_samples(
                sampler.samples, t_requests_start, t_requests_end
            )
            for src, dst in (
                ("rss_peak_bytes", "server_rss_peak_bytes"),
                ("cpu_mean_percent", "server_cpu_mean_percent"),
                ("cpu_peak_percent", "server_cpu_peak_percent"),
            ):
                if src in request_window:
                    phase[dst] = request_window[src]
            return phase
        finally:
            sampler.stop()
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()
```

- [ ] **Step 2: Syntax gate + unit suite still green**

Run: `python3 -m py_compile scripts/eval.py && python3 scripts/eval_test.py`
Expected: `OK` (11 tests)

- [ ] **Step 3: Commit**

```bash
git add scripts/eval.py
git commit -m "feat(eval): server phase orchestration via ferrite-openai-throughput"
```

---

### Task 6: Environment capture + model resolution/download

**Files:**
- Modify: `scripts/eval.py` (append)

**Interfaces:**
- Consumes: `MODELS_DIR`, `DEFAULT_MODEL_URL`, `REPO_ROOT`.
- Produces: `capture_env() -> dict` (timestamp_utc, hostname, platform, python, logical_cores, git_commit, git_branch, git_dirty, rustc_version; darwin: cpu, physical_cores, ram_bytes; linux: best-effort cpu/ram_bytes); `resolve_models(args) -> list[Path]` (from `--model` args, else `target/models/*.gguf` scan, else download offer; `SystemExit(2)` when declined/forbidden); `download_default_model() -> Path`.

No unit test (environment- and network-dependent); verified in Task 8.

- [ ] **Step 1: Implement env capture and model resolution**

Append to `scripts/eval.py`:

```python
def _cmd_output(cmd, cwd=None):
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return proc.stdout.strip() if proc.returncode == 0 else None


def capture_env():
    env = {
        "timestamp_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "hostname": socket.gethostname(),
        "platform": platform.platform(),
        "python": platform.python_version(),
        "logical_cores": os.cpu_count(),
        "git_commit": _cmd_output(["git", "rev-parse", "HEAD"], cwd=REPO_ROOT),
        "git_branch": _cmd_output(["git", "branch", "--show-current"], cwd=REPO_ROOT),
        "git_dirty": bool(_cmd_output(["git", "status", "--porcelain"], cwd=REPO_ROOT)),
        "rustc_version": _cmd_output(["rustc", "--version"]),
    }
    if sys.platform == "darwin":
        env["cpu"] = _cmd_output(["sysctl", "-n", "machdep.cpu.brand_string"])
        physical = _cmd_output(["sysctl", "-n", "hw.physicalcpu"])
        env["physical_cores"] = int(physical) if physical else None
        memsize = _cmd_output(["sysctl", "-n", "hw.memsize"])
        env["ram_bytes"] = int(memsize) if memsize else None
    else:
        try:
            with open("/proc/cpuinfo", encoding="utf-8") as handle:
                for line in handle:
                    if line.startswith("model name"):
                        env["cpu"] = line.partition(":")[2].strip()
                        break
            with open("/proc/meminfo", encoding="utf-8") as handle:
                for line in handle:
                    if line.startswith("MemTotal:"):
                        env["ram_bytes"] = int(line.split()[1]) * 1024
                        break
        except OSError:
            pass
    return env


def download_default_model():
    MODELS_DIR.mkdir(parents=True, exist_ok=True)
    target = MODELS_DIR / DEFAULT_MODEL_URL.rsplit("/", 1)[-1]
    print(f"downloading {DEFAULT_MODEL_URL}")
    print(f"        -> {target}")

    def report(blocks, block_size, total):
        done_mb = blocks * block_size / 1e6
        total_mb = total / 1e6 if total > 0 else float("nan")
        print(f"\r  {done_mb:8.1f} / {total_mb:.1f} MB", end="", flush=True)

    partial = target.with_suffix(".gguf.part")
    urllib.request.urlretrieve(DEFAULT_MODEL_URL, partial, reporthook=report)
    partial.rename(target)
    print()
    return target


def resolve_models(args):
    if args.model:
        models = [Path(m) for m in args.model]
        missing = [str(m) for m in models if not m.is_file()]
        if missing:
            print(f"model file(s) not found: {', '.join(missing)}", file=sys.stderr)
            raise SystemExit(2)
        return models
    found = sorted(MODELS_DIR.glob("*.gguf"))
    if found:
        return found
    hint = (
        f"No .gguf models found in {MODELS_DIR}.\n"
        f"Download one manually, e.g.:\n"
        f"  curl -L -o {MODELS_DIR}/qwen2.5-0.5b-instruct-q4_k_m.gguf \\\n"
        f"    {DEFAULT_MODEL_URL}"
    )
    if args.no_download:
        print(hint, file=sys.stderr)
        raise SystemExit(2)
    if not args.download:
        answer = input(
            f"No models in {MODELS_DIR}. Download the ~400MB reference model "
            "(Qwen2.5-0.5B-Instruct Q4_K_M)? [y/N] "
        )
        if answer.strip().lower() not in ("y", "yes"):
            print(hint, file=sys.stderr)
            raise SystemExit(2)
    return [download_default_model()]
```

- [ ] **Step 2: Syntax gate + unit suite still green**

Run: `python3 -m py_compile scripts/eval.py && python3 scripts/eval_test.py`
Expected: `OK` (11 tests)

- [ ] **Step 3: Commit**

```bash
git add scripts/eval.py
git commit -m "feat(eval): environment capture and model resolution with download offer"
```

---

### Task 7: Report building, Markdown rendering, main()

**Files:**
- Modify: `scripts/eval.py` (append)
- Modify: `scripts/eval_test.py` (append test classes)

**Interfaces:**
- Consumes: everything above.
- Produces: `build_report(env, cfg, model_results, tag=None) -> dict` (`schema_version`, `tag`, `env`, `config`, `models`); `render_markdown(report) -> str`; `output_stem(timestamp: str, model_paths: list[Path]) -> str` (first model stem lowercased, `-multi` suffix when >1); `write_outputs(report, stem) -> (Path, Path)`; `build_binaries() -> dict[str, Path]`; `parse_args(argv) -> argparse.Namespace`; `main(argv=None) -> int`.

- [ ] **Step 1: Write the failing tests**

Append to `scripts/eval_test.py`:

```python
class OutputStemTest(unittest.TestCase):
    def test_single_model(self):
        stem = ev.output_stem("2026-07-09-120000", [Path("target/models/Qwen2.5 0.5B.gguf")])
        self.assertEqual(stem, "2026-07-09-120000-qwen2.5-0.5b")

    def test_multi_model(self):
        stem = ev.output_stem(
            "2026-07-09-120000",
            [Path("a/first.gguf"), Path("b/second.gguf")],
        )
        self.assertEqual(stem, "2026-07-09-120000-first-multi")


class RenderMarkdownTest(unittest.TestCase):
    def test_report_renders_cli_and_server_sections(self):
        report = ev.build_report(
            env={"timestamp_utc": "2026-07-09T12:00:00Z", "hostname": "mac",
                 "cpu": "Apple M-test", "physical_cores": 8,
                 "ram_bytes": 16 << 30, "platform": "macOS", "python": "3.14",
                 "git_commit": "abc123", "git_branch": "main", "git_dirty": False,
                 "rustc_version": "rustc 1.x", "logical_cores": 8},
            cfg=ev.EvalConfig("hi", 64, 64, 2000, 4),
            model_results=[{
                "model_path": "target/models/model.gguf",
                "cli": {"status": "ok", "load_seconds": 1.0,
                        "ttft_prefill_seconds": 0.5,
                        "decode_tokens_per_second_precise": 12.3,
                        "rss_peak_bytes": 1 << 30},
                "server": {"status": "ok",
                           "streaming_time_to_first_token_ms": "450",
                           "streaming_tokens_per_second": "11.5"},
            }],
            tag="unit-test",
        )
        self.assertEqual(report["schema_version"], ev.SCHEMA_VERSION)
        markdown = ev.render_markdown(report)
        self.assertIn("## model.gguf", markdown)
        self.assertIn("| load | 1.0 s |", markdown)
        self.assertIn("12.3", markdown)
        self.assertIn("| TTFT | 450 ms |", markdown)
        self.assertIn("tag: unit-test", markdown)
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 scripts/eval_test.py OutputStemTest RenderMarkdownTest -v`
Expected: ERROR with `AttributeError: module 'eval' has no attribute 'output_stem'`
(Also add `from pathlib import Path` to the test file imports if not present — it is already imported in Task 1's scaffold.)

- [ ] **Step 3: Implement report, markdown, and main**

Append to `scripts/eval.py`:

```python
def build_report(env, cfg, model_results, tag=None):
    return {
        "schema_version": SCHEMA_VERSION,
        "tag": tag,
        "env": env,
        "config": cfg._asdict(),
        "models": model_results,
    }


def _fmt_bytes(value):
    try:
        number = int(value)
    except (TypeError, ValueError):
        return "-"
    return f"{number / (1 << 20):.1f} MiB"


def render_markdown(report):
    env = report["env"]
    lines = [
        f"# Ferrite eval — {env.get('timestamp_utc', '?')}",
        "",
        f"- host: {env.get('hostname')} — {env.get('cpu')}, "
        f"{env.get('physical_cores')} cores, {_fmt_bytes(env.get('ram_bytes'))} RAM",
        f"- os: {env.get('platform')}",
        f"- commit: {env.get('git_commit')} ({env.get('git_branch')}"
        f"{', dirty' if env.get('git_dirty') else ''})",
        f"- rustc: {env.get('rustc_version')}",
        f"- config: {json.dumps(report['config'])}",
    ]
    if report.get("tag"):
        lines.append(f"- tag: {report['tag']}")
    for entry in report["models"]:
        lines += ["", f"## {Path(entry['model_path']).name}", ""]
        cli = entry.get("cli")
        if cli:
            lines += [
                "| CLI metric | value |",
                "| --- | --- |",
                f"| status | {cli.get('status')} |",
                f"| load | {cli.get('load_seconds', '-')} s |",
                f"| TTFT (prefill, load excluded) | {cli.get('ttft_prefill_seconds', '-')} s |",
                f"| decode tok/s (precise, in-process) | {cli.get('decode_tokens_per_second_precise', '-')} |",
                f"| decode tok/s (streamed wall-clock) | {cli.get('decode_tokens_per_second_streamed', '-')} |",
                f"| token latency p50 / p95 | {cli.get('token_latency_ms_p50', '-')} / {cli.get('token_latency_ms_p95', '-')} ms |",
                f"| RSS post-load / peak | {_fmt_bytes(cli.get('rss_post_load_bytes'))} / {_fmt_bytes(cli.get('rss_peak_bytes'))} |",
                f"| CPU mean / peak (generation) | {cli.get('cpu_mean_percent', '-')} / {cli.get('cpu_peak_percent', '-')} % |",
                f"| model file / weights / kv cache | {_fmt_bytes(cli.get('model_file_bytes'))} / {_fmt_bytes(cli.get('scalar_weight_bytes'))} / {_fmt_bytes(cli.get('kv_cache_bytes'))} |",
            ]
        server = entry.get("server")
        if server:
            lines += [
                "",
                "| Server metric | value |",
                "| --- | --- |",
                f"| status | {server.get('status')} |",
                f"| TTFT | {server.get('streaming_time_to_first_token_ms', '-')} ms |",
                f"| streamed tok/s | {server.get('streaming_tokens_per_second', '-')} |",
                f"| token latency p50 / p95 | {server.get('streaming_token_latency_p50_ms', '-')} / {server.get('streaming_token_latency_p95_ms', '-')} ms |",
                f"| requests/s | {server.get('requests_per_second', '-')} |",
                f"| server RSS peak | {_fmt_bytes(server.get('server_rss_peak_bytes'))} |",
                f"| server CPU mean / peak | {server.get('server_cpu_mean_percent', '-')} / {server.get('server_cpu_peak_percent', '-')} % |",
            ]
    lines.append("")
    return "\n".join(lines)


def output_stem(timestamp, model_paths):
    stem = Path(model_paths[0]).stem.lower().replace(" ", "-")
    if len(model_paths) > 1:
        stem += "-multi"
    return f"{timestamp}-{stem}"


def write_outputs(report, stem):
    EVALS_DIR.mkdir(parents=True, exist_ok=True)
    json_path = EVALS_DIR / f"{stem}.json"
    md_path = EVALS_DIR / f"{stem}.md"
    json_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    md_path.write_text(render_markdown(report), encoding="utf-8")
    return json_path, md_path


def build_binaries():
    cmd = ["cargo", "build", "--release", "-p", "ferrite-cli", "-p", "ferrite-server"]
    print("$ " + " ".join(cmd))
    proc = subprocess.run(cmd, cwd=REPO_ROOT)
    if proc.returncode != 0:
        raise SystemExit(proc.returncode)
    release = REPO_ROOT / "target" / "release"
    return {
        "ferrite": release / "ferrite",
        "server": release / "ferrite-server",
        "throughput": release / "ferrite-openai-throughput",
    }


def parse_args(argv):
    parser = argparse.ArgumentParser(
        description="Ferrite evaluation harness: tokens/s, TTFT, memory, CPU."
    )
    parser.add_argument(
        "--model", action="append",
        help="path to a .gguf model (repeatable); default: target/models/*.gguf",
    )
    parser.add_argument("--prompt", default=DEFAULT_PROMPT)
    parser.add_argument("--generate-tokens", type=int, default=64)
    parser.add_argument("--benchmark-runs", type=int, default=64)
    parser.add_argument(
        "--sleep-ms", type=int, default=2000,
        help="post-load pause used to sample the load-only RSS footprint",
    )
    parser.add_argument("--requests", type=int, default=4, help="server phase request count")
    parser.add_argument("--skip-cli", action="store_true")
    parser.add_argument("--skip-server", action="store_true")
    parser.add_argument("--download", action="store_true",
                        help="download the reference model without asking")
    parser.add_argument("--no-download", action="store_true",
                        help="never download; exit 2 if no model is available")
    parser.add_argument("--tag", default=None,
                        help="free-form label recorded in the output (e.g. locus-kv)")
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    models = resolve_models(args)
    bins = build_binaries()
    env = capture_env()
    cfg = EvalConfig(
        prompt=args.prompt,
        generate_tokens=args.generate_tokens,
        benchmark_runs=args.benchmark_runs,
        sleep_ms=args.sleep_ms,
        requests=args.requests,
    )
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%d-%H%M%S")
    model_results = []
    for model_path in models:
        print(f"\n=== evaluating {model_path.name} ===")
        entry = {"model_path": str(model_path)}
        if not args.skip_cli:
            print("cli phase: generation + benchmark runs ...")
            entry["cli"] = run_cli_phase(bins["ferrite"], model_path, cfg)
        if not args.skip_server:
            print("server phase: ferrite-server + throughput client ...")
            entry["server"] = run_server_phase(
                bins["server"], bins["throughput"], model_path, cfg
            )
        model_results.append(entry)
    report = build_report(env, cfg, model_results, tag=args.tag)
    json_path, md_path = write_outputs(report, output_stem(timestamp, models))
    print()
    print(render_markdown(report))
    print(f"wrote {json_path}")
    print(f"wrote {md_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 4: Run all tests + help smoke test**

Run: `python3 scripts/eval_test.py && python3 scripts/eval.py --help`
Expected: `OK` (14 tests), then the argparse help text listing `--model`, `--tag`, `--skip-server`, etc.

- [ ] **Step 5: Commit**

```bash
git add scripts/eval.py scripts/eval_test.py
git commit -m "feat(eval): report building, markdown rendering, and CLI entry point"
```

---

### Task 8: End-to-end verification, first eval record, dev-note

**Files:**
- Create: `scripts/evals/<timestamp>-qwen2.5-0.5b-instruct-q4_k_m.json` + `.md` (produced by the run)
- Create: `documentation/dev-notes/2026-07-09-eval-harness.md`

**Interfaces:**
- Consumes: the complete `scripts/eval.py`.
- Produces: the first committed eval record (baseline for future comparisons).

- [ ] **Step 1: Exercise the no-model error path**

Run: `python3 scripts/eval.py --no-download` (with `target/models/` absent)
Expected: hint message with the manual curl command, exit code 2 (`echo $?`).

- [ ] **Step 2: Full run with download**

Run: `python3 scripts/eval.py --download --tag baseline`
Expected: model downloads to `target/models/qwen2.5-0.5b-instruct-q4_k_m.gguf`, release build completes, CLI phase prints, server phase prints, Markdown summary printed, two files written under `scripts/evals/`.

- [ ] **Step 3: Sanity-check the record**

Read the produced `.json` and `.md`. Verify: `load_seconds` > 0; `ttft_prefill_seconds` plausible (well under load time); `decode_tokens_per_second_precise` and `_streamed` within ~20% of each other; `rss_post_load_bytes` ≥ `model_file_bytes`; server `streaming_tokens_per_second` in the same ballpark as the CLI streamed rate; all command lines recorded.

- [ ] **Step 4: Write the dev-note**

Create `documentation/dev-notes/2026-07-09-eval-harness.md` following the existing dev-note style: what was built (scripts/eval.py + eval_test.py), how measurements are taken (line timestamps, ps cputime deltas, in-process benchmark_avg_ns), the first baseline numbers, and known caveats (TTFT defined as prefill excluding load; streamed tok/s includes pipe/print overhead; ps sampling at 100 ms).

- [ ] **Step 5: Commit everything**

```bash
git add scripts/evals/ documentation/dev-notes/2026-07-09-eval-harness.md
git commit -m "feat(eval): first baseline eval record and dev-note"
```

---

## Self-review notes

- Spec coverage: interface flags (`--model`, `--download`/`--no-download`, `--generate-tokens`, `--benchmark-runs`, `--prompt`, `--requests`, `--skip-cli`/`--skip-server`, `--tag`) → Tasks 6–7. CLI metrics → Tasks 3–4. Server metrics → Task 5. Env capture → Task 6. JSON+MD outputs → Task 7. Error paths (exit 2, failed phases recorded, server teardown) → Tasks 5–6, verified in Task 8. Testing section → unit tests throughout + E2E in Task 8.
- Type consistency: `EvalConfig` defined once (Task 4), consumed by Tasks 5 and 7; `Sample`/`RunResult` namedtuples defined before use; window-bound keys (`t_sleep`, `t_gen_start`, `t_last_stream`) produced by `compute_cli_metrics` and popped in `run_cli_phase`.
- Known judgment call: `run_cli_phase`/`run_server_phase`/`main` have no unit tests (subprocess-heavy); the E2E run in Task 8 is their verification. Pure logic is all unit-tested.
