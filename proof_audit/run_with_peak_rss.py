#!/usr/bin/env python3
"""Run a command on Linux and report peak RSS for its whole process tree."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import time


def process_table() -> tuple[dict[int, list[int]], dict[int, int]]:
    children: dict[int, list[int]] = {}
    rss_kib: dict[int, int] = {}
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        pid = int(entry.name)
        try:
            stat = (entry / "stat").read_text(encoding="utf-8")
            after_name = stat[stat.rfind(")") + 2 :].split()
            ppid = int(after_name[1])
            children.setdefault(ppid, []).append(pid)

            for line in (entry / "status").read_text(encoding="utf-8").splitlines():
                if line.startswith("VmRSS:"):
                    rss_kib[pid] = int(line.split()[1])
                    break
        except (FileNotFoundError, PermissionError, ProcessLookupError, ValueError):
            continue
    return children, rss_kib


def descendants(root: int, children: dict[int, list[int]]) -> set[int]:
    result = {root}
    pending = [root]
    while pending:
        parent = pending.pop()
        for child in children.get(parent, []):
            if child not in result:
                result.add(child)
                pending.append(child)
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--interval", type=float, default=0.1)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a command is required after --")
    if os.name != "posix" or not Path("/proc").is_dir():
        parser.error("this monitor requires Linux /proc")

    started = time.monotonic()
    process = subprocess.Popen(command)
    peak_aggregate_kib = 0
    peak_single_process_kib = 0
    peak_process_count = 0

    while True:
        children, rss_kib = process_table()
        tree = descendants(process.pid, children)
        values = [rss_kib[pid] for pid in tree if pid in rss_kib]
        aggregate = sum(values)
        if aggregate > peak_aggregate_kib:
            peak_aggregate_kib = aggregate
            peak_process_count = len(values)
        if values:
            peak_single_process_kib = max(peak_single_process_kib, max(values))
        return_code = process.poll()
        if return_code is not None:
            break
        time.sleep(args.interval)

    elapsed = time.monotonic() - started
    print(
        "PEAK_RSS "
        f"aggregate_kib={peak_aggregate_kib} "
        f"single_process_kib={peak_single_process_kib} "
        f"processes_at_aggregate_peak={peak_process_count} "
        f"elapsed_seconds={elapsed:.3f} "
        f"sample_interval_seconds={args.interval:.3f}",
        flush=True,
    )
    return return_code


if __name__ == "__main__":
    raise SystemExit(main())
