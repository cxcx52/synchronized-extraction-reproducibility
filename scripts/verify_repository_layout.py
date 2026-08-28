#!/usr/bin/env python3
"""Lightweight pre-push checks for the public reproducibility repository."""

from __future__ import annotations

import csv
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MAX_TRACKED_BYTES = 20 * 1024 * 1024

REQUIRED = (
    "README.md",
    "REPRODUCIBILITY.md",
    "requirements.txt",
    "concentrationfold_reproducibility/run_all.py",
    "concentrationfold_reproducibility/generated/product_vs_sync.csv",
    "proof_audit/README.md",
    "proof_audit/verify_cyclo_radix6.py",
    "proof_audit/verify_qperf_quartic_exact_strong.py",
    "rokoko/Cargo.toml",
    "rokoko/Cargo.lock",
    "rokoko/LICENSE",
    "rokoko/Makefile",
)

TEXT_SUFFIXES = {
    ".csv",
    ".json",
    ".md",
    ".py",
    ".rs",
    ".sh",
    ".tex",
    ".toml",
    ".txt",
}

FORBIDDEN_ABSOLUTE_MARKERS = (
    "C:\\Users\\",
    "C:/Users/",
    "/home/",
    "/Users/",
)


def tracked_files() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
    )
    return [ROOT / item.decode("utf-8") for item in result.stdout.split(b"\0") if item]


def check_metric_csv(path: Path) -> None:
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream))
    row = next(item for item in rows if int(item["L"]) == 2)
    gate = float(row["log2_centered_gate"])
    shifted = float(row["log2_euclidean_scaled_gate"])
    assert float(row["log2_B_sync"]) < gate < float(row["log2_B_prod"])
    assert float(row["log2_l2_sync"]) < shifted < float(row["log2_l2_prod"])


def main() -> int:
    errors: list[str] = []
    warnings: list[str] = []

    for relative in REQUIRED:
        if not (ROOT / relative).is_file():
            errors.append(f"missing required file: {relative}")

    files = tracked_files()
    for path in files:
        relative = path.relative_to(ROOT).as_posix()
        if not path.is_file():
            continue
        if path.stat().st_size > MAX_TRACKED_BYTES:
            warnings.append(
                f"large tracked file ({path.stat().st_size / 2**20:.1f} MiB): {relative}"
            )
        if path.suffix == ".json":
            try:
                json.loads(path.read_text(encoding="utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                errors.append(f"invalid JSON: {relative}: {exc}")
        if (
            path.resolve() != Path(__file__).resolve()
            and path.suffix in TEXT_SUFFIXES
            and path.stat().st_size <= 2 * 1024 * 1024
        ):
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError as exc:
                errors.append(f"non-UTF-8 text file: {relative}: {exc}")
                continue
            for marker in FORBIDDEN_ABSOLUTE_MARKERS:
                if marker in text:
                    errors.append(f"absolute local path marker {marker!r} in {relative}")

    try:
        check_metric_csv(
            ROOT
            / "concentrationfold_reproducibility"
            / "generated"
            / "product_vs_sync.csv"
        )
    except (AssertionError, KeyError, StopIteration, ValueError) as exc:
        errors.append(f"metric-normalization CSV check failed: {exc}")

    for warning in warnings:
        print(f"WARNING: {warning}")
    for error in errors:
        print(f"ERROR: {error}")

    if errors:
        print(f"repository check failed with {len(errors)} error(s)")
        return 1
    print(f"repository check passed: {len(files)} tracked files, {len(warnings)} warning(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
