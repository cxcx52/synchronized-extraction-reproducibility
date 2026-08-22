"""Reproduce the estimator-derived columns of the exact-strong table.

This anonymous artifact is a formula-equivalent Python port of the Euclidean
SIS path in Cyclo's ``estimates.ipynb``.  It validates the port against the
pinned notebook baseline and then evaluates the manuscript's exact-strong
parameter line.

Pinned Cyclo commit: 31158a781c16a967c714c7fbde31ae1558e23118
Pinned lattice-estimator commit: 352ddaf4a288a0543f5d9eb588d2f89c7acec463

The calculation uses Cyclo's MATZOV classical reduction-cost model.  It is
not an invocation of the complete Sage notebook and not a multi-attack or
quantum-security estimate.
"""

from __future__ import annotations

import csv
import math
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUT = ROOT / "generated"

PHI = 128
M_RING = 2**20
ELL = 11
COEFFICIENT_DIMENSION = PHI * M_RING * ELL

B = 2**10
B_DIGIT = 1
T = 64
GAMMA = 2**7

Q_EXACT = 2**50 - 351
EXACT_STRONG_EXTENSION_DEGREE = 8

MATZOV_A = 0.29613500308205365
MATZOV_B = 20.387885985467914


def delta_model(block_size: int) -> float:
    """Root-Hermite-factor model used by the pinned Cyclo notebook."""
    return (
        block_size
        / (2 * math.pi * math.e)
        * (math.pi * block_size) ** (1 / block_size)
    ) ** (1 / (2 * (block_size - 1)))


def required_block_size(target_delta: float) -> int:
    block_size = 40
    while delta_model(block_size) >= target_delta:
        block_size += 1
    return block_size


def log2_add(x: float, y: float) -> float:
    top = max(x, y)
    return top + math.log2(2 ** (x - top) + 2 ** (y - top))


def beta_hat(arity: int) -> int:
    return B + 2 * T * arity * B_DIGIT * GAMMA


def synchronized_radius(arity: int) -> int:
    beta = beta_hat(arity)
    return max(2 * beta, 8 * beta * GAMMA, 2 * B_DIGIT)


def product_radius(beta: int, arity: int) -> int:
    """Original printed Cyclo path, used only for the baseline assertion."""
    bar_beta = beta * (2 * GAMMA) ** arity
    bar_beta += arity * 2 * beta * (2 * GAMMA) ** (arity - 1)
    return 2 * (2 * GAMMA) ** arity * bar_beta


def estimate(sis_radius: int, rank: int, modulus: int) -> dict[str, object]:
    """Evaluate the pinned coefficient-SIS reduction-cost path."""
    euclidean_radius = sis_radius * math.sqrt(COEFFICIENT_DIMENSION)
    trivial = euclidean_radius >= (modulus - 1) / 2
    result: dict[str, object] = {
        "rank": rank,
        "coefficient_rank": PHI * rank,
        "log2_euclidean_radius": math.log2(euclidean_radius),
        "trivial_by_centered_check": trivial,
    }
    if trivial:
        result.update(
            {
                "reduction_dimension": "",
                "root_hermite_factor": "",
                "bkz_block_size": "",
                "classical_log2_rop": "",
            }
        )
        return result

    coefficient_rank = PHI * rank
    log_length = math.log2(euclidean_radius)
    log_modulus = math.log2(modulus)
    log_delta_opt = log_length**2 / (4 * coefficient_rank * log_modulus)
    reduction_dimension = math.floor(
        math.sqrt(coefficient_rank * log_modulus / log_delta_opt)
    )
    log2_delta = (
        log_length - (coefficient_rank / reduction_dimension) * log_modulus
    ) / (reduction_dimension - 1)
    root_hermite = 2**log2_delta
    block_size = required_block_size(root_hermite)

    c = 1 / (1 - 2 ** (-MATZOV_A))
    dimensions_for_free = max(
        block_size
        * math.log(4 / 3)
        / math.log(block_size / (2 * math.pi * math.e)),
        0,
    )
    paid_block_size = block_size - dimensions_for_free
    log2_gate = math.log2(c) + MATZOV_A * paid_block_size + MATZOV_B
    log2_svp_calls = math.log2(c * max(reduction_dimension - block_size, 1))
    log2_lll = 3 * math.log2(reduction_dimension)

    result.update(
        {
            "reduction_dimension": reduction_dimension,
            "root_hermite_factor": root_hermite,
            "bkz_block_size": block_size,
            "classical_log2_rop": log2_add(
                log2_lll, log2_svp_calls + log2_gate
            ),
        }
    )
    return result


def validate_port_against_pinned_baseline() -> None:
    """Check the port against the published notebook's pinned L=1 row."""
    baseline_beta = beta_hat(1)
    baseline = estimate(product_radius(baseline_beta, 1), 13, 2**50)
    assert baseline["reduction_dimension"] == 3591
    assert baseline["bkz_block_size"] == 337
    assert abs(float(baseline["classical_log2_rop"]) - 127.08624532188) < 1e-9


def table_row(arity: int) -> dict[str, object]:
    sis_radius = synchronized_radius(arity)
    rank_13 = estimate(sis_radius, 13, Q_EXACT)

    minimum_rank = None
    for rank in range(1, 14):
        candidate = estimate(sis_radius, rank, Q_EXACT)
        if (
            not candidate["trivial_by_centered_check"]
            and float(candidate["classical_log2_rop"]) >= 128
        ):
            minimum_rank = candidate
            break
    if minimum_rank is None:
        raise RuntimeError(f"no tested rank reaches 128 bits for L={arity}")

    return {
        "L": arity,
        "q_exact": Q_EXACT,
        "protocol_extension_degree": EXACT_STRONG_EXTENSION_DEGREE,
        "phi": PHI,
        "m_ring": M_RING,
        "ell": ELL,
        "coefficient_dimension": COEFFICIENT_DIMENSION,
        "beta_hat": beta_hat(arity),
        "B_sync": sis_radius,
        "log2_l2_radius": rank_13["log2_euclidean_radius"],
        "rank13_reduction_dimension": rank_13["reduction_dimension"],
        "rank13_root_hermite_factor": rank_13["root_hermite_factor"],
        "rank13_bkz_block_size": rank_13["bkz_block_size"],
        "rank13_classical_log2_rop": rank_13["classical_log2_rop"],
        "minimum_rank_ge_128": minimum_rank["rank"],
        "minimum_rank_reduction_dimension": minimum_rank["reduction_dimension"],
        "minimum_rank_root_hermite_factor": minimum_rank["root_hermite_factor"],
        "minimum_rank_bkz_block_size": minimum_rank["bkz_block_size"],
        "minimum_rank_classical_log2_rop": minimum_rank["classical_log2_rop"],
    }


def main() -> None:
    validate_port_against_pinned_baseline()
    rows = [table_row(arity) for arity in (1, 2, 4, 8)]

    OUT.mkdir(exist_ok=True)
    output_path = OUT / "exact_strong_estimator_results.csv"
    with output_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)

    print("Pinned Cyclo baseline assertion passed.")
    for row in rows:
        print(
            "L={L}: radius={log2_l2_radius:.4f}, "
            "rank13={rank13_classical_log2_rop:.6f}, "
            "minimum={minimum_rank_ge_128} "
            "({minimum_rank_classical_log2_rop:.6f})".format(**row)
        )
    print(f"Wrote {output_path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
