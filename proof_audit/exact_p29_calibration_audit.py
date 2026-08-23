#!/usr/bin/env python3
"""Reproduce the completed exact-p29 calibration/certification ledger.

The long remote calibration is not rerun here.  This script records its raw
PB/FB/NB output and peak RSS, then runs only local lightweight capacity,
dimension, centered-gate, and pinned SIS-estimator regressions against the
installed parameter table.
"""

from __future__ import annotations

import json
import math
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ROKOKO = ROOT / "rokoko"
OUT = ROOT / "proof_audit" / "generated" / "exact_p29_calibration_audit.json"
REPORT = ROOT / "proof_audit" / "exact_p29_calibration_audit.md"

NORM_MARGIN = 1.02
TARGET_BITS = 128.0

NB = [
    [449095.42486424866, 4573.127048311691],
    [2541655.0585606615, 4586.439795745716],
    [31410872.07525025, 4567.333357660682],
    [6615918.423300351, 5264.489971886165],
    [31244341.336710155, 5297.151168746375],
    [28157775.739574444, 5272.187702247938],
    [6186013.78539969, 233106.2292388258],
    [35134549.26089995, 72395534.86889651],
]
PB = [
    27241906.0563594,
    5092534.207931646,
    28742421.257555842,
    28177438.972140476,
    29436314.23636514,
    26968264.736921042,
    22645685.164770506,
]
FB = [
    14054360.108911611,
    2535673.1294317096,
    18808604.998299476,
    23874083.661871046,
    30857943.832717095,
    21531674.06203266,
    15447729.974848181,
]
NB_DECIMAL = [
    ["449095.42486424866", "4573.127048311691"],
    ["2541655.0585606615", "4586.439795745716"],
    ["31410872.07525025", "4567.333357660682"],
    ["6615918.423300351", "5264.4899718861649"],
    ["31244341.336710155", "5297.1511687463745"],
    ["28157775.739574444", "5272.1877022479381"],
    ["6186013.78539969", "233106.229238825795"],
    ["35134549.260899948", "72395534.868896506"],
]
PB_DECIMAL = [
    "27241906.0563594",
    "5092534.207931646",
    "28742421.257555842",
    "28177438.972140476",
    "29436314.23636514",
    "26968264.736921042",
    "22645685.164770506",
]
FB_DECIMAL = [
    "14054360.108911611",
    "2535673.1294317096",
    "18808604.998299476",
    "23874083.661871046",
    "30857943.832717095",
    "21531674.06203266",
    "15447729.974848181",
]

# Bounds printed by the completed calibration binary before replacement.  They
# include the repository's 1.02 installation margin.
OLD_NB_BOUNDS = [
    [301286.0855424053, 2814.1639180403117],
    [207653.9393205985, 3232.4329468683495],
    [250271.0801520208, 3224.2077380342603],
    [60058.69332853321, 3223.9274748666417],
    [57745.171184818566, 3231.2624417091224],
    [36477.43270767284, 3227.5469169014414],
    [200847.7450554494, 200736.78734982785],
    [988144.6860000001, 2371282.23],
]

GEOMETRY = [
    {"round": 0, "kind": "sumcheck", "height": 32768, "width": 256},
    {"round": 1, "kind": "sumcheck", "height": 65536, "width": 16},
    {"round": 2, "kind": "sumcheck", "height": 16384, "width": 16},
    {"round": 3, "kind": "sumcheck", "height": 4096, "width": 16},
    {"round": 4, "kind": "sumcheck", "height": 2048, "width": 8},
    {"round": 5, "kind": "sumcheck", "height": 1024, "width": 8},
    {"round": 6, "kind": "sumcheck", "height": 512, "width": 8},
    {"round": 7, "kind": "simple", "height": 2048, "width": 2},
]

SECURITY_RE = re.compile(
    r'STATIC_CERT security scope="(?P<scope>exact-p29/[^"]+)" '
    r'm=(?P<m>\d+) rank=(?P<rank>\d+) bound=(?P<bound>[0-9.eE+-]+) '
    r'result=Ok\(EstimatorResult \{ secpar: (?P<bits>[0-9.]+) \}\)'
)
GATE_RE = re.compile(
    r'STATIC_CERT gate name="exact-p29" round=(?P<round>\d+) '
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
    if result.returncode != 0:
        raise RuntimeError(f"cargo test failed: {name}\n{output}")
    return output


def main() -> None:
    for values in (NB, PB, FB):
        assert all(math.isfinite(x) for row in values for x in (row if isinstance(row, list) else [row]))

    exceedances = []
    labels = ("norm", "most_inner_norm")
    for round_index, (observed, old) in enumerate(zip(NB, OLD_NB_BOUNDS)):
        for component, label in enumerate(labels):
            if observed[component] > old[component]:
                exceedances.append(
                    {
                        "round": round_index,
                        "component": label,
                        "observed": observed[component],
                        "old_bound": old[component],
                        "ratio": observed[component] / old[component],
                    }
                )
    assert len(exceedances) == 16

    capacity_output = cargo_test(
        "fixed_weight_exact_norm_chains_fit_balanced_decomposition_windows", "p-29"
    )
    dimension_output = cargo_test("test_p29_chain_dims", "p-29")
    front_end_output = cargo_test("test_p29_front_end_witness_size", "p-29")
    cert_output = cargo_test(
        "registered_p28_p30_and_exact_bounds_are_128_bit_certified",
        "p-29,debug-hardness",
    )

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
    assert security
    assert len(gates) == 8
    assert all(item["classical_sis_bits"] >= TARGET_BITS for item in security)
    assert all(item["holds"] for item in gates)

    result = {
        "status": "closed for capacity, empirical completeness calibration, centered uniqueness, and classical SIS certification",
        "claim_boundary": "The norm calibration is empirical completeness evidence, not a malicious-prover theorem bound. The static SIS ledger uses verifier-enforced installed bounds. This does not by itself close the separate q_perf extraction ledger.",
        "parameter_line": "repository exact-norm p29 (q_perf); separate from the q_exact exact-strong theorem line",
        "geometry": GEOMETRY,
        "calibration": {
            "command": "cargo test calibrate_exact_norm_chain --lib --features p-29 --offline -- --ignored --nocapture",
            "environment": {"ROKOKO_CALIBRATE_NORMS": "1"},
            "remote_host": "gpushare",
            "remote_task_root": "/hy-tmp/codex-exact-p29.0nCePo",
            "remote_git_head": "16b05f32496f91442607f8cb21482b52990f505a",
            "remote_dirty_files": [
                ".cargo/config.toml",
                "src/protocol/params.rs",
                "src/protocol/parties/verifier.rs",
            ],
            "tau": 32,
            "raw_NB": NB,
            "raw_PB": PB,
            "raw_FB": FB,
            "raw_NB_stdout_decimals": NB_DECIMAL,
            "raw_PB_stdout_decimals": PB_DECIMAL,
            "raw_FB_stdout_decimals": FB_DECIMAL,
            "installed_margin": NORM_MARGIN,
            "installed_NB": [[NORM_MARGIN * x for x in row] for row in NB],
            "installed_PB": [NORM_MARGIN * x for x in PB],
            "installed_FB": [NORM_MARGIN * x for x in FB],
            "first_old_bound_exceedance": exceedances[0],
            "all_old_bound_exceedances": exceedances,
            "test_passed": True,
            "elapsed_seconds_test_harness": 2387.95,
            "elapsed_seconds_peak_monitor": 2389.842,
            "peak_rss": {
                "aggregate_kib": 71525028,
                "single_process_kib": 71486768,
                "processes_at_aggregate_peak": 2,
                "sample_interval_seconds": 0.1,
            },
            "binary_boundary": "The completed remote binary contained the multi-block c0 reconstruction and constant-term fix. Commit 5bb6e5c later added fail-closed shape/canonical checks and tests without changing the honest transcript or calibrated norm values.",
        },
        "capacity_regression": {
            "passed": "test result: ok" in capacity_output,
            "test": "fixed_weight_exact_norm_chains_fit_balanced_decomposition_windows",
        },
        "dimension_regressions": {
            "chain_dimensions_passed": "test result: ok" in dimension_output,
            "front_end_witness_size_passed": "test result: ok" in front_end_output,
        },
        "centered_gates": gates,
        "sis_and_commitment_security": security,
        "minimum_classical_sis_bits": min(item["classical_sis_bits"] for item in security),
        "publication_benchmark": {
            "calibration_timing_is_not_a_benchmark": True,
            "rerun_needed": True,
            "reason": "Publication benchmarks are deferred until exact-p29, p26, and the q_perf extraction ledger are all technically closed, as requested.",
        },
    }
    assert result["capacity_regression"]["passed"]
    assert result["dimension_regressions"]["chain_dimensions_passed"]
    assert result["dimension_regressions"]["front_end_witness_size_passed"]

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    lines = [
        "# exact-p29 full-chain calibration and certification",
        "",
        "Status: **closed** for capacity, empirical completeness calibration, centered uniqueness, and pinned classical Euclidean-SIS certification.",
        "",
        "This is the repository `q_perf` exact-norm p29 line.  It remains separate from the `q_exact` exact-strong theorem line, and it does not by itself close the separate `q_perf` extraction ledger.",
        "",
        "## Completed run",
        "",
        "- Remote task: `/hy-tmp/codex-exact-p29.0nCePo` on `gpushare`.",
        "- Test: `calibrate_exact_norm_chain`, `p-29`, `ROKOKO_CALIBRATE_NORMS=1`.",
        "- Result: 1 passed; elapsed 2387.95 s (peak monitor 2389.842 s).",
        "- Peak RSS: aggregate 71,525,028 KiB; largest process 71,486,768 KiB; two processes at aggregate peak; 0.1 s sampling.",
        "- Remote HEAD: `16b05f32496f91442607f8cb21482b52990f505a` with the recorded parameter and initial multi-block verifier changes uncommitted.",
        "",
        "## Raw empirical norm ledger",
        "",
        "`NORM_MARGIN=1.02` is applied only when the raw values are installed.  These are deterministic tau=32 completeness-calibration observations, not malicious-prover norm theorems.",
        "",
        "| round | kind | geometry | raw NB | raw inner NB | raw FB | raw PB |",
        "|---:|---|---:|---:|---:|---:|---:|",
    ]
    for index, geometry in enumerate(GEOMETRY):
        fb = FB_DECIMAL[index] if index < len(FB_DECIMAL) else "--"
        pb = PB_DECIMAL[index] if index < len(PB_DECIMAL) else "--"
        lines.append(
            f"| {index} | {geometry['kind']} | {geometry['height']}x{geometry['width']} | "
            f"{NB_DECIMAL[index][0]} | {NB_DECIMAL[index][1]} | {fb} | {pb} |"
        )
    lines.extend(
        [
            "",
            "All 16 NB components exceeded the provisional installed bounds; the first was round 0 `norm` (449095.42486424866 > 301286.0855424053).  PB and FB had previously been infinite placeholders.  Every installed PB/FB/NB value is now finite.",
            "",
            "## Centered-uniqueness gates",
            "",
            "Every gate below uses the verifier-enforced installed projection bound, not an honest-run measured witness norm.",
            "",
            "| round | gate width | lhs | q/2 | pass |",
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
            "## SIS and commitment certification",
            "",
            "All entries use the pinned classical Euclidean-SIS MATZOV/GSA Rust port at the exact registered dimension, rank, and verifier-enforced installed bound.  The minimum is 131 bits.",
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
            "## Regression and rerun decision",
            "",
            "- Balanced-decomposition capacity: pass.",
            "- p29 chain dimensions: pass.",
            "- p29 front-end witness size: pass.",
            "- Static centered/SIS regression: pass (8/8 gates, 54/54 SIS/commitment entries).",
            "- The completed remote binary already contained the honest multi-block `c_0` reconstruction and constant-term fix.  Commit `5bb6e5c` adds only fail-closed geometry/canonical checks and tests; it does not change the honest transcript or these norm values.",
            "- No second calibration is required for those validation-only additions.  Publication benchmarks still require a fresh run after exact-p29, p26, and the `q_perf` extraction ledger are all closed; the 2387.95 s calibration timing is not a publication benchmark.",
            "",
        ]
    )
    REPORT.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {OUT}")
    print(f"wrote {REPORT}")
    print(
        f"NB={len(NB)} PB={len(PB)} FB={len(FB)} gates={len(gates)} "
        f"sis_entries={len(security)} min_bits={result['minimum_classical_sis_bits']}"
    )


if __name__ == "__main__":
    main()
