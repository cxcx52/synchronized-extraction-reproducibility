#!/usr/bin/env python3
"""Mechanically check the q_perf quadratic-CRT nonunit obstruction.

This is a negative audit: it proves that the current whole-ring, single-fork
interface cannot have a uniform 128-bit nonunit-failure upper bound.  It does
not estimate the actual nonunit probability and it does not apply to q_exact.
"""

from __future__ import annotations

import json
import math
from fractions import Fraction
from pathlib import Path


Q_PERF = 2**50 - 2687
N = 128
WEIGHTS = (32, 34)


def is_prime_u64(n: int) -> bool:
    """Deterministic Miller--Rabin for n < 2^64."""
    if n < 2:
        return False
    small_primes = (2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37)
    for prime in small_primes:
        if n % prime == 0:
            return n == prime

    d = n - 1
    s = 0
    while d % 2 == 0:
        s += 1
        d //= 2
    for base in (2, 325, 9375, 28178, 450775, 9780504, 1795265022):
        if base % n == 0:
            continue
        x = pow(base, d, n)
        if x in (1, n - 1):
            continue
        for _ in range(s - 1):
            x = x * x % n
            if x == n - 1:
                break
        else:
            return False
    return True


def valuation_two(n: int) -> int:
    value = 0
    while n % 2 == 0:
        value += 1
        n //= 2
    return value


def support_size(weight: int) -> int:
    return math.comb(N, weight) * 2**weight


def distinct_collision_lower_bound(weight: int) -> Fraction:
    """Lower bound conditioned on C != C' for one fixed quadratic factor."""
    support = support_size(weight)
    codomain = Q_PERF**2
    if support <= codomain:
        return Fraction(0, 1)
    # (q^-2 - M^-1) / (1 - M^-1)
    return Fraction(support - codomain, codomain * (support - 1))


def row(weight: int) -> dict[str, object]:
    support = support_size(weight)
    lower = distinct_collision_lower_bound(weight)
    return {
        "weight": weight,
        "support_size": support,
        "log2_support_size": math.log2(support),
        "quadratic_component_size": Q_PERF**2,
        "log2_quadratic_component_size": 2 * math.log2(Q_PERF),
        "support_over_component": support / Q_PERF**2,
        "distinct_nonunit_probability_lower_bound_numerator": lower.numerator,
        "distinct_nonunit_probability_lower_bound_denominator": lower.denominator,
        "distinct_nonunit_probability_lower_bound": float(lower),
        "minus_log2_distinct_lower_bound": -math.log2(float(lower)),
        "uniform_pointwise_upper_below_2^-128_possible": lower < Fraction(1, 2**128),
    }


def main() -> None:
    assert is_prime_u64(Q_PERF)
    assert Q_PERF % 256 == 129
    assert valuation_two(Q_PERF - 1) == 7
    assert 128 == math.prod((64, 2))

    rows = [row(weight) for weight in WEIGHTS]
    for item in rows:
        assert item["log2_support_size"] > 2 * math.log2(Q_PERF)
        assert item["uniform_pointwise_upper_below_2^-128_possible"] is False

    result = {
        "status": "information-theoretic obstruction",
        "scope": "q_perf whole-ring single-fork unit-difference interface only",
        "constants": {
            "q_perf": Q_PERF,
            "q_is_prime": True,
            "q_mod_256": Q_PERF % 256,
            "v2_q_minus_1": valuation_two(Q_PERF - 1),
            "ring_degree": N,
            "irreducible_factor_count": 64,
            "irreducible_factor_degree": 2,
        },
        "proof_ledger": [
            "X^128+1 splits into 64 irreducible quadratics X^2-omega",
            "reduction modulo any fixed factor maps challenges into F_(q^2)",
            "Cauchy--Schwarz gives unconditional component collision at least q^-2",
            "removing C=C' gives (M-q^2)/(q^2(M-1)) when M>q^2",
            "averaging over the first challenge yields a pointwise obstruction",
        ],
        "rows": rows,
        "minimum_interface_change": {
            "necessary_factor_degree_at_50_bit_q": 3,
            "warning": "degree at least three is necessary, not sufficient; a matching upper bound is still required",
            "alternatives": [
                "change the performance modulus so relevant factors have degree at least three",
                "prove a genuinely joint multi-fork bound under the conditional fork law",
                "change the extraction relation to a rigorously justified component-wise interface",
            ],
        },
        "separation": "No q_exact theorem-line property is imported into q_perf, or conversely.",
    }
    output = Path(__file__).resolve().parent / "generated" / "fixed_weight_crt_obstruction.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output}")
    for item in rows:
        print(
            f"tau={item['weight']}",
            f"log2(M)={item['log2_support_size']:.12f}",
            f"-log2(lower)={item['minus_log2_distinct_lower_bound']:.12f}",
        )


if __name__ == "__main__":
    main()
