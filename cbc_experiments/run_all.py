#!/usr/bin/env python3
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent

for script in [
    ROOT / "reproduce_cyclo_estimator.py",
    ROOT / "benchmark_integer_ipa.py",
]:
    print(f"\n=== Running {script.name} ===")
    subprocess.run(
        [sys.executable, str(script)],
        check=True
    )
