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
