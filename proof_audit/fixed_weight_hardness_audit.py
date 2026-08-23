#!/usr/bin/env python3
"""Recompute the current p28 verifier-bound hardness ledger.

Completeness maxima are empirical. After the 2% margin is installed as a
verifier predicate, however, the accepted-witness norm used below is a
deterministic malicious-prover bound. Estimator bit values are the outputs of
the pinned classical Euclidean-SIS MATZOV/GSA Rust port and are rechecked by
the Rust static-certification test named in the generated ledger.
"""

from __future__ import annotations

import json
import math
from pathlib import Path


MOD_Q = 1_125_899_906_839_937
NORM_MARGIN = 1.02
JL_ALPHA_RP = math.sqrt(30.0)
SPECTRAL_OP_NORM_SAFE_BOUND = 32.0
EXTRACTION_SLACK = 8.0
TARGET_BITS = 128.0

# Raw deterministic completeness maxima. These match NB_P_28, FB_P_28 and
# PB_P_28 in src/protocol/params.rs after the combined boundary stream was
# included. None marks the no-projection root.
ROWS = [
    # label, height, NB outer, NB inner, FB, PB, next width,
    # basic bits, recursive outer bits, recursive inner bits
    ("r0", 1 << 14, 13_703_071.36831871, 3_747.026954800299,
     26_360_585.84413571, None, None, 138.0, 151.0, 149.0),
    ("r1", 1 << 13, 27_607_098.932705678, 4_577.198160447066,
     27_583_490.76053078, 23_961_662.027517207, 16, 138.0, 137.0, 141.0),
    ("r2", 1 << 11, 28_436_523.79970082, 5_268.861451964741,
     28_885_042.49396701, 24_907_637.347168, 16, 137.0, 137.0, 136.0),
    ("r3", 1 << 9, 22_176_852.43726607, 5_280.734986722966,
     30_295_320.427160215, 21_096_021.725988906, 8, 137.0, 142.0, 136.0),
    ("r4", 1 << 9, 28_962_741.11024302, 5_273.19912387158,
     26_371_584.948205918, 28_272_485.717434715, 8, 138.0, 137.0, 136.0),
    # The final recursive layer has rank two and no depth-one recursion. Its
    # estimator consumes the most-inner NB entry, as the Rust certifier does.
    ("r5", 1 << 9, 5_547_059.4504792355, 230_762.43129461087,
     15_572_725.118706232, 31_090_130.645210836, 4, 146.0, 131.0, None),
]

SIMPLE = {
    "label": "r6-simple",
    "height": 1 << 10,
    "width": 4,
    "witness_raw": 33_050_090.278953552,
    "projection_raw": 63_268_938.28014243,
    "basic_bits": 136.0,
}


def sumcheck_row(source: tuple) -> dict:
    (label, height, nb_outer, nb_inner, folded_raw, projection_raw,
     next_width, basic_bits, recursive_outer_bits, recursive_inner_bits) = source

    folded_bound = folded_raw * NORM_MARGIN
    extracted_bound = folded_bound * SPECTRAL_OP_NORM_SAFE_BOUND * EXTRACTION_SLACK
    projection_bound = None if projection_raw is None else projection_raw * NORM_MARGIN
    argued_bound = None if projection_bound is None else projection_bound / JL_ALPHA_RP
    selected_bound = extracted_bound if argued_bound is None else max(extracted_bound, argued_bound)
    gate_lhs = None if argued_bound is None else next_width * argued_bound**2
    gate_rhs = MOD_Q / 2.0

    # A terminal recursive layer consumes the most-inner bound directly.
    recursive_outer_bound = (nb_inner if recursive_inner_bits is None else nb_outer) * NORM_MARGIN
    return {
        "label": label,
        "witness_height": height,
        "basic_rank": 7,
        "folded_recomposed_bound": folded_bound,
        "projection_recomposed_bound": projection_bound,
        "selected_sis_bound": selected_bound,
        "basic_estimator_bits": basic_bits,
        "recursive_outer_rank": 2 if recursive_inner_bits is None else 4,
        "recursive_outer_bound": recursive_outer_bound,
        "recursive_outer_estimator_bits": recursive_outer_bits,
        "recursive_inner_rank": None if recursive_inner_bits is None else 1,
        "recursive_inner_bound": None if recursive_inner_bits is None else nb_inner * NORM_MARGIN,
        "recursive_inner_estimator_bits": recursive_inner_bits,
        "gate_width": next_width,
        "centered_gate_lhs": gate_lhs,
        "centered_gate_rhs": None if gate_lhs is None else gate_rhs,
        "centered_gate_holds": None if gate_lhs is None else gate_lhs < gate_rhs,
        "bound_provenance": "verifier-enforced accepted-witness bound",
        "completeness_source": "empirical maximum plus 2% margin",
    }


def simple_row() -> dict:
    witness_bound = SIMPLE["witness_raw"] * NORM_MARGIN
    projection_bound = SIMPLE["projection_raw"] * NORM_MARGIN
    argued_bound = projection_bound / JL_ALPHA_RP
    selected_bound = max(
        witness_bound * SPECTRAL_OP_NORM_SAFE_BOUND * EXTRACTION_SLACK,
        argued_bound,
    )
    lhs = SIMPLE["width"] * argued_bound**2
    rhs = MOD_Q / 2.0
    return {
        "label": SIMPLE["label"],
        "witness_height": SIMPLE["height"],
        "basic_rank": 7,
        "selected_sis_bound": selected_bound,
        "basic_estimator_bits": SIMPLE["basic_bits"],
        "gate_width": SIMPLE["width"],
        "centered_gate_lhs": lhs,
        "centered_gate_rhs": rhs,
        "centered_gate_holds": lhs < rhs,
        "bound_provenance": "verifier-enforced accepted-witness bound",
        "completeness_source": "empirical maximum plus 2% margin",
    }


def main() -> None:
    rows = [sumcheck_row(row) for row in ROWS] + [simple_row()]
    for row in rows:
        assert row["selected_sis_bound"] < (MOD_Q - 1) / 2
        assert row["basic_estimator_bits"] >= TARGET_BITS
        if row.get("recursive_outer_estimator_bits") is not None:
            assert row["recursive_outer_estimator_bits"] >= TARGET_BITS
        if row.get("recursive_inner_estimator_bits") is not None:
            assert row["recursive_inner_estimator_bits"] >= TARGET_BITS
        if row["centered_gate_holds"] is not None:
            assert row["centered_gate_holds"]

    result = {
        "status": "p28 capacity, empirical completeness calibration, centered gates, and 128-bit SIS certification pass",
        "scope": "q_perf repository performance line only",
        "provenance": {
            "completeness": "deterministic tau=32/tau=34 and combined-boundary observations; not a tail proof",
            "security_norm": "deterministic verifier predicate, hence a malicious-prover bound",
            "estimator": "pinned classical Euclidean-SIS MATZOV/GSA Rust port",
            "estimator_reproduction": "cargo +nightly test registered_p28_p30_and_exact_bounds_are_128_bit_certified --lib --features debug-hardness -- --nocapture",
        },
        "constants": {
            "q_perf": MOD_Q,
            "q_over_two": MOD_Q / 2.0,
            "norm_margin": NORM_MARGIN,
            "jl_alpha_rp": JL_ALPHA_RP,
            "spectral_operator_norm_bound": SPECTRAL_OP_NORM_SAFE_BOUND,
            "extraction_slack": EXTRACTION_SLACK,
            "target_bits": TARGET_BITS,
        },
        "rows": rows,
    }
    output = Path(__file__).resolve().parent / "generated" / "fixed_weight_hardness_audit.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output}")
    for row in rows:
        print(
            row["label"],
            f"basic_bits={row['basic_estimator_bits']}",
            f"recursive_outer_bits={row.get('recursive_outer_estimator_bits')}",
            f"recursive_inner_bits={row.get('recursive_inner_estimator_bits')}",
            f"centered_gate={row['centered_gate_holds']}",
        )


if __name__ == "__main__":
    main()
