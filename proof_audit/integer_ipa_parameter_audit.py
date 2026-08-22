#!/usr/bin/env python3
"""Mechanical checks for the normalized integer-IPA parameter ledger.

This script does not prove the extraction theorem.  It verifies the algebraic
expansions and dominance comparisons used by the proposed theorem statement,
and sanity-checks the local cubic-denominator counterexample exactly over Q.
"""

from __future__ import annotations

import argparse
import json
import math
from fractions import Fraction
from pathlib import Path


def s(mu: int, lam: int) -> float:
    if mu < 1:
        raise ValueError("S_mu is defined only for mu >= 1; use D_r = 1 separately")
    return lam * math.log2(2 * mu) + 8 * mu * mu


def e(mu: int, lam: int) -> int:
    return lam * mu


def c_root(r: int, lam: int, log_b: float) -> float:
    return lam * r + log_b


def cbc_unexpanded(r: int, lam: int, log_b: float) -> float:
    if r < 2:
        raise ValueError("the CBC formula requires r >= 2")
    return (
        4 * lam
        + 3
        + 3 * s(r, lam)
        + s(r - 1, lam)
        + e(r, lam)
        + c_root(r, lam, log_b)
    )


def cbc_expanded(r: int, lam: int, log_b: float) -> float:
    if r < 2:
        raise ValueError("the CBC formula requires r >= 2")
    return (
        (2 * r + 8 + 3 * math.log2(r) + math.log2(r - 1)) * lam
        + 32 * r * r
        - 16 * r
        + 11
        + log_b
    )


def local_cramer(r: int, lam: int, log_b: float) -> float:
    if r < 2:
        raise ValueError("the local-Cramer comparison requires r >= 2")
    return (
        4 * lam
        + 4
        + 4 * s(r - 1, lam)
        + e(r - 1, lam)
        + c_root(r, lam, log_b)
    )


def cbc_minus_local_expanded(r: int, lam: int) -> float:
    return (1 + 3 * math.log2(r / (r - 1))) * lam + 48 * r - 25


def fu_old(r: int, lam: int, log_b: float) -> float:
    return (
        (2 * r + 16 + 8 * math.log2(r)) * lam
        + 64 * r * r
        + log_b
        + 9
    )


def old_minus_new_expanded(r: int, lam: int) -> float:
    return (
        (8 + 5 * math.log2(r) - math.log2(r - 1)) * lam
        + 32 * r * r
        + 16 * r
        - 2
    )


def m1_pre_y_envelope(lam: int, log_b_prime: float) -> float:
    # q/2 > 2 H N D^3 + H^3 D^4, with
    # log N = 6 lambda + 32 + log B' and log D = 2 lambda + 32.
    log_n = 6 * lam + 32 + log_b_prime
    log_d = 2 * lam + 32
    term_1 = 1 + lam + log_n + 3 * log_d
    term_2 = 3 * lam + 4 * log_d
    # One bit for the sum and one bit for converting q/2 > ... into q > ... .
    return max(term_1, term_2) + 2


def check_cubic_denominator_example() -> None:
    for p, rho in ((3, 1), (5, 2), (7, 3), (11, 1)):
        assert (2 * rho) % p != 0
        x_l = Fraction(1, p * p)
        x_r = Fraction(-rho, p * p)
        y_l = Fraction(-rho, p)
        y_r = Fraction(1, p)
        for k in (0, 1, 2, 5):
            alpha = rho + k * p
            x_child = alpha * x_l + x_r
            y_child = y_l + alpha * y_r
            assert x_child.denominator <= p
            assert y_child.denominator == 1
        parent_ip = x_l * y_l + x_r * y_r
        assert parent_ip.denominator == p**3


def run_checks(lam: int, log_b: float) -> dict[str, object]:
    check_cubic_denominator_example()
    rows = []
    for r in range(2, 33):
        q_cbc = cbc_unexpanded(r, lam, log_b)
        q_cbc_expanded = cbc_expanded(r, lam, log_b)
        q_local = local_cramer(r, lam, log_b)
        q_old = fu_old(r, lam, log_b)
        assert math.isclose(q_cbc, q_cbc_expanded, abs_tol=1e-9)
        assert math.isclose(
            q_cbc - q_local,
            cbc_minus_local_expanded(r, lam),
            abs_tol=1e-9,
        )
        assert q_cbc > q_local
        assert math.isclose(
            q_old - q_cbc,
            old_minus_new_expanded(r, lam),
            abs_tol=1e-9,
        )
        rows.append(
            {
                "r": r,
                "old_log2_q": q_old,
                "cbc_log2_q": q_cbc,
                "local_cramer_log2_q": q_local,
                "old_minus_cbc": q_old - q_cbc,
            }
        )

    m1_cbc = 15 * lam + 107 + log_b
    m1_outer = 11 * lam + 100 + log_b
    m1_pre_y = m1_pre_y_envelope(lam, log_b)
    assert math.isclose(m1_pre_y, 13 * lam + 131 + log_b, abs_tol=1e-9)
    if lam >= 13:
        assert m1_cbc > m1_pre_y
    assert m1_cbc > m1_outer

    return {
        "lambda": lam,
        "log2_B": log_b,
        "checked_r_range": [2, 32],
        "cubic_denominator_counterexample": "passed",
        "m1": {
            "cbc": m1_cbc,
            "outer_normalized_claim6": m1_outer,
            "pre_y": m1_pre_y,
            "cbc_minus_pre_y": m1_cbc - m1_pre_y,
        },
        "rows": rows,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lambda", dest="lam", type=int, default=128)
    parser.add_argument("--log2-b", type=float, default=0.0)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = run_checks(args.lam, args.log2_b)
    rendered = json.dumps(report, indent=2, sort_keys=True)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered + "\n", encoding="utf-8")
    print(rendered)


if __name__ == "__main__":
    main()
