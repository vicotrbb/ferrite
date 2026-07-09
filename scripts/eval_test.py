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
