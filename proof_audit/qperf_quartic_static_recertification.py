#!/usr/bin/env python3
"""Re-run every registered static gate and SIS estimate on the quartic line.

This is a post-wiring geometry screen over the currently installed verifier
bounds.  It does not claim that old empirically calibrated bounds transfer as
completeness bounds to the new quartic geometry.
"""

from __future__ import annotations

import json
import math
import os
import re
import subprocess
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ROKOKO = ROOT / "rokoko"
Q_QUARTIC = 926_510_094_425_921
JSON_OUT = ROOT / "proof_audit" / "generated" / "qperf_quartic_static_recertification.json"
REPORT_OUT = ROOT / "proof_audit" / "qperf_quartic_static_recertification.md"

SECURITY_RE = re.compile(
    r'STATIC_CERT security scope="(?P<scope>[^"]+)" '
    r'm=(?P<m>\d+) rank=(?P<rank>\d+) bound=(?P<bound>[0-9.eE+-]+) '
    r'result=(?P<result>Ok\(EstimatorResult \{ secpar: (?P<bits>[0-9.]+) \}\)|Err\([^\r\n]+\)) '
    r'modulus=(?P<modulus>\d+)'
)
GATE_RE = re.compile(
    r'STATIC_CERT gate name="(?P<name>[^"]+)" round=(?P<round>\d+) '
    r'width=(?P<width>\d+) lhs=(?P<lhs>[0-9.eE+-]+) '
    r'rhs=(?P<rhs>[0-9.eE+-]+) holds=(?P<holds>true|false)'
)


def cargo_test(name: str) -> str:
    environment = os.environ.copy()
    environment["ROKOKO_AUDIT_MODULUS"] = str(Q_QUARTIC)
    environment["ROKOKO_AUDIT_HARDNESS"] = "1"
    result = subprocess.run(
        [
            "cargo",
            "+nightly",
            "test",
            name,
            "--lib",
            "--features",
            "debug-hardness,quartic-q",
            "--offline",
            "--",
            "--nocapture",
        ],
        cwd=ROKOKO,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    output = result.stdout + result.stderr
    if result.returncode:
        raise RuntimeError(f"cargo test failed: {name}\n{output}")
    return output


def parse_outputs(outputs: list[str]) -> tuple[list[dict], list[dict]]:
    combined = "\n".join(outputs)
    security = []
    for match in SECURITY_RE.finditer(combined):
        bits = None if match.group("bits") is None else float(match.group("bits"))
        security.append(
            {
                "scope": match.group("scope"),
                "m": int(match.group("m")),
                "rank": int(match.group("rank")),
                "certified_bound": float(match.group("bound")),
                "result": match.group("result"),
                "classical_sis_bits": bits,
                "meets_128_bits": bits is not None and bits >= 128.0,
                "modulus": int(match.group("modulus")),
                "bound_provenance": "currently installed verifier-enforced bound",
                "estimator": "pinned classical Euclidean-SIS MATZOV/GSA Rust port",
            }
        )
    gates = []
    for match in GATE_RE.finditer(combined):
        lhs = float(match.group("lhs"))
        rhs = float(match.group("rhs"))
        width = int(match.group("width"))
        max_integer_width = math.floor(width * rhs / lhs)
        max_power_of_two_width = 1 << (max_integer_width.bit_length() - 1)
        gates.append(
            {
                "parameter_line": match.group("name"),
                "round": int(match.group("round")),
                "width": width,
                "lhs": lhs,
                "rhs": rhs,
                "holds": match.group("holds") == "true",
                "lhs_over_rhs": lhs / rhs,
                "max_width_at_same_projection_bound": max_integer_width,
                "max_power_of_two_width_at_same_projection_bound": max_power_of_two_width,
                "bound_provenance": "currently installed verifier-enforced projection bound",
            }
        )
    return security, gates


def summarize(security: list[dict], gates: list[dict]) -> list[dict]:
    names = sorted({row["scope"].split("/", 1)[0] for row in security})
    result = []
    for name in names:
        security_rows = [row for row in security if row["scope"].startswith(name + "/")]
        gate_rows = [row for row in gates if row["parameter_line"] == name]
        result.append(
            {
                "parameter_line": name,
                "security_entries": len(security_rows),
                "minimum_classical_sis_bits": min(
                    row["classical_sis_bits"] for row in security_rows
                ),
                "security_entries_ge_128": sum(row["meets_128_bits"] for row in security_rows),
                "centered_gates": len(gate_rows),
                "centered_gates_passing": sum(row["holds"] for row in gate_rows),
                "failing_rounds": [row["round"] for row in gate_rows if not row["holds"]],
            }
        )
    return result


def write_report(result: dict) -> None:
    all_static_checks_pass = (
        result["security_passing"] == result["security_entries"]
        and result["gates_passing"] == result["gates"]
    )
    status = (
        "**static geometry screen passed; quartic completeness calibration remains required**"
        if all_static_checks_pass
        else "**redesign required**"
    )
    lines = [
        "# Static quartic geometry screen",
        "",
        f"Status: {status}.",
        "",
        "This is a static screen of the quartic-only geometry over the previously installed verifier-enforced bounds at `q_4`; it is not a completeness calibration, final parameter certification, or benchmark.",
        "",
        f"- `q_4 = {Q_QUARTIC}`",
        f"- `q_4/2 = {Q_QUARTIC / 2}`",
        f"- security: {result['security_passing']}/{result['security_entries']} entries at least 128 bits",
        f"- centered gates: {result['gates_passing']}/{result['gates']} pass",
        "",
        "## Quartic-only geometry changes",
        "",
        "| scope | old geometry | quartic geometry | reason |",
        "|---|---:|---:|---|",
        "| `p30/p_2` | `2048 x 32` | `4096 x 16` | close r1 centered gate at unchanged input capacity |",
        "| `p30/p_3` | `512 x 16` | `1024 x 16` | carry the enlarged p2 composed image without widening |",
        "| `p30/p_4` | `512 x 8` | `1024 x 8` | carry the enlarged p3 composed image without widening |",
        "| `exact-p28/p_3` | `512 x 16` | `1024 x 8` | close r3 centered gate at unchanged input capacity |",
        "| terminal (`p26`, `p28`, `p30`, `exact-p26`, `exact-p28`) | `1024 x 4` | `2048 x 2` | close the final centered gate at unchanged input capacity |",
        "",
        "The exact-p29 terminal was already `2048 x 2`; no q4 override changes it.",
        "",
        "## Per-line summary",
        "",
        "| line | centered gates | failing rounds | SIS/commitment entries | minimum bits |",
        "|---|---:|---|---:|---:|",
    ]
    for row in result["summary"]:
        failures = ", ".join(map(str, row["failing_rounds"])) or "none"
        lines.append(
            f"| `{row['parameter_line']}` | {row['centered_gates_passing']}/{row['centered_gates']} | {failures} | {row['security_entries_ge_128']}/{row['security_entries']} | {row['minimum_classical_sis_bits']:.0f} |"
        )
    lines.extend(["", "## Failing centered gates", ""])
    failing_gates = [row for row in result["centered_gates"] if not row["holds"]]
    if failing_gates:
        lines.extend(
            [
                "| line/round | current width | lhs | q4/2 | lhs/rhs | largest power-of-two width at same bound |",
                "|---|---:|---:|---:|---:|---:|",
            ]
        )
        for row in failing_gates:
            lines.append(
                f"| `{row['parameter_line']}/r{row['round']}` | {row['width']} | {row['lhs']:.6f} | {row['rhs']:.1f} | {row['lhs_over_rhs']:.6f} | {row['max_power_of_two_width_at_same_projection_bound']} |"
            )
    else:
        lines.append("None in the post-wiring static screen.")
    lines.extend(
        [
            "",
            "Every changed layout preserves the predecessor capacity required by the generated chain.  The old bounds used here are only a static redesign screen: quartic full-chain calibration must regenerate PB/FB/NB before the gates and estimator rows become final certificates.  Increasing rank cannot repair centered uniqueness.",
            "",
            "`exact-p29/r2` passes but is close to the boundary; its static lhs/rhs ratio is recorded in the JSON and must not be treated as empirical quartic-backend headroom.",
        ]
    )
    REPORT_OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    outputs = [
        cargo_test("registered_p26_bounds_are_128_bit_certified"),
        cargo_test("registered_p28_p30_and_exact_bounds_are_128_bit_certified"),
    ]
    security, gates = parse_outputs(outputs)
    assert security and gates
    assert all(row["modulus"] == Q_QUARTIC for row in security)
    summary = summarize(security, gates)
    result = {
        "status": "static geometry screen pending evaluation",
        "scope": "post-wiring quartic geometry screen only; no new-geometry completeness calibration or benchmark",
        "q_quartic": Q_QUARTIC,
        "q_quartic_over_two": Q_QUARTIC / 2,
        "security_entries": len(security),
        "security_passing": sum(row["meets_128_bits"] for row in security),
        "minimum_classical_sis_bits": min(row["classical_sis_bits"] for row in security),
        "gates": len(gates),
        "gates_passing": sum(row["holds"] for row in gates),
        "summary": summary,
        "centered_gates": gates,
        "sis_commitment_security": security,
        "provenance": {
            "bounds": "currently installed verifier-enforced q_perf bounds",
            "estimator": "pinned classical Euclidean-SIS MATZOV/GSA Rust port rerun at exact q_quartic",
            "rust_audit_environment": {
                "ROKOKO_AUDIT_MODULUS": str(Q_QUARTIC),
                "ROKOKO_AUDIT_HARDNESS": "1",
                "cargo_features": "debug-hardness,quartic-q",
            },
            "claim_boundary": "These bounds have not been recalibrated on the changed quartic geometry. Passing rows are a static redesign screen only; completeness, final malicious-prover-bound certification, and performance remain uncertified.",
        },
    }
    if (
        result["security_passing"] == result["security_entries"]
        and result["gates_passing"] == result["gates"]
    ):
        result["status"] = "static geometry screen passed; quartic completeness calibration pending"
    else:
        result["status"] = "redesign required: a static SIS or centered gate failed"
    JSON_OUT.parent.mkdir(parents=True, exist_ok=True)
    JSON_OUT.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    write_report(result)
    print(f"wrote {JSON_OUT}")
    print(f"wrote {REPORT_OUT}")
    print(
        f"security={result['security_passing']}/{result['security_entries']} "
        f"minimum_bits={result['minimum_classical_sis_bits']:.0f} "
        f"gates={result['gates_passing']}/{result['gates']}"
    )
    for row in summary:
        print(
            row["parameter_line"],
            f"gates={row['centered_gates_passing']}/{row['centered_gates']}",
            f"failures={row['failing_rounds']}",
            f"security={row['security_entries_ge_128']}/{row['security_entries']}",
            f"minimum_bits={row['minimum_classical_sis_bits']:.0f}",
        )


if __name__ == "__main__":
    main()
