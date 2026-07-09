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
        self.assertEqual(
            ev.compute_cli_metrics(0.0, [(1.0, "next_token_id=5")], 2000), {}
        )


class OutputStemTest(unittest.TestCase):
    def test_single_model(self):
        stem = ev.output_stem(
            "2026-07-09-120000", [Path("target/models/Qwen2.5 0.5B.gguf")]
        )
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
            cfg=ev.EvalConfig("hi", 64, 64, 2000, 4, (2,)),
            model_results=[{
                "model_path": "target/models/model.gguf",
                "cli": {"status": "ok", "load_seconds": 1.0,
                        "inference_threads": 8,
                        "ttft_prefill_seconds": 0.5,
                        "decode_tokens_per_second_precise": 12.3,
                        "rss_peak_bytes": 1 << 30,
                        "batch_benchmarks": [{
                            "streams": 2,
                            "status": "ok",
                            "average_step_ns": 20_000_000,
                            "aggregate_tokens_per_second": 100.0,
                            "per_stream_tokens_per_second": 50.0,
                            "stream_0_matches_single": True,
                            "rss_peak_bytes": 2 << 30,
                        }]},
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
        self.assertIn("| inference threads | 8 |", markdown)
        self.assertIn("| 2 | 100.0 | 50.0 | 20.00 ms | True |", markdown)
        self.assertIn("| TTFT | 450 ms |", markdown)
        self.assertIn("tag: unit-test", markdown)


if __name__ == "__main__":
    unittest.main()
