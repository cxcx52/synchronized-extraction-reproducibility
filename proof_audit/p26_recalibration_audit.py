#!/usr/bin/env python3
"""Reproduce the redesigned performance-p26 parameter certification."""

from __future__ import annotations

import json
import math
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ROKOKO = ROOT / "rokoko"
JSON_OUT = ROOT / "proof_audit" / "generated" / "p26_recalibration_audit.json"
REPORT_OUT = ROOT / "proof_audit" / "p26_recalibration_audit.md"
NORM_MARGIN = 1.02
TARGET_BITS = 128.0

GEOMETRY = [
    {"round": 0, "kind": "sumcheck", "height": 8192, "width": 128},
    {"round": 1, "kind": "sumcheck", "height": 8192, "width": 4},
    {"round": 2, "kind": "sumcheck", "height": 2048, "width": 16},
    {"round": 3, "kind": "sumcheck", "height": 1024, "width": 8},
    {"round": 4, "kind": "sumcheck", "height": 512, "width": 8},
    {"round": 5, "kind": "sumcheck", "height": 512, "width": 8},
    {"round": 6, "kind": "simple", "height": 1024, "width": 4},
]

RUNS = {
    "tau32": {
        "features": ["p-26"],
        "elapsed_seconds": 66.13,
        "NB": [
            [9688355.015184827, 3732.067657478894],
            [31224594.71942014, 4567.8971091739795],
            [11673670.45703334, 5290.237707324691],
            [22962363.566686466, 5313.507504464447],
            [11213778.655962361, 5245.613024232725],
            [5628297.382666271, 230279.29944091805],
            [31665159.852133103, 59941428.41463394],
        ],
        "PB": [
            0.0,
            28055846.050965242,
            33437465.32426311,
            20829988.632926688,
            9280530.616271734,
            8589523.192291467,
        ],
        "FB": [
            28752557.389113426,
            2736871.2124451525,
            30766569.356588393,
            12505865.623022743,
            21133576.557251874,
            26726407.77466259,
        ],
    },
    "tau34": {
        "features": ["p-26", "challenge-weight-34"],
        "elapsed_seconds": 61.03,
        "NB": [
            [9682732.507398777, 3732.067657478894],
            [28232326.449292593, 4574.773764898107],
            [17017102.188843023, 5261.480970221217],
            [30482311.339410435, 5258.504635350244],
            [31225883.499218643, 5250.408079378211],
            [5624810.456691941, 230952.77182359167],
            [32972167.774389204, 61506376.39596541],
        ],
        "PB": [
            0.0,
            24689426.714048628,
            11895652.13082057,
            28906270.144289907,
            30584859.105609626,
            27562308.781788617,
        ],
        "FB": [
            11086329.333465653,
            13360008.664023874,
            25088939.001246884,
            30884390.879491862,
            32480282.823585525,
            19260217.99323359,
        ],
    },
}

# The boundary regression executes two prefixes without reseeding between them.
# Its second prefix therefore exercises a different deterministic completeness
# stream than the standalone full-chain calibration.  These observations are
# kept separate so the raw provenance is not mistaken for another full chain.
BOUNDARY_RUNS = {
    "tau32": {
        "features": ["p-26"],
        "elapsed_seconds": 123.44,
        "prefixes": [
            {
                "cut": 3,
                "rounds": [
                    {"round": 0, "NB": [9688355.015184827, 3732.067657478894], "FB": 28752557.389113426},
                    {"round": 1, "NB": [31224594.71942014, 4567.8971091739795], "FB": 2736871.2124451525, "PB": 28055846.050965242},
                    {"round": 2, "NB": [11673670.45703334, 5290.237707324691], "FB": 30766569.356588393, "PB": 33437465.32426311},
                ],
            },
            {
                "cut": 4,
                "rounds": [
                    {"round": 0, "NB": [9687717.61608951, 3732.0274650650686], "FB": 22256752.37641458},
                    {"round": 1, "NB": [31955598.685107652, 4581.529984623041], "FB": 33529894.410260584, "PB": 28870050.818056054},
                    {"round": 2, "NB": [21237279.230148338, 5261.1506346045635], "FB": 19644338.345788207, "PB": 17502201.64040313},
                    {"round": 3, "NB": [16608669.105184497, 5265.666529509821], "FB": 21629965.722183265, "PB": 13491278.466594633},
                ],
            },
        ],
    },
    "tau34": {
        "features": ["p-26", "challenge-weight-34"],
        "elapsed_seconds": 119.62,
        "prefixes": [
            {
                "cut": 3,
                "rounds": [
                    {"round": 0, "NB": [9682732.507398777, 3732.067657478894], "FB": 11086329.333465653},
                    {"round": 1, "NB": [28232326.449292593, 4574.773764898107], "FB": 13360008.664023874, "PB": 24689426.714048628},
                    {"round": 2, "NB": [17017102.188843023, 5261.480970221217], "FB": 25088939.001246884, "PB": 11895652.13082057},
                ],
            },
            {
                "cut": 4,
                "rounds": [
                    {"round": 0, "NB": [9680260.032508373, 3732.0274650650686], "FB": 31332894.601975046},
                    {"round": 1, "NB": [30587062.9245337, 4618.282797750697], "FB": 13498226.549771382, "PB": 27352919.130654283},
                    {"round": 2, "NB": [19215119.590281375, 5280.345159172835], "FB": 18252126.684876807, "PB": 14977896.917536488},
                    {"round": 3, "NB": [13868234.124225406, 5292.0804037731705], "FB": 27729410.84855093, "PB": 9939771.693030931},
                ],
            },
        ],
    },
}

SECURITY_RE = re.compile(
    r'STATIC_CERT security scope="(?P<scope>p26/[^"]+)" '
    r'm=(?P<m>\d+) rank=(?P<rank>\d+) bound=(?P<bound>[0-9.eE+-]+) '
    r'result=Ok\(EstimatorResult \{ secpar: (?P<bits>[0-9.]+) \}\)'
)
GATE_RE = re.compile(
    r'STATIC_CERT gate name="p26" round=(?P<round>\d+) '
    r'width=(?P<width>\d+) lhs=(?P<lhs>[0-9.eE+-]+) '
    r'rhs=(?P<rhs>[0-9.eE+-]+) holds=(?P<holds>true|false)'
)


def cargo_test(name: str, features: str) -> str:
    result = subprocess.run(
        [
            "cargo",
            "+nightly",
            "test",
            name,
            "--lib",
            "--features",
            features,
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
        raise RuntimeError(f"cargo test failed: {name}\n{output}")
    return output


def coordinatewise_max() -> tuple[list[list[float]], list[float], list[float]]:
    nb = [
        [
            max(run["NB"][round_index][component] for run in RUNS.values())
            for component in range(2)
        ]
        for round_index in range(7)
    ]
    pb = [max(run["PB"][index] for run in RUNS.values()) for index in range(6)]
    fb = [max(run["FB"][index] for run in RUNS.values()) for index in range(6)]
    for boundary_run in BOUNDARY_RUNS.values():
        for prefix in boundary_run["prefixes"]:
            for observation in prefix["rounds"]:
                round_index = observation["round"]
                nb[round_index][0] = max(nb[round_index][0], observation["NB"][0])
                nb[round_index][1] = max(nb[round_index][1], observation["NB"][1])
                if "FB" in observation and round_index < len(fb):
                    fb[round_index] = max(fb[round_index], observation["FB"])
                if "PB" in observation and round_index > 0:
                    pb[round_index] = max(pb[round_index], observation["PB"])
    return nb, pb, fb


def main() -> None:
    nb, pb, fb = coordinatewise_max()
    assert all(math.isfinite(x) for row in nb for x in row)
    assert all(math.isfinite(x) for x in pb + fb)

    capacity = cargo_test(
        "fixed_weight_plain_chains_fit_balanced_decomposition_windows", "p-26"
    )
    dimensions = cargo_test("test_p_snark_chain_dims", "p-26")
    cert_output = cargo_test(
        "registered_p26_bounds_are_128_bit_certified", "p-26,debug-hardness"
    )

    gates = [
        {
            "round": int(match.group("round")),
            "gate_width": int(match.group("width")),
            "lhs": float(match.group("lhs")),
            "rhs": float(match.group("rhs")),
            "holds": match.group("holds") == "true",
            "bound_provenance": "verifier-enforced installed projection bound",
        }
        for match in GATE_RE.finditer(cert_output)
    ]
    security = [
        {
            "scope": match.group("scope"),
            "m": int(match.group("m")),
            "rank": int(match.group("rank")),
            "certified_bound": float(match.group("bound")),
            "classical_sis_bits": float(match.group("bits")),
            "estimator": "pinned classical Euclidean-SIS MATZOV/GSA Rust port",
            "bound_provenance": "verifier-enforced installed bound",
        }
        for match in SECURITY_RE.finditer(cert_output)
    ]
    assert len(gates) == 6 and all(gate["holds"] for gate in gates)
    assert security and all(item["classical_sis_bits"] >= TARGET_BITS for item in security)

    result = {
        "status": "closed for capacity, empirical completeness calibration, centered uniqueness, and classical SIS certification",
        "claim_boundary": "The tau=32/tau=34 norms are empirical completeness observations. Malicious-prover SIS certification uses the separately installed verifier-enforced bounds. The q_perf extraction ledger remains a separate task.",
        "parameter_line": "repository performance p26 over q_perf; separate from q_exact",
        "geometry": GEOMETRY,
        "geometry_change": {
            "old": "8192x128 -> 8192x4 -> 1024x32 -> 512x16 -> 512x8 -> 512x8 -> 1024x4",
            "new": "8192x128 -> 8192x4 -> 2048x16 -> 1024x8 -> 512x8 -> 512x8 -> 1024x4",
            "capacity_preserved": True,
        },
        "calibration_runs": RUNS,
        "boundary_regression_runs": BOUNDARY_RUNS,
        "coordinatewise_maxima": {"NB": nb, "PB": pb, "FB": fb},
        "installed_margin": NORM_MARGIN,
        "installed_bounds": {
            "NB": [[NORM_MARGIN * x for x in row] for row in nb],
            "PB": [NORM_MARGIN * x for x in pb],
            "FB": [NORM_MARGIN * x for x in fb],
        },
        "capacity_test_passed": "test result: ok" in capacity,
        "dimension_test_passed": "test result: ok" in dimensions,
        "centered_gates": gates,
        "sis_and_commitment_security": security,
        "minimum_classical_sis_bits": min(item["classical_sis_bits"] for item in security),
        "publication_benchmark": {
            "rerun_needed": True,
            "reason": "The p26 registered geometry changed; calibration timings are not publication benchmarks.",
        },
    }
    assert result["capacity_test_passed"] and result["dimension_test_passed"]

    JSON_OUT.parent.mkdir(parents=True, exist_ok=True)
    JSON_OUT.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")

    lines = [
        "# performance-p26 geometry, calibration, and certification",
        "",
        "Status: **closed** for balanced capacity, empirical tau=32/tau=34 completeness calibration, centered uniqueness, and pinned classical Euclidean-SIS certification.",
        "",
        "This is the repository `q_perf` performance line and remains separate from `q_exact`.  The whole-ring extraction ledger is also separate.",
        "",
        "## Frozen geometry",
        "",
        "`8192x128 -> 8192x4 -> 2048x16 -> 1024x8 -> 512x8 -> 512x8 -> 1024x4`",
        "",
        "The two changed stages preserve their prior element capacities: `1024*32 = 2048*16` and `512*16 = 1024*8`.",
        "",
        "## Empirical calibration",
        "",
        "Each installed raw entry is the coordinatewise maximum of the completed tau=32 and tau=34 full-chain runs and both deterministic prefixes of the corresponding boundary-regression runs.  The registered verifier bound adds the standard 2% margin.  These values are empirical completeness evidence, not a malicious-prover norm theorem.",
        "",
        "| round | geometry | max raw NB | max raw inner NB | max raw FB | max raw PB |",
        "|---:|---:|---:|---:|---:|---:|",
    ]
    for index, geometry in enumerate(GEOMETRY):
        lines.append(
            f"| {index} | {geometry['height']}x{geometry['width']} | {nb[index][0]} | {nb[index][1]} | "
            f"{fb[index] if index < len(fb) else '--'} | {pb[index] if index < len(pb) else '--'} |"
        )
    lines.extend(
        [
            "",
            "The full-chain runs completed in 66.13 s (tau=32) and 61.03 s (tau=34). The two-prefix boundary runs completed in 123.44 s and 119.62 s. All four runs passed their intended prover/verifier paths.",
            "",
            "## Centered-uniqueness gates",
            "",
            "| round | width | lhs | q/2 | pass |",
            "|---:|---:|---:|---:|:---:|",
        ]
    )
    for gate in gates:
        lines.append(
            f"| {gate['round']} | {gate['gate_width']} | {gate['lhs']} | {gate['rhs']} | {'yes' if gate['holds'] else 'no'} |"
        )
    lines.extend(
        [
            "",
            f"All six gates pass.  In particular, the former failures are now round 1 `{gates[0]['lhs']} < q/2` and round 2 `{gates[1]['lhs']} < q/2`.",
            "",
            "## SIS and commitment certification",
            "",
            "| scope | m | rank | certified bound | classical bits |",
            "|---|---:|---:|---:|---:|",
        ]
    )
    for item in security:
        lines.append(
            f"| `{item['scope']}` | {item['m']} | {item['rank']} | {item['certified_bound']} | {item['classical_sis_bits']} |"
        )
    lines.extend(
        [
            "",
            f"All {len(security)} entries pass; the minimum is {result['minimum_classical_sis_bits']} classical bits.",
            "",
            "## Benchmark decision",
            "",
            "The p26 publication benchmark must be rerun because the registered geometry changed.  The calibration and boundary-regression timings above are correctness diagnostics and must not be reported as optimized performance numbers.",
            "",
        ]
    )
    REPORT_OUT.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {JSON_OUT}")
    print(f"wrote {REPORT_OUT}")
    print(
        f"gates={len(gates)} security_entries={len(security)} "
        f"minimum_bits={result['minimum_classical_sis_bits']}"
    )


if __name__ == "__main__":
    main()
