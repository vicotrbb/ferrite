#!/usr/bin/env python3
"""Validate Ferrite's maintained Markdown documentation."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parent.parent
EXCLUDED_PARTS = {".git", ".superpowers", "target"}
LINK_PATTERN = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")


def markdown_files() -> list[Path]:
    return sorted(
        path
        for path in ROOT.rglob("*.md")
        if not EXCLUDED_PARTS.intersection(path.relative_to(ROOT).parts)
    )


def link_target(raw_target: str) -> str:
    target = raw_target.strip()
    if target.startswith("<") and ">" in target:
        return target[1 : target.index(">")]
    return target.split(maxsplit=1)[0]


def check_em_dashes(files: list[Path]) -> list[str]:
    errors = []
    for path in files:
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if "—" in line:
                errors.append(
                    f"{path.relative_to(ROOT)}:{line_number}: em dash is not allowed"
                )
    return errors


def check_relative_links(files: list[Path]) -> list[str]:
    errors = []
    for path in files:
        text = path.read_text(encoding="utf-8")
        for match in LINK_PATTERN.finditer(text):
            target = unquote(link_target(match.group(1)))
            parsed = urlsplit(target)
            if parsed.scheme or target.startswith(("#", "mailto:")):
                continue
            relative_target = parsed.path
            if not relative_target:
                continue
            resolved = (path.parent / relative_target).resolve()
            try:
                resolved.relative_to(ROOT)
            except ValueError:
                errors.append(
                    f"{path.relative_to(ROOT)}: link escapes repository: {target}"
                )
                continue
            if not resolved.exists():
                errors.append(
                    f"{path.relative_to(ROOT)}: missing relative link target: {target}"
                )
    return errors


def check_process_artifacts(files: list[Path]) -> list[str]:
    errors = []
    for path in files:
        relative = path.relative_to(ROOT)
        directory_parts = set(relative.parts[:-1])
        if directory_parts.intersection({"plans", "specs"}):
            errors.append(f"{relative}: transient plan/spec directory is not allowed")
        if path.stem.endswith(("-plan", "-spec")):
            errors.append(f"{relative}: transient plan/spec file is not allowed")
    return errors


def main() -> int:
    files = markdown_files()
    errors = check_em_dashes(files)
    errors.extend(check_relative_links(files))
    errors.extend(check_process_artifacts(files))
    if errors:
        print("documentation checks failed:", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        return 1
    print(f"documentation checks passed for {len(files)} Markdown files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
