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
CURRENT_VERSION = "0.2.0"


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
            CURRENT_VERSION,
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
        archive_name = f"ferrite-v{CURRENT_VERSION}-test-target.tar.gz"
        first = (output_one / archive_name).read_bytes()
        second = (output_two / archive_name).read_bytes()
        assert hashlib.sha256(first).digest() == hashlib.sha256(second).digest()
        with tarfile.open(output_one / archive_name) as archive:
            assert archive.getnames() == [
                f"ferrite-v{CURRENT_VERSION}-test-target",
                f"ferrite-v{CURRENT_VERSION}-test-target/bin",
                f"ferrite-v{CURRENT_VERSION}-test-target/bin/ferrite",
                f"ferrite-v{CURRENT_VERSION}-test-target/bin/ferrite-server",
                f"ferrite-v{CURRENT_VERSION}-test-target/LICENSE",
                f"ferrite-v{CURRENT_VERSION}-test-target/README.md",
                f"ferrite-v{CURRENT_VERSION}-test-target/CHANGELOG.md",
                f"ferrite-v{CURRENT_VERSION}-test-target/SECURITY.md",
            ]


def test_current_release_metadata() -> None:
    run(sys.executable, str(PREFLIGHT), "--version", CURRENT_VERSION)
    with tempfile.TemporaryDirectory() as temporary:
        notes = Path(temporary) / "release-notes.md"
        run(
            sys.executable,
            str(NOTES),
            "--version",
            CURRENT_VERSION,
            "--output",
            str(notes),
        )
        assert notes.read_text().startswith(
            f"# Ferrite v{CURRENT_VERSION}\n\n### Added"
        )


def main() -> int:
    test_deterministic_archive()
    test_current_release_metadata()
    print("release tool tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
