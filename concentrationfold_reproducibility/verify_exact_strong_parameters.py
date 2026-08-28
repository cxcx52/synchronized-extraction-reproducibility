"""Verify the exact-strong parameter, soundness, and communication ledgers.

The script is anonymous and uses only relative output paths.  SymPy is used
for an independent primality and polynomial-factorization check; all remaining
calculations use the Python standard library.
"""

from __future__ import annotations

import csv
import json
import math
import os
import warnings
from pathlib import Path

# SymPy may otherwise select python-flint ground types when the optional
# package is installed.  SymPy 1.13 then fails while sorting finite-field
# factors (``TypeError: nmods cannot be ordered``) on Python 3.12.  The pure
# Python ground types give the same exact computation and make this verifier
# independent of optional site packages.
os.environ.setdefault("SYMPY_GROUND_TYPES", "python")

import sympy as sp
from sympy.utilities.exceptions import SymPyDeprecationWarning


warnings.filterwarnings("ignore", category=SymPyDeprecationWarning)


ROOT = Path(__file__).resolve().parent
OUT = ROOT / "generated"

Q_EXACT = 2**50 - 351
Q_BITS = Q_EXACT.bit_length()
E = 8
PHI = 128
M = 2**20
ELL = 11
ELL_1 = 31
ELL_C = 4
B = 2**10
B_DIGIT = 1
T = 64
GAMMA = 128
K = 3
N = 1
A_PRIME = 13
KFIX = 3.0**-128
ARITIES = (1, 2, 4, 8)


def multiplicative_order(a: int, modulus: int) -> int:
    value = 1
    for order in range(1, modulus + 1):
        value = value * a % modulus
        if value == 1:
            return order
    raise AssertionError("multiplicative order not found")


def ell_0(arity: int) -> int:
    return math.ceil(
        math.log2(PHI * (2 + K + arity * (2 + N + K)))
    )


def beta_hat(arity: int) -> int:
    return B + 2 * T * arity * B_DIGIT * GAMMA


def b_sync(arity: int) -> int:
    beta = beta_hat(arity)
    return max(2 * beta, 8 * beta * GAMMA, 2 * B_DIGIT)


def communication_kib(arity: int) -> float:
    ring_q = arity * (A_PRIME + 1) * PHI
    ring_qe = (K + 2) * (arity + 1) * PHI * E
    field_qe = (2 + arity * (2 * B_DIGIT + 2)) * ELL_1 * E
    return Q_BITS * (ring_q + ring_qe + field_qe) / (8 * 1024)


def soundness_row(arity: int) -> dict[str, object]:
    q_to_e = Q_EXACT**E
    integrated_terms = {
        "integrated_4L_kfix": 4 * arity * KFIX,
        "integrated_sumcheck": 2 * (ell_0(arity) + 2 * ELL_1) / q_to_e,
        "integrated_range": (
            2 * arity * ELL_1 * (2 * B_DIGIT + 3) / q_to_e
        ),
        "integrated_extension": arity * ELL_C / q_to_e,
    }
    row: dict[str, object] = {
        "L": arity,
        "ell_0": ell_0(arity),
        "base_sync_loss_bits": -math.log2(2 * arity * KFIX),
        "integrated_fixed_total_bits": -math.log2(sum(integrated_terms.values())),
        "anchor_count": arity + 1,
        "equal_anchor_budget_exponent": 129 + math.ceil(math.log2(arity + 1)),
        "communication_KiB": communication_kib(arity),
    }
    for name, probability in integrated_terms.items():
        row[f"{name}_inverse_bits"] = -math.log2(probability)

    # Separate diagnostic line retained in Appendix F, never mixed into the
    # exact-strong table.
    heuristic_coefficient = (
        4 * arity * 64
        + 2 * (ell_0(arity) + 2 * ELL_1)
        + 2 * arity * ELL_1 * (2 * B_DIGIT + 3)
        + arity * ELL_C
    )
    row["quadratic_factor_heuristic_coefficient"] = heuristic_coefficient
    return row


def exact_factorization_check() -> dict[str, object]:
    x = sp.symbols("x")
    unit, factor_data = sp.factor_list(
        sp.Poly(x**PHI + 1, x, modulus=Q_EXACT)
    )
    factors = [factor for factor, exponent in factor_data for _ in range(exponent)]

    assert Q_EXACT == 2**50 - 351
    assert sp.isprime(Q_EXACT)
    assert Q_EXACT % 256 == 161
    assert Q_EXACT % 64 == 33
    assert multiplicative_order(Q_EXACT % 256, 256) == E
    assert unit == 1
    assert len(factors) == 16
    assert all(factor.degree() == 8 and factor.is_irreducible for factor in factors)

    margin = Q_EXACT ** (1 / 16) / 8
    assert margin > 1
    return {
        "sympy_version": sp.__version__,
        "q_exact": Q_EXACT,
        "q_bits": Q_BITS,
        "is_prime": True,
        "q_mod_256": Q_EXACT % 256,
        "q_mod_64": Q_EXACT % 64,
        "order_mod_256": E,
        "factor_count": len(factors),
        "factor_degrees": [factor.degree() for factor in factors],
        "all_factors_irreducible": True,
        "strong_margin": margin,
        "log2_ternary_support_size": PHI * math.log2(3),
        "coefficient_dimension": PHI * M * ELL,
    }


def main() -> None:
    OUT.mkdir(exist_ok=True)
    factorization = exact_factorization_check()
    rows = [soundness_row(arity) for arity in ARITIES]

    with (OUT / "exact_strong_parameter_check.json").open(
        "w", encoding="utf-8"
    ) as handle:
        json.dump(factorization, handle, indent=2)
        handle.write("\n")

    with (OUT / "exact_strong_soundness.csv").open(
        "w", newline="", encoding="utf-8"
    ) as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)

    # Manuscript-critical arithmetic assertions.
    l2 = rows[1]
    assert abs(float(l2["integrated_4L_kfix_inverse_bits"]) - 199.875200) < 1e-5
    assert abs(float(l2["integrated_sumcheck_inverse_bits"]) - 392.7905) < 1e-4
    assert abs(float(l2["integrated_range_inverse_bits"]) - 390.7239) < 1e-4
    assert abs(float(l2["integrated_extension_inverse_bits"]) - 397.0) < 1e-6
    assert [row["quadratic_factor_heuristic_coefficient"] for row in rows] == [
        716,
        1288,
        2428,
        4710,
    ]
    assert [round(float(row["communication_KiB"]), 3) for row in rows] == [
        82.520,
        130.762,
        227.246,
        420.215,
    ]

    print("Exact-strong primality and factorization checks passed.")
    for row in rows:
        print(
            "L={L}: base-loss={base_sync_loss_bits:.4f}, "
            "anchor=2^-{equal_anchor_budget_exponent}, "
            "communication={communication_KiB:.3f} KiB".format(**row)
        )
    print("Wrote generated/exact_strong_parameter_check.json")
    print("Wrote generated/exact_strong_soundness.csv")


if __name__ == "__main__":
    main()
