#!/usr/bin/env python3
"""Verify a 50-bit quartic-splitting exact-strong fixed-weight line.

This audit is deliberately separate from both the repository's current
quadratic q_perf and the manuscript's degree-eight q_exact.  It certifies a
candidate replacement modulus and the exact-weight boundary of the proof.
"""

from __future__ import annotations

import json
import math
from functools import reduce
from fractions import Fraction
from itertools import product
from pathlib import Path


N = 128
BASE = 3
DIGITS = 32
Q_QUARTIC = (BASE**DIGITS + 1) // 2
CURRENT_Q_PERF = 2**50 - 2687
Q_EXACT = 2**50 - 351
TARGET_WEIGHT = 32
OUTPUT = (
    Path(__file__).resolve().parent
    / "generated"
    / "qperf_quartic_exact_strong_certificate.json"
)

FACTOR_TREES: dict[int, dict[int, int]] = {
    Q_QUARTIC: {2: 6, 5: 1, 17: 1, 41: 1, 193: 1, 21523361: 1},
    21523361: {2: 5, 5: 1, 17: 1, 41: 1, 193: 1},
    193: {2: 6, 3: 1},
    41: {2: 3, 5: 1},
    17: {2: 4},
    5: {2: 2},
    3: {2: 1},
}


def factor_product(factors: dict[int, int]) -> int:
    return math.prod(prime**power for prime, power in factors.items())


def find_lucas_witness(n: int, prime: int) -> int:
    for witness in range(2, 10000):
        if pow(witness, n - 1, n) != 1:
            continue
        if math.gcd(pow(witness, (n - 1) // prime, n) - 1, n) == 1:
            return witness
    raise AssertionError(f"no Lucas witness found for n={n}, p={prime}")


def build_lucas_certificate(n: int) -> dict[str, object]:
    if n == 2:
        return {"n": 2, "kind": "base-prime"}
    factors = FACTOR_TREES[n]
    assert factor_product(factors) == n - 1
    children = {
        str(prime): build_lucas_certificate(prime) for prime in factors
    }
    witnesses = {
        str(prime): find_lucas_witness(n, prime) for prime in factors
    }
    return {
        "n": n,
        "kind": "complete-factorization Lucas certificate",
        "n_minus_one_factorization": {
            str(prime): power for prime, power in factors.items()
        },
        "witnesses": witnesses,
        "children": children,
    }


def verify_lucas_certificate(cert: dict[str, object]) -> None:
    n = int(cert["n"])
    if cert["kind"] == "base-prime":
        assert n == 2
        return
    factors = {int(p): int(e) for p, e in cert["n_minus_one_factorization"].items()}
    assert factor_product(factors) == n - 1
    children = cert["children"]
    witnesses = cert["witnesses"]
    for prime in factors:
        child = children[str(prime)]
        assert int(child["n"]) == prime
        verify_lucas_certificate(child)
        witness = int(witnesses[str(prime)])
        assert pow(witness, n - 1, n) == 1
        assert math.gcd(pow(witness, (n - 1) // prime, n) - 1, n) == 1


def valuation_two(value: int) -> int:
    result = 0
    while value % 2 == 0:
        value //= 2
        result += 1
    return result


def multiplicative_order(value: int, modulus: int) -> int:
    assert math.gcd(value, modulus) == 1
    order = 1
    current = value % modulus
    while current != 1:
        current = current * value % modulus
        order += 1
    return order


def polynomial_multiply(left: list[int], right: list[int], modulus: int) -> list[int]:
    result = [0] * (len(left) + len(right) - 1)
    for i, a in enumerate(left):
        for j, b in enumerate(right):
            result[i + j] = (result[i + j] + a * b) % modulus
    return result


def verify_quartic_factorization() -> list[dict[str, int]]:
    roots = []
    product_poly = [1]
    for exponent in range(1, 64, 2):
        omega = pow(BASE, exponent, Q_QUARTIC)
        assert multiplicative_order(omega, Q_QUARTIC) == 64
        roots.append({"exponent": exponent, "omega": omega})
        product_poly = polynomial_multiply(
            product_poly, [(-omega) % Q_QUARTIC, 0, 0, 0, 1], Q_QUARTIC
        )
    expected = [0] * 129
    expected[0] = 1
    expected[128] = 1
    assert product_poly == expected
    return roots


def signed_digit_profiles(target: int) -> set[tuple[int, int]]:
    """Return (# abs-one digits, # abs-two digits) for 32-digit expansions."""
    states: dict[int, set[tuple[int, int]]] = {target: {(0, 0)}}
    for _ in range(DIGITS):
        next_states: dict[int, set[tuple[int, int]]] = {}
        for remaining, profiles in states.items():
            for digit in range(-2, 3):
                if (remaining - digit) % BASE:
                    continue
                quotient = (remaining - digit) // BASE
                for ones, twos in profiles:
                    profile = (
                        ones + int(abs(digit) == 1),
                        twos + int(abs(digit) == 2),
                    )
                    next_states.setdefault(quotient, set()).add(profile)
        states = next_states
    return states.get(0, set())


def equal_weight_pair_feasible(ones: int, twos: int, weight: int) -> bool:
    """Whether a delta profile can come from two signed ternary weight-w words."""
    return ones % 2 == 0 and twos + ones // 2 <= weight


def verify_weight_boundary() -> dict[str, object]:
    zero_profiles = signed_digit_profiles(0)
    plus_profiles = signed_digit_profiles(Q_QUARTIC)
    minus_profiles = signed_digit_profiles(-Q_QUARTIC)
    expected = {(ones, DIGITS - ones) for ones in range(1, DIGITS, 2)}
    assert zero_profiles == {(0, 0)}
    assert plus_profiles == expected
    assert minus_profiles == expected

    active_component_profiles: dict[str, object] = {}
    current = {(0, 0)}
    for active_components in range(1, 5):
        current = {
            (a_ones + b_ones, a_twos + b_twos)
            for a_ones, a_twos in current
            for b_ones, b_twos in expected
        }
        feasible_32 = sorted(
            [
                [ones, twos]
                for ones, twos in current
                if equal_weight_pair_feasible(ones, twos, TARGET_WEIGHT)
            ]
        )
        even_costs = [
            twos + ones / 2 for ones, twos in current if ones % 2 == 0
        ]
        active_component_profiles[str(active_components)] = {
            "minimum_equal_weight_cost": min(even_costs) if even_costs else None,
            "weight_32_feasible_profiles": feasible_32,
        }
        assert not feasible_32

    # Tight counterexample for weights 33 and 34 at the factor X^4 - 3.
    left = [0] * N
    right = [0] * N
    for residue in (0, 1):
        index = residue
        left[index] = 1
        right[index] = -1
        for digit_index in range(1, DIGITS):
            index = 4 * digit_index + residue
            if residue == 0:
                left[index] = 1
            else:
                right[index] = -1
    assert sum(value != 0 for value in left) == 33
    assert sum(value != 0 for value in right) == 33
    delta = [a - b for a, b in zip(left, right)]
    residues = [
        sum(delta[4 * digit_index + residue] * BASE**digit_index for digit_index in range(DIGITS))
        % Q_QUARTIC
        for residue in range(4)
    ]
    assert residues == [0, 0, 0, 0]
    assert left != right

    left_34 = left.copy()
    right_34 = right.copy()
    left_34[2] = right_34[2] = 1
    assert sum(value != 0 for value in left_34) == 34
    assert sum(value != 0 for value in right_34) == 34
    assert [a - b for a, b in zip(left_34, right_34)] == delta

    return {
        "zero_profiles": sorted(map(list, zero_profiles)),
        "plus_or_minus_q_profiles": sorted(map(list, expected)),
        "active_component_profiles": active_component_profiles,
        "weight_33_counterexample": {
            "left_nonzero_indices_and_values": [
                [index, value] for index, value in enumerate(left) if value
            ],
            "right_nonzero_indices_and_values": [
                [index, value] for index, value in enumerate(right) if value
            ],
            "residue_mod_x4_minus_3": residues,
        },
        "weight_34_counterexample_adds_common_index": 2,
    }


def support_size(weight: int) -> int:
    return math.comb(N, weight) * 2**weight


def structured_support_audit(weights: tuple[int, int, int, int]) -> dict[str, object]:
    """Audit an independent fixed-weight sampler on the four residue classes.

    In one quartic slot each residue-class radix sum has at most two preimages.
    Thus a slot collision is at most 16/support.  Exact challenge equality,
    whose probability is 1/support, is contained in every one of the 32 slot
    collision events.  Removing it slot-by-slot gives the sharper conditioned
    union bound 32*(16-1)/(support-1) = 480/(support-1).
    """
    class_supports = [math.comb(DIGITS, weight) * 2**weight for weight in weights]
    support = math.prod(class_supports)
    conditioned_nonunit = Fraction(32 * (2**4 - 1), support - 1)
    ledger = []
    for terminal_arity, final_query_multiplier in product((1, 2, 4, 8), (1, 2)):
        exposure = 2 * terminal_arity * final_query_multiplier
        combined = exposure * (Fraction(1, support) + conditioned_nonunit)
        ledger.append(
            {
                "L": terminal_arity,
                "M": final_query_multiplier,
                "exposure_2LM": exposure,
                "combined_repeat_and_nonunit_negative_log2": -math.log2(float(combined)),
                "meets_128_bits": combined <= Fraction(1, 2**128),
            }
        )
    return {
        "weights_by_coefficient_class_mod_4": list(weights),
        "operator_norm_bound": sum(weights),
        "class_supports": class_supports,
        "support_size": support,
        "support_log2": math.log2(support),
        "per_slot_collision_bound": f"16/{support}",
        "conditioned_distinct_nonunit_bound": f"480/{support - 1}",
        "conditioned_distinct_nonunit_negative_log2": -math.log2(
            float(conditioned_nonunit)
        ),
        "two_star_combined_ledger": ledger,
    }


def unit_ledger() -> list[dict[str, object]]:
    support = support_size(TARGET_WEIGHT)
    rows = []
    for terminal_arity, final_query_multiplier in product((1, 2, 4, 8), (1, 2)):
        exposure = 2 * terminal_arity * final_query_multiplier
        rows.append(
            {
                "L": terminal_arity,
                "M": final_query_multiplier,
                "exposure_2LM": exposure,
                "loss": f"{exposure}/{support}",
                "negative_log2_loss": math.log2(support) - math.log2(exposure),
                "meets_128_bits": support >= exposure * 2**128,
            }
        )
    return rows


def transfer_precheck() -> dict[str, object]:
    """Old-bound gate screen only; this is not a recalibration or certification."""
    gates = {
        "p26/r1": 462481290421503.7,
        "p26/simple": 524782523278880.44,
        "p28/simple": 555290410208102.5,
        "p30/r1": 543044077889662.0,
        "p30/simple": 520068198812393.44,
        "exact-p26/simple": 519276105607047.75,
        "exact-p28/r3": 472953850901292.25,
        "exact-p28/simple": 554758566778591.6,
        "exact-p29/r2": 458401227545903.3,
    }
    rhs = Q_QUARTIC / 2
    return {
        "warning": "Uses q_perf-installed old-modulus bounds only to locate required redesigns; not a q_quartic completeness or SIS certificate.",
        "q_quartic_over_2": rhs,
        "rows": [
            {"gate": name, "old_bound_lhs": lhs, "passes_new_rhs": lhs < rhs}
            for name, lhs in gates.items()
        ],
    }


def main() -> None:
    assert 2 * Q_QUARTIC == BASE**DIGITS + 1
    assert Q_QUARTIC.bit_length() == 50
    assert Q_QUARTIC % 256 == 65
    assert valuation_two(Q_QUARTIC - 1) == 6
    assert pow(BASE, 32, Q_QUARTIC) == Q_QUARTIC - 1
    assert multiplicative_order(BASE, Q_QUARTIC) == 64
    assert multiplicative_order(Q_QUARTIC % 256, 256) == 4

    primality_certificate = build_lucas_certificate(Q_QUARTIC)
    verify_lucas_certificate(primality_certificate)
    quartic_factors = verify_quartic_factorization()
    weight_boundary = verify_weight_boundary()
    support = support_size(TARGET_WEIGHT)

    result = {
        "status": "proved",
        "scope": "candidate 50-bit quartic replacement line; not current q_perf and not q_exact",
        "moduli_separation": {
            "current_q_perf_quadratic": CURRENT_Q_PERF,
            "candidate_q_quartic": Q_QUARTIC,
            "manuscript_q_exact_degree_eight": Q_EXACT,
        },
        "arithmetic": {
            "q": Q_QUARTIC,
            "q_bits": Q_QUARTIC.bit_length(),
            "q_mod_256": Q_QUARTIC % 256,
            "v2_q_minus_one": valuation_two(Q_QUARTIC - 1),
            "ord_256_q": multiplicative_order(Q_QUARTIC % 256, 256),
            "base": BASE,
            "base_order_mod_q": multiplicative_order(BASE, Q_QUARTIC),
            "identity": "2q = 3^32 + 1",
            "factorization": "X^128+1 = product_{u odd mod 64} (X^4-3^u)",
            "irreducible_factor_count": 32,
            "irreducible_factor_degree": 4,
        },
        "primality_certificate": primality_certificate,
        "quartic_factors": quartic_factors,
        "exact_strong": {
            "challenge_distribution": "uniform signed ternary exact Hamming weight 32 in degree 128",
            "support_size": support,
            "support_log2": math.log2(support),
            "pointwise_repeat_probability": f"1/{support}",
            "distinct_difference_nonunit_probability": 0,
            "operator_norm_bound": TARGET_WEIGHT,
            "proved_weight_range": "every fixed exact weight tau <= 32",
            "tight_failure_boundary": "explicit collisions exist at exact weights 33 and 34",
            "digit_profile_certificate": weight_boundary,
        },
        "structured_approximate_strong_cross_check": {
            "status": "proved alternative; exact weight 32 is norm-superior, while structured weight 40 is ledger-superior at L=8,M=2",
            "explanation": "The supplied (10,10,10,9) construction is rigorous, but has gamma=39 and misses 128 bits at L=8,M=2 under the repository 2LM combined ledger.  The balanced (10,10,10,10) variant clears that ledger with gamma=40.  Both are weaker in norm than the exact-strong weight-32 construction; the weight-40 variant is useful only when the L=8,M=2 exposure is mandatory.",
            "weight_39": structured_support_audit((10, 10, 10, 9)),
            "weight_40": structured_support_audit((10, 10, 10, 10)),
        },
        "two_star_unit_ledger": unit_ledger(),
        "route_comparison": {
            "new_modulus": "proof-closed while preserving a 50-bit word size and the weight-32 sampler; implementation needs quartic rather than quadratic slots",
            "joint_multifork_current_q_perf": "black-box accepted-fork conditioning and short Bezout coefficients remain unresolved; no 128-bit current-interface theorem follows",
            "crt_component_current_q_perf": "a component relation is not the repository whole-ring short SIS relation; standard CRT lifting has no certified centered shortness",
        },
        "implementation_boundary": {
            "current_repository_slots": "64 quadratic slots via two length-64 transforms",
            "required_slots": "32 quartic slots via four length-32 transforms",
            "benchmark_inherited": False,
            "calibration_inherited": False,
            "word_size_class_preserved": True,
            "old_bound_transfer_precheck": transfer_precheck(),
        },
    }

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT}")
    print(
        f"q={Q_QUARTIC} bits={Q_QUARTIC.bit_length()} factors=32xdegree-4 "
        f"support_bits={math.log2(support):.12f} status=proved"
    )
    for row in result["two_star_unit_ledger"]:
        print(
            f"L={row['L']} M={row['M']} bits={row['negative_log2_loss']:.12f} "
            f"pass={row['meets_128_bits']}"
        )


if __name__ == "__main__":
    main()
