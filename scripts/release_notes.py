#!/usr/bin/env python3
"""Extract one Ferrite release section from CHANGELOG.md as release notes."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent


def release_notes(version: str) -> str:
    lines = (ROOT / "CHANGELOG.md").read_text().splitlines()
    heading = re.compile(rf"^## {re.escape(version)} - \d{{4}}-\d{{2}}-\d{{2}}$")
    start = next((index for index, line in enumerate(lines) if heading.fullmatch(line)), None)
    if start is None:
        raise ValueError(f"no dated changelog section exists for {version}")
    end = next(
        (index for index in range(start + 1, len(lines)) if lines[index].startswith("## ")),
        len(lines),
    )
    body = "\n".join(lines[start + 1 : end]).strip()
    if not body:
        raise ValueError(f"changelog section {version} has no release notes")
    return f"# Ferrite v{version}\n\n{body}\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    try:
        notes = release_notes(arguments.version)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2
    arguments.output.write_text(notes)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
