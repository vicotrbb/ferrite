#!/usr/bin/env python3
"""Validate the metadata and repository state required for a Ferrite release."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
PACKAGE_MANIFESTS = {
    "ferrite-cli": ROOT / "crates/ferrite-cli/Cargo.toml",
    "ferrite-fixtures": ROOT / "crates/ferrite-fixtures/Cargo.toml",
    "ferrite-inference": ROOT / "crates/ferrite-inference/Cargo.toml",
    "ferrite-model": ROOT / "crates/ferrite-model/Cargo.toml",
    "ferrite-server": ROOT / "crates/ferrite-server/Cargo.toml",
}
PUBLISHED_PACKAGES = {"ferrite-model", "ferrite-inference"}
SEMVER = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$")


def command_output(*arguments: str) -> str:
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def load_toml(path: Path) -> dict[str, object]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def release_heading_exists(version: str) -> bool:
    heading = re.compile(rf"^## {re.escape(version)} - \d{{4}}-\d{{2}}-\d{{2}}$", re.MULTILINE)
    return heading.search((ROOT / "CHANGELOG.md").read_text()) is not None


def validate(version: str, require_clean: bool, require_tag: bool, require_main: bool) -> list[str]:
    errors: list[str] = []
    if SEMVER.fullmatch(version) is None:
        errors.append(f"{version!r} is not a final semantic version")

    workspace = load_toml(ROOT / "Cargo.toml")
    workspace_package = workspace.get("workspace", {}).get("package", {})
    if workspace_package.get("license") != "Apache-2.0":
        errors.append("workspace package license must be Apache-2.0")

    for name, path in PACKAGE_MANIFESTS.items():
        manifest = load_toml(path)
        package = manifest.get("package", {})
        if package.get("name") != name:
            errors.append(f"{path.relative_to(ROOT)} has unexpected package name")
        if package.get("version") != version:
            errors.append(f"{name} version is not {version}")
        published = name in PUBLISHED_PACKAGES
        if published and package.get("publish") is False:
            errors.append(f"{name} must remain publishable")
        if not published and package.get("publish") is not False:
            errors.append(f"{name} must set publish = false")

    if "Apache License" not in (ROOT / "LICENSE").read_text():
        errors.append("LICENSE does not contain the Apache License text")
    for relative in ("README.md", "CONTRIBUTING.md", "docs/models.md"):
        if "MIT license" in (ROOT / relative).read_text().lower():
            errors.append(f"{relative} still refers to the MIT license")

    if not release_heading_exists(version):
        errors.append(f"CHANGELOG.md lacks a dated {version} heading")

    for relative in (
        "Dockerfile",
        ".dockerignore",
        ".github/workflows/release.yml",
        "docs/install.md",
        "scripts/package_release.py",
        "scripts/release_notes.py",
    ):
        if not (ROOT / relative).is_file():
            errors.append(f"required release file is missing: {relative}")

    if require_clean and command_output("git", "status", "--porcelain"):
        errors.append("worktree is not clean")

    if require_tag:
        tag = f"v{version}"
        tags = command_output("git", "tag", "--points-at", "HEAD").splitlines()
        if tag not in tags:
            errors.append(f"HEAD is not tagged {tag}")

    if require_main:
        head = command_output("git", "rev-parse", "HEAD")
        main = command_output("git", "rev-parse", "origin/main")
        if head != main:
            errors.append("release tag must point at the current origin/main commit")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True, help="final version without the v prefix")
    parser.add_argument("--require-clean", action="store_true")
    parser.add_argument("--require-tag", action="store_true")
    parser.add_argument("--require-main", action="store_true")
    arguments = parser.parse_args()

    errors = validate(
        arguments.version,
        arguments.require_clean,
        arguments.require_tag,
        arguments.require_main,
    )
    if errors:
        print("release preflight failed:", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        return 1
    print(f"release preflight passed for v{arguments.version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
