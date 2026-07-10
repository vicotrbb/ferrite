#!/usr/bin/env python3
"""Reject generated, private, and oversized files from Ferrite's repository."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
FORBIDDEN_DIRECTORIES = {
    ".superpowers",
    "__pycache__",
    "documentation",
    "plans",
    "research",
    "specs",
}
FORBIDDEN_SUFFIXES = {
    ".bin",
    ".gguf",
    ".npy",
    ".npz",
    ".pyc",
    ".pyo",
}
MAX_TRACKED_FILE_BYTES = 1 << 20


def repository_files() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    paths = [Path(raw.decode()) for raw in result.stdout.split(b"\0") if raw]
    return [path for path in paths if (ROOT / path).is_file()]


def validate(paths: list[Path]) -> list[str]:
    errors = []
    for relative in paths:
        absolute = ROOT / relative
        if not absolute.is_file():
            continue

        parts = set(relative.parts)
        if parts.intersection(FORBIDDEN_DIRECTORIES):
            errors.append(f"{relative}: forbidden generated or process directory")
        if relative.stem.endswith(("-plan", "-spec")):
            errors.append(f"{relative}: transient plan or spec file")
        if relative.suffix.lower() in FORBIDDEN_SUFFIXES:
            errors.append(f"{relative}: binary or generated asset is not allowed")

        size = absolute.stat().st_size
        if size > MAX_TRACKED_FILE_BYTES:
            errors.append(
                f"{relative}: repository file is {size} bytes, limit is "
                f"{MAX_TRACKED_FILE_BYTES}"
            )
    return errors


def main() -> int:
    paths = repository_files()
    errors = validate(paths)
    if errors:
        print("repository layout checks failed:", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        return 1
    print(f"repository layout checks passed for {len(paths)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
