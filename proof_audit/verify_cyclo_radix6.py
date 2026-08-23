#!/usr/bin/env python3
"""Verify the explicit radix-six quadratic-splitting Cyclo sampler.

The verifier uses only the Python standard library.  In particular, primality
of the 59-bit modulus is not delegated to a probable-prime routine: the data
below form a complete recursive Pocklington certificate for the modulus.
Running this file verifies the certificate and every derived inequality, then
writes ``generated/cyclo_radix6_certificate.json``.
"""

from __future__ import annotations

import json
import math
from fractions import Fraction
from pathlib import Path


Q = 447_183_309_836_853_377
N = 128
RADIX = 6
DIGITS = 22
ALPHABET = (-3, -2, -1, 0, 1, 2)

# Each entry gives the complete factorization of n-1 and one Pocklington
# witness for every distinct prime divisor.  All prime divisors are themselves
# certified by another entry, down to the base prime 2.
POCKLINGTON = {
    2: {"factors": [], "witnesses": {}},
    3: {"factors": [(2, 1)], "witnesses": {2: 2}},
    5: {"factors": [(2, 2)], "witnesses": {2: 2}},
    7: {"factors": [(2, 1), (3, 1)], "witnesses": {2: 3, 3: 2}},
    11: {"factors": [(2, 1), (5, 1)], "witnesses": {2: 2, 5: 2}},
    13: {"factors": [(2, 2), (3, 1)], "witnesses": {2: 2, 3: 2}},
    23: {"factors": [(2, 1), (11, 1)], "witnesses": {2: 5, 11: 2}},
    29: {"factors": [(2, 2), (7, 1)], "witnesses": {2: 2, 7: 2}},
    47: {"factors": [(2, 1), (23, 1)], "witnesses": {2: 5, 23: 2}},
    547: {
        "factors": [(2, 1), (3, 1), (7, 1), (13, 1)],
        "witnesses": {2: 2, 3: 2, 7: 2, 13: 2},
    },
    2_633: {
        "factors": [(2, 3), (7, 1), (47, 1)],
        "witnesses": {2: 3, 7: 2, 47: 2},
    },
    15_313: {
        "factors": [(2, 4), (3, 1), (11, 1), (29, 1)],
        "witnesses": {2: 5, 3: 2, 11: 2, 29: 2},
    },
    19_801: {
        "factors": [(2, 3), (3, 2), (5, 2), (11, 1)],
        "witnesses": {2: 7, 3: 2, 5: 2, 11: 2},
    },
    11_522_009: {
        "factors": [(2, 3), (547, 1), (2_633, 1)],
        "witnesses": {2: 3, 547: 2, 2_633: 2},
    },
    Q: {
        "factors": [(2, 7), (15_313, 1), (19_801, 1), (11_522_009, 1)],
        "witnesses": {2: 3, 15_313: 2, 19_801: 2, 11_522_009: 2},
    },
}


def fraction_record(value: Fraction) -> dict[str, int | float | str]:
    bits = math.log2(value.denominator) - math.log2(value.numerator)
    return {
        "numerator": value.numerator,
        "denominator": value.denominator,
        "decimal": float(value),
        "negative_log2": bits,
    }


def verify_pocklington(n: int, verified: set[int]) -> None:
    if n in verified:
        return
    assert n in POCKLINGTON, f"missing certificate node for {n}"
    if n == 2:
        verified.add(n)
        return

    node = POCKLINGTON[n]
    factors = node["factors"]
    product = 1
    for prime, exponent in factors:
        assert exponent >= 1
        verify_pocklington(prime, verified)
        product *= prime**exponent
    assert product == n - 1, f"factorization of {n}-1 is incomplete"
    assert product * product > n, "Pocklington known factor must exceed sqrt(n)"

    witnesses = node["witnesses"]
    assert set(witnesses) == {prime for prime, _ in factors}
    for prime, _ in factors:
        witness = witnesses[prime]
        assert 1 < witness < n
        assert pow(witness, n - 1, n) == 1
        residue = pow(witness, (n - 1) // prime, n)
        assert math.gcd(residue - 1, n) == 1
    verified.add(n)


def polynomial_multiply(left: list[int], right: list[int]) -> list[int]:
    product = [0] * (len(left) + len(right) - 1)
    for i, a in enumerate(left):
        for j, b in enumerate(right):
            product[i + j] = (product[i + j] + a * b) % Q
    return product


def verify_factorization() -> list[int]:
    assert Q % 256 == 129
    assert pow(6, 64, Q) == Q - 1
    assert pow(6, 128, Q) == 1
    for proper_divisor in (1, 2, 4, 8, 16, 32, 64):
        assert pow(6, proper_divisor, Q) != 1

    omegas = [pow(6, odd, Q) for odd in range(1, 128, 2)]
    assert len(omegas) == 64
    assert len(set(omegas)) == 64

    product = [1]
    for omega in omegas:
        # omega has order 128.  Since v_2(Q-1)=7, it is a nonsquare, so
        # X^2-omega is irreducible over F_Q.
        assert pow(omega, (Q - 1) // 2, Q) == Q - 1
        product = polynomial_multiply(product, [(-omega) % Q, 0, 1])

    expected = [0] * 129
    expected[0] = 1
    expected[128] = 1
    assert product == expected
    return omegas


def verify_radix_reindexing() -> None:
    assert len(ALPHABET) == RADIX
    assert tuple(range(min(ALPHABET), max(ALPHABET) + 1)) == ALPHABET
    assert RADIX**DIGITS - 1 < Q
    assert Q < RADIX ** (DIGITS + 1) - 1

    # For every omega=6^u, u odd, select 22 of the 64 even (respectively odd)
    # coefficient positions.  Their powers are signed 6^t for t=0,...,21.
    for odd in range(1, 128, 2):
        inverse = pow(odd, -1, 128)
        selected: list[int] = []
        omega = pow(6, odd, Q)
        for exponent in range(DIGITS):
            residue = (inverse * exponent) % 128
            if residue < 64:
                index, sign = residue, 1
            else:
                index, sign = residue - 64, -1
            selected.append(index)
            assert pow(omega, index, Q) == (sign * pow(6, exponent, Q)) % Q
        assert len(set(selected)) == DIGITS

    # If two selected digit blocks collide modulo Q, their integer difference
    # has absolute value at most 5*(1+6+...+6^21)=6^22-1<Q.  It is therefore
    # zero over the integers.  Reduction modulo 6 and induction force every
    # digit difference to be zero.  The assertions below check the numerical
    # premises of that argument.
    max_digit_difference = max(ALPHABET) - min(ALPHABET)
    max_block_difference = max_digit_difference * sum(
        RADIX**i for i in range(DIGITS)
    )
    assert max_digit_difference == RADIX - 1
    assert max_block_difference == RADIX**DIGITS - 1
    assert max_block_difference < Q


def pocklington_json() -> dict[str, dict]:
    return {
        str(n): {
            "n": n,
            "factorization_of_n_minus_1": [
                {"prime": prime, "exponent": exponent}
                for prime, exponent in node["factors"]
            ],
            "witness_by_prime_divisor": {
                str(prime): witness for prime, witness in node["witnesses"].items()
            },
        }
        for n, node in sorted(POCKLINGTON.items())
    }


def main() -> None:
    verified: set[int] = set()
    verify_pocklington(Q, verified)
    omegas = verify_factorization()
    verify_radix_reindexing()

    support_size = RADIX**N
    repeat_probability = Fraction(1, support_size)
    slot_collision_bound = Fraction(1, RADIX ** (2 * DIGITS))
    union_bound = 64 * slot_collision_bound
    conditioned_distinct = (union_bound - repeat_probability) / (
        1 - repeat_probability
    )
    assert conditioned_distinct > 0

    minimum_digits = next(
        digits
        for digits in range(1, 65)
        if (
            math.log2(
                ((64 * Fraction(1, RADIX ** (2 * digits)) - repeat_probability)
                 / (1 - repeat_probability)).denominator
            )
            - math.log2(
                ((64 * Fraction(1, RADIX ** (2 * digits)) - repeat_probability)
                 / (1 - repeat_probability)).numerator
            )
        )
        >= 128
    )
    assert minimum_digits == 26

    factor_of_six_to_64_plus_one = (
        4_926_056_449 * Q * 28_753_787_197_056_661_026_689
    )
    assert factor_of_six_to_64_plus_one == 6**64 + 1

    result = {
        "status": "proved",
        "scope": "explicit heuristic-free degree-2 Cyclo approximate-strong sampler",
        "modulus": {
            "q": Q,
            "bit_length": Q.bit_length(),
            "q_mod_256": Q % 256,
            "six_to_64_mod_q": pow(6, 64, Q),
            "six_to_128_mod_q": pow(6, 128, Q),
            "factorization_of_six_to_64_plus_one": [
                4_926_056_449,
                Q,
                28_753_787_197_056_661_026_689,
            ],
            "pocklington_certificate": pocklington_json(),
        },
        "cyclotomic_factorization": {
            "polynomial": "X^128 + 1",
            "number_of_factors": 64,
            "factor_degree": 2,
            "factors": [f"X^2 - {omega}" for omega in omegas],
            "all_constant_terms_are_nonsquares": True,
        },
        "challenge": {
            "coefficient_alphabet": list(ALPHABET),
            "coefficient_law": "independent exact uniform",
            "dimension": N,
            "support_size": support_size,
            "support_bits": math.log2(support_size),
            "radix_digits_per_slot_coordinate": DIGITS,
            "radix_injectivity_inequality": {
                "six_to_22_minus_1": 6**22 - 1,
                "q": Q,
                "six_to_23_minus_1": 6**23 - 1,
            },
            "operator_norm_upper_bound": N * max(abs(value) for value in ALPHABET),
            "difference_operator_norm_upper_bound": 2
            * N
            * max(abs(value) for value in ALPHABET),
        },
        "probabilities": {
            "single_slot_point_probability_upper": fraction_record(slot_collision_bound),
            "challenge_repeat_probability": fraction_record(repeat_probability),
            "unconditioned_nonunit_union_bound": fraction_record(union_bound),
            "conditioned_on_distinct_nonunit_bound": fraction_record(
                conditioned_distinct
            ),
        },
        "design_frontier": {
            "minimum_radix_digits_for_128_bit_union_bound": minimum_digits,
            "injectivity_requires_q_at_least": RADIX**minimum_digits,
            "minimum_modulus_bit_length_from_this_argument": (
                RADIX**minimum_digits
            ).bit_length(),
            "interpretation": "trade-off observation for this radix proof only",
        },
    }

    output = Path(__file__).resolve().parent / "generated" / "cyclo_radix6_certificate.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output}")
    print(f"q={Q} bits={Q.bit_length()} factors=64xdegree-2")
    print(
        "conditioned_nonunit_bits="
        f"{result['probabilities']['conditioned_on_distinct_nonunit_bound']['negative_log2']:.12f}"
    )
    print(
        "frontier_digits="
        f"{minimum_digits} frontier_modulus_bits={(RADIX**minimum_digits).bit_length()}"
    )


if __name__ == "__main__":
    main()
