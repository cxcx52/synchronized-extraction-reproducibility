#!/usr/bin/env python3
"""Recompute the deterministic p28 fixed-weight hardness-audit ledger.

The norms below are the observations emitted by the Rust full-chain run on
2026-08-23.  This script checks every algebraic conversion from decomposition
norms to extraction/JL bounds and records the independently rerun Rust MATZOV
outputs.  Observed norms are benchmark diagnostics, not theorem-level tails.
"""

from __future__ import annotations

import json
import math
from pathlib import Path


MOD_Q = 1_125_899_906_839_937
JL_ALPHA_RP = math.sqrt(30.0)
SPECTRAL_OP_NORM_SAFE_BOUND = 32.0
EXTRACTION_SLACK = 8.0


ROWS = [
    {
        "label": "root",
        "witness_height": 1 << 14,
        "current_rank": 6,
        "decomposed_folded_witness_l2": 154_219.21643556617,
        "witness_base_log": 8,
        "witness_chunks": 4,
        "current_estimator_bits": None,
        "current_estimator_status": "trivially easy: bound >= (q-1)/2",
        "minimum_rank_ge_128": None,
        "minimum_rank_bits": None,
    },
    {
        "label": "p1",
        "witness_height": 1 << 13,
        "current_rank": 5,
        "decomposed_folded_witness_l2": 298_029.3883059186,
        "witness_base_log": 10,
        "witness_chunks": 2,
        "projection_width": 32,
        "decomposed_projection_l2": 177_162.64274671453,
        "observed_recomposed_projection_l2": 1_872_113.90977499,
        "projection_base_log": 10,
        "projection_chunks": 2,
        "current_estimator_bits": 79.0,
        "current_estimator_status": "ok",
        "minimum_rank_ge_128": 8,
        "minimum_rank_bits": 128.0,
    },
    {
        "label": "p2",
        "witness_height": 1 << 10,
        "current_rank": 5,
        "decomposed_folded_witness_l2": 168_981.72171569327,
        "witness_base_log": 11,
        "witness_chunks": 2,
        "projection_width": 32,
        "decomposed_projection_l2": 90_411.95998870945,
        "observed_recomposed_projection_l2": 3_912_301.738279398,
        "projection_base_log": 10,
        "projection_chunks": 2,
        "current_estimator_bits": 78.0,
        "current_estimator_status": "ok",
        "minimum_rank_ge_128": 9,
        "minimum_rank_bits": 145.0,
    },
    {
        "label": "p3",
        "witness_height": 1 << 8,
        "current_rank": 4,
        "decomposed_folded_witness_l2": 102_959.85187440782,
        "witness_base_log": 11,
        "witness_chunks": 2,
        "projection_width": 8,
        "decomposed_projection_l2": 48_573.25967031655,
        "observed_recomposed_projection_l2": 2_178_640.3781441306,
        "projection_base_log": 10,
        "projection_chunks": 2,
        "current_estimator_bits": 65.0,
        "current_estimator_status": "ok",
        "minimum_rank_ge_128": 8,
        "minimum_rank_bits": 133.0,
    },
    {
        "label": "p4",
        "witness_height": 1 << 9,
        "current_rank": 4,
        "decomposed_folded_witness_l2": 72_273.40281182283,
        "witness_base_log": 10,
        "witness_chunks": 2,
        "projection_width": 8,
        "decomposed_projection_l2": 33_027.58182186519,
        "observed_recomposed_projection_l2": 1_319_349.9681668999,
        "projection_base_log": 10,
        "projection_chunks": 2,
        "current_estimator_bits": 71.0,
        "current_estimator_status": "ok",
        "minimum_rank_ge_128": 8,
        "minimum_rank_bits": 146.0,
    },
    {
        "label": "p5",
        "witness_height": 1 << 8,
        "current_rank": 3,
        "decomposed_folded_witness_l2": 51_323.84727395249,
        "witness_base_log": 10,
        "witness_chunks": 2,
        "projection_width": 4,
        "decomposed_projection_l2": 18_353.200293136888,
        "observed_recomposed_projection_l2": 918_454.6475787468,
        "projection_base_log": 10,
        "projection_chunks": 2,
        "current_estimator_bits": 55.0,
        "current_estimator_status": "ok",
        "minimum_rank_ge_128": 7,
        "minimum_rank_bits": 130.0,
    },
    {
        "label": "simple",
        "witness_height": 1 << 8,
        "current_rank": 4,
        "observed_folded_witness_l2": 1_211_023.1894237204,
        "extracted_witness_bound": 310_021_936.4924724,
        "projection_argued_bound": 493_306.23289903125,
        "current_estimator_bits": 104.0,
        "current_estimator_status": "ok",
        "minimum_rank_ge_128": 5,
        "minimum_rank_bits": 132.0,
    },
]


def recomposition_l2_operator_norm(base_log: int, chunks: int) -> float:
    return math.sqrt(sum(2.0 ** (2 * base_log * i) for i in range(chunks)))


def audit_row(source: dict) -> dict:
    row = dict(source)
    if row["label"] != "simple":
        witness_operator = recomposition_l2_operator_norm(
            row["witness_base_log"], row["witness_chunks"]
        )
        extracted_bound = (
            row["decomposed_folded_witness_l2"]
            * witness_operator
            * SPECTRAL_OP_NORM_SAFE_BOUND
            * EXTRACTION_SLACK
        )
        row["witness_recomposition_operator_norm"] = witness_operator
        row["extracted_witness_bound"] = extracted_bound

    row["sis_nontrivial"] = row["extracted_witness_bound"] < (MOD_Q - 1) / 2

    if "projection_width" in row:
        projection_operator = recomposition_l2_operator_norm(
            row["projection_base_log"], row["projection_chunks"]
        )
        recomposed_projection_bound = row["decomposed_projection_l2"] * projection_operator
        argued_bound = recomposed_projection_bound / JL_ALPHA_RP
        gate_lhs = row["projection_width"] * argued_bound**2
        gate_rhs = MOD_Q / 2
        observed_argued_bound = row["observed_recomposed_projection_l2"] / JL_ALPHA_RP
        observed_gate_lhs = row["projection_width"] * observed_argued_bound**2
        row.update(
            {
                "projection_recomposition_operator_norm": projection_operator,
                "recomposed_projection_bound": recomposed_projection_bound,
                "projection_argued_bound": argued_bound,
                "uniqueness_gate_lhs": gate_lhs,
                "uniqueness_gate_rhs": gate_rhs,
                "uniqueness_holds": gate_lhs < gate_rhs,
                "observed_projection_argued_bound": observed_argued_bound,
                "observed_uniqueness_gate_lhs": observed_gate_lhs,
                "observed_uniqueness_holds": observed_gate_lhs < gate_rhs,
            }
        )
    return row


def main() -> None:
    audited = [audit_row(row) for row in ROWS]
    result = {
        "status": "current fixed-weight p28 parameter line fails the corrected hardness audit",
        "provenance": {
            "norms": "deterministic tau=32 full-chain Rust audit, 2026-08-23",
            "estimator": "pinned classical Euclidean-SIS MATZOV/GSA Rust port",
            "warning": "observed norms are benchmark diagnostics, not theorem-level tail bounds",
        },
        "constants": {
            "q": MOD_Q,
            "q_minus_one_over_two": (MOD_Q - 1) // 2,
            "jl_alpha_rp": JL_ALPHA_RP,
            "spectral_operator_norm_bound": SPECTRAL_OP_NORM_SAFE_BOUND,
            "extraction_slack": EXTRACTION_SLACK,
        },
        "recursive_commitment_findings": [
            {
                "layer": "p1",
                "scopes": ["commitment", "opening", "projection image"],
                "depth": 0,
                "current_rank": 2,
                "observed_length_bound": 346_997.79747571883,
                "current_estimator_bits": 122.0,
                "meets_128_bit_target": False,
            }
        ],
        "rows": audited,
    }
    output = Path(__file__).resolve().parent / "generated" / "fixed_weight_hardness_audit.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output}")
    for row in audited:
        gate = row.get("uniqueness_holds")
        gate_fields = [] if gate is None else [f"uniqueness_holds={gate}"]
        print(
            row["label"],
            f"sis_nontrivial={row['sis_nontrivial']}",
            f"bits={row['current_estimator_bits']}",
            f"minimum_rank_ge_128={row['minimum_rank_ge_128']}",
            *gate_fields,
        )


if __name__ == "__main__":
    main()
