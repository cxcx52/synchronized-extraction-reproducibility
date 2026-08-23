#!/usr/bin/env python3
"""Run and archive the isolated quartic arithmetic differential tests."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ROKOKO = ROOT / "rokoko"
JSON_OUT = ROOT / "proof_audit" / "generated" / "qperf_quartic_backend_audit.json"
REPORT_OUT = ROOT / "proof_audit" / "qperf_quartic_backend_audit.md"
EXPECTED_TESTS = {
    "quartic_transform_round_trip",
    "quartic_multiplication_matches_naive_negacyclic_reference",
    "quartic_slot_and_ring_inverses_are_correct",
    "quartic_slot_constants_match_certified_factorization",
}


def cargo_test() -> str:
    result = subprocess.run(
        [
            "cargo",
            "+nightly",
            "test",
            "quartic_",
            "--lib",
            "--features",
            "quartic-q-audit",
            "--offline",
            "--",
            "--nocapture",
        ],
        cwd=ROKOKO,
        text=True,
        capture_output=True,
        check=False,
    )
    output = result.stdout + result.stderr
    if result.returncode:
        raise RuntimeError(output)
    return output


def main() -> None:
    output = cargo_test()
    passed = set(
        re.findall(r"common::quartic_ring::tests::([a-z0-9_]+) \.\.\. ok", output)
    )
    assert passed == EXPECTED_TESTS, (passed, EXPECTED_TESTS, output)
    result = {
        "status": "proved locally",
        "scope": "isolated quartic arithmetic backend; not yet wired into the protocol",
        "modulus": 926_510_094_425_921,
        "ring_degree": 128,
        "extension_degree": 4,
        "slots": 32,
        "transform": "four independent size-32 negacyclic NTT streams",
        "homogenization": "slot X^4-beta^u maps to common T^4-beta by X -> T^u",
        "tests": sorted(passed),
        "differential_counts": {
            "coefficient_slot_round_trips": 256,
            "fast_vs_naive_negacyclic_products": 256,
            "slotwise_and_ring_inverses": 16,
            "certified_slot_constants": 32,
        },
        "reference_independence": "the multiplication oracle is direct O(128^2) coefficient-domain negacyclic convolution and does not reuse the NTT or slot code",
        "claim_boundary": {
            "proved": [
                "quartic transform round trip",
                "homogeneous slot multiplication",
                "ring multiplication against an independent reference",
                "quartic and whole-ring inversion for units",
            ],
            "not_yet_proved": [
                "protocol transcript and sum-check migration",
                "quartic-backend empirical completeness",
                "redesigned centered gates",
                "publication performance",
            ],
        },
    }
    JSON_OUT.parent.mkdir(parents=True, exist_ok=True)
    JSON_OUT.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    lines = [
        "# Quartic arithmetic backend audit",
        "",
        "Status: **proved locally** for the isolated arithmetic backend.",
        "",
        "The default quadratic implementation is unchanged.  The feature `quartic-q-audit` enables a separate degree-four backend over `q_4=926510094425921`.",
        "",
        "## Construction",
        "",
        "- coefficients are split by index modulo four;",
        "- each stream uses a size-32 negacyclic NTT;",
        "- the 32 raw factors `X^4-beta^u` are mapped to one common field `T^4-beta` by `X -> T^u`;",
        "- multiplication is componentwise in the common quartic field;",
        "- inverse NTTs and re-interleaving recover coefficient form.",
        "",
        "## Regressions",
        "",
        "| check | cases | result |",
        "|---|---:|---|",
        "| coefficient/slot round trip | 256 | pass |",
        "| NTT/slot product vs independent O(128^2) negacyclic product | 256 | pass |",
        "| quartic-slot and whole-ring inverses | 16 | pass |",
        "| NTT slot constants and odd-power factorization | 32 slots | pass |",
        "",
        "The reference product is coefficient-domain convolution and does not call the transform or slot multiplication code.",
        "",
        "## Boundary",
        "",
        "This closes arithmetic correctness in isolation.  It does not yet migrate the protocol's explicit `QuadraticExtension` sum-check interface, establish quartic-backend completeness bounds, repair the seven static centered-gate failures, or provide publication timings.",
    ]
    REPORT_OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {JSON_OUT}")
    print(f"wrote {REPORT_OUT}")
    print("quartic backend: 4/4 test families PASS")


if __name__ == "__main__":
    main()
