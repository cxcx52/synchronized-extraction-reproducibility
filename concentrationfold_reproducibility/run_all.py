"""Run every claim-supporting reproducibility script in this directory."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SCRIPTS = (
    "generate_product_sync_comparison.py",
    "verify_exact_strong_parameters.py",
    "run_exact_strong_estimator_port.py",
    "generate_extractable_anchor_tables.py",
)


def main() -> None:
    for script in SCRIPTS:
        print(f"\n== {script} ==", flush=True)
        subprocess.run([sys.executable, str(ROOT / script)], check=True, cwd=ROOT)


if __name__ == "__main__":
    main()
