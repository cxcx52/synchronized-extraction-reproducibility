"""Generate the extractable-anchor radius and budget checks.

The calculations specialize the formulas in Section 8 and Appendix E to the
pinned exact-strong parameter line.  They are theorem-derived and do not run a
lattice estimator.
"""

from __future__ import annotations

import csv
import math
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUT = ROOT / "generated"

Q_EXACT = 2**50 - 351
PHI = 128
M = 2**20
ELL = 11
B = 2**10
B_DIGIT = 1
C_DIGIT = 1
T = 64
GAMMA = 128
ARITIES = (1, 2, 4, 8)


def beta_hat(arity: int) -> int:
    return B + 2 * T * arity * B_DIGIT * GAMMA


def extractable_anchor_radius(arity: int) -> int:
    return (
        2 * beta_hat(arity)
        + arity * B_DIGIT * GAMMA
        + arity * C_DIGIT * GAMMA
    )


def synchronized_radius(arity: int) -> int:
    beta = beta_hat(arity)
    return max(2 * beta, 8 * beta * GAMMA, 2 * C_DIGIT)


def row(arity: int) -> dict[str, object]:
    conversion = 0.5 * math.log2(M * ELL * PHI)
    radius = extractable_anchor_radius(arity)
    log2_l2_radius = math.log2(radius) + conversion
    centered_gate = math.log2((Q_EXACT - 1) / 2)
    anchor_count = arity + 1
    budget_exponent = 129 + math.ceil(math.log2(anchor_count))
    return {
        "L": arity,
        "beta_hat": beta_hat(arity),
        "B_EA": radius,
        "log2_B_EA": math.log2(radius),
        "log2_l2_EA": log2_l2_radius,
        "centered_gate_margin_bits": centered_gate - log2_l2_radius,
        "B_sync_over_B_EA": synchronized_radius(arity) / radius,
        "anchor_count": anchor_count,
        "sufficient_per_anchor_budget": f"2^-{budget_exponent}",
    }


def main() -> None:
    OUT.mkdir(exist_ok=True)
    rows = [row(arity) for arity in ARITIES]
    output_path = OUT / "extractable_anchor_parameters.csv"
    with output_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)

    for entry in rows:
        print(
            "L={L}: B_EA={B_EA}, log2_l2={log2_l2_EA:.4f}, "
            "anchor_budget={sufficient_per_anchor_budget}".format(**entry)
        )
    print("Wrote generated/extractable_anchor_parameters.csv")


if __name__ == "__main__":
    main()
