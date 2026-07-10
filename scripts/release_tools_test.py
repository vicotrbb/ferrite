#!/usr/bin/env python3
"""Exercise Ferrite's local release packaging and metadata helpers."""

from __future__ import annotations

import hashlib
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
PACKAGER = ROOT / "scripts/package_release.py"
PREFLIGHT = ROOT / "scripts/release_preflight.py"
NOTES = ROOT / "scripts/release_notes.py"


def run(*arguments: str) -> None:
    subprocess.run(arguments, cwd=ROOT, check=True)


def test_deterministic_archive() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        ferrite = root / "ferrite"
        server = root / "ferrite-server"
        ferrite.write_bytes(b"ferrite test binary\n")
        server.write_bytes(b"ferrite-server test binary\n")
        output_one = root / "one"
        output_two = root / "two"
        common = (
            sys.executable,
            str(PACKAGER),
            "--version",
            "0.1.0",
            "--target",
            "test-target",
            "--ferrite",
            str(ferrite),
            "--server",
            str(server),
            "--source-date-epoch",
            "1700000000",
        )
        run(*common, "--output-dir", str(output_one))
        run(*common, "--output-dir", str(output_two))
        archive_name = "ferrite-v0.1.0-test-target.tar.gz"
        first = (output_one / archive_name).read_bytes()
        second = (output_two / archive_name).read_bytes()
        assert hashlib.sha256(first).digest() == hashlib.sha256(second).digest()
        with tarfile.open(output_one / archive_name) as archive:
            assert archive.getnames() == [
                "ferrite-v0.1.0-test-target",
                "ferrite-v0.1.0-test-target/bin",
                "ferrite-v0.1.0-test-target/bin/ferrite",
                "ferrite-v0.1.0-test-target/bin/ferrite-server",
                "ferrite-v0.1.0-test-target/LICENSE",
                "ferrite-v0.1.0-test-target/README.md",
                "ferrite-v0.1.0-test-target/CHANGELOG.md",
                "ferrite-v0.1.0-test-target/SECURITY.md",
            ]


def test_current_release_metadata() -> None:
    run(sys.executable, str(PREFLIGHT), "--version", "0.1.0")
    with tempfile.TemporaryDirectory() as temporary:
        notes = Path(temporary) / "release-notes.md"
        run(sys.executable, str(NOTES), "--version", "0.1.0", "--output", str(notes))
        assert notes.read_text().startswith("# Ferrite v0.1.0\n\n### Added")


def main() -> int:
    test_deterministic_archive()
    test_current_release_metadata()
    print("release tool tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
