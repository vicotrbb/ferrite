#!/usr/bin/env python3
"""Create a deterministic Ferrite native release archive."""

from __future__ import annotations

import argparse
import gzip
import io
import os
import re
import sys
import tarfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SEMVER = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$")
DOCUMENTS = ("LICENSE", "README.md", "CHANGELOG.md", "SECURITY.md")


def normalized_info(name: str, mode: int, timestamp: int, size: int = 0) -> tarfile.TarInfo:
    info = tarfile.TarInfo(name)
    info.mode = mode
    info.uid = 0
    info.gid = 0
    info.uname = "root"
    info.gname = "root"
    info.mtime = timestamp
    info.size = size
    return info


def add_directory(archive: tarfile.TarFile, name: str, timestamp: int) -> None:
    info = normalized_info(name.rstrip("/") + "/", 0o755, timestamp)
    info.type = tarfile.DIRTYPE
    archive.addfile(info)


def add_file(archive: tarfile.TarFile, source: Path, name: str, mode: int, timestamp: int) -> None:
    data = source.read_bytes()
    archive.addfile(normalized_info(name, mode, timestamp, len(data)), io.BytesIO(data))


def package(
    version: str,
    target: str,
    ferrite: Path,
    server: Path,
    output_dir: Path,
    timestamp: int,
) -> Path:
    if SEMVER.fullmatch(version) is None:
        raise ValueError(f"{version!r} is not a final semantic version")
    for binary in (ferrite, server):
        if not binary.is_file():
            raise ValueError(f"release binary is missing: {binary}")

    root_name = f"ferrite-v{version}-{target}"
    output_dir.mkdir(parents=True, exist_ok=True)
    archive_path = output_dir / f"{root_name}.tar.gz"
    temporary_path = archive_path.with_suffix(".tar.gz.tmp")

    with temporary_path.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=timestamp) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                add_directory(archive, root_name, timestamp)
                add_directory(archive, f"{root_name}/bin", timestamp)
                add_file(archive, ferrite, f"{root_name}/bin/ferrite", 0o755, timestamp)
                add_file(archive, server, f"{root_name}/bin/ferrite-server", 0o755, timestamp)
                for document in DOCUMENTS:
                    add_file(archive, ROOT / document, f"{root_name}/{document}", 0o644, timestamp)

    os.replace(temporary_path, archive_path)
    return archive_path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--ferrite", required=True, type=Path)
    parser.add_argument("--server", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument(
        "--source-date-epoch",
        type=int,
        default=int(os.environ.get("SOURCE_DATE_EPOCH", "0")),
    )
    arguments = parser.parse_args()
    if arguments.source_date_epoch < 0:
        print("source date epoch must be non-negative", file=sys.stderr)
        return 2

    try:
        archive = package(
            arguments.version,
            arguments.target,
            arguments.ferrite,
            arguments.server,
            arguments.output_dir,
            arguments.source_date_epoch,
        )
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2
    print(archive)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
