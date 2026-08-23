#!/usr/bin/env python3
"""Run a command and report peak RSS for its whole process tree.

Linux uses ``/proc`` without third-party dependencies.  Other platforms use
``psutil`` when available.  ``--log`` tees combined stdout/stderr to a durable
raw log while preserving live console output.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

try:
    import psutil
except ImportError:  # Linux /proc remains dependency-free.
    psutil = None


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


def rss_values(root: int) -> list[int]:
    if psutil is not None:
        try:
            processes = [psutil.Process(root)]
            processes.extend(processes[0].children(recursive=True))
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            return []
        values = []
        for process in processes:
            try:
                values.append(process.memory_info().rss // 1024)
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                continue
        return values

    if os.name == "posix" and Path("/proc").is_dir():
        children, rss_kib = process_table()
        tree = descendants(root, children)
        return [rss_kib[pid] for pid in tree if pid in rss_kib]

    raise RuntimeError("process-tree monitoring requires psutil or Linux /proc")


def pump_output(stream, log_file) -> None:
    while True:
        chunk = stream.read(8192)
        if not chunk:
            break
        sys.stdout.buffer.write(chunk)
        sys.stdout.buffer.flush()
        log_file.write(chunk)
        log_file.flush()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--interval", type=float, default=0.1)
    parser.add_argument("--log", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a command is required after --")
    if args.interval <= 0:
        parser.error("--interval must be positive")
    if psutil is None and (os.name != "posix" or not Path("/proc").is_dir()):
        parser.error("process-tree monitoring requires psutil or Linux /proc")

    started = time.monotonic()
    log_file = None
    pump = None
    popen_args = {}
    if args.log is not None:
        args.log.parent.mkdir(parents=True, exist_ok=True)
        log_file = args.log.open("wb")
        popen_args = {"stdout": subprocess.PIPE, "stderr": subprocess.STDOUT}
    process = subprocess.Popen(command, **popen_args)
    if log_file is not None:
        assert process.stdout is not None
        pump = threading.Thread(
            target=pump_output,
            args=(process.stdout, log_file),
            daemon=True,
        )
        pump.start()
    peak_aggregate_kib = 0
    peak_single_process_kib = 0
    peak_process_count = 0

    while True:
        values = rss_values(process.pid)
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

    if pump is not None:
        pump.join()
    elapsed = time.monotonic() - started
    result = (
        "PEAK_RSS "
        f"aggregate_kib={peak_aggregate_kib} "
        f"single_process_kib={peak_single_process_kib} "
        f"processes_at_aggregate_peak={peak_process_count} "
        f"elapsed_seconds={elapsed:.3f} "
        f"sample_interval_seconds={args.interval:.3f}"
    )
    print(result, flush=True)
    if log_file is not None:
        log_file.write((result + "\n").encode())
        log_file.close()
    return return_code


if __name__ == "__main__":
    raise SystemExit(main())
