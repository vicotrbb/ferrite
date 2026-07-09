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
