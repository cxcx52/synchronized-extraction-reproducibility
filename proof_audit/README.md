# Proof and Implementation Audit Index

This directory contains replayable verifiers, human-readable audit reports,
and selected frozen outputs.  Files under `generated/` are evidence snapshots;
scripts beside them are the preferred replay entry points.

## Challenge algebra

| Topic | Verifier | Report or theorem text | Generated evidence |
| --- | --- | --- | --- |
| Rigorous radix-6 approximate-strong sampler | `verify_cyclo_radix6.py` | `cyclo_radix6_approx_strong.md`, `cyclo_radix6_approx_strong_lemma.tex` | `generated/cyclo_radix6_certificate.json` |
| Quadratic CRT obstruction for fixed weight | `fixed_weight_crt_obstruction.py` | `fixed_weight_crt_obstruction.md` | `generated/fixed_weight_crt_obstruction.json` |
| Quartic exact-strong line and tight weight boundary | `verify_qperf_quartic_exact_strong.py` | `qperf_quartic_exact_strong.md` | `generated/qperf_quartic_exact_strong_certificate.json` |

## Parameter and extraction audits

| Topic | Script | Report | Generated evidence |
| --- | --- | --- | --- |
| Fixed-weight hardness status | `fixed_weight_hardness_audit.py` | `fixed_weight_code_status.md` | `generated/fixed_weight_hardness_audit.json` |
| Integer IPA normalized extraction | `integer_ipa_parameter_audit.py` | `integer_ipa_status.md` | `generated/integer_ipa_parameter_audit.json` |
| Exact-p29 calibration ledger | `exact_p29_calibration_audit.py` | `exact_p29_calibration_audit.md` | JSON and captured stdout under `generated/` |
| Performance-p26 recalibration | `p26_recalibration_audit.py` | `p26_recalibration_audit.md` | `generated/p26_recalibration_audit.json` |

## Protocol implementation audits

| Topic | Script | Report | Generated evidence |
| --- | --- | --- | --- |
| Quartic backend | `qperf_quartic_backend_audit.py` | `qperf_quartic_backend_audit.md` | backend JSON and selected frozen logs |
| Quartic protocol wiring | Rust tests and audit commands in report | `qperf_quartic_protocol_wiring.md` | `generated/qperf_quartic_protocol_smoke.json` |
| Quartic static recertification | `qperf_quartic_static_recertification.py` | `qperf_quartic_static_recertification.md` | `generated/qperf_quartic_static_recertification.json` |
| Simple terminal multi-block batching | `simple_projection_batching_reference.py` | `simple_terminal_multiblock_audit.md` | `generated/simple_projection_batching_audit.json` |

## Evidence policy

- JSON/CSV certificates are tracked when they are small and support a named
  claim or audit conclusion.
- Selected full-chain logs are retained only when they are the provenance for
  an already completed run.
- Partial runs are archival and must not be used as calibration or theorem
  evidence.
- New local output should go under `proof_audit/generated/local/`, which is
  ignored by Git, until it has been reviewed and deliberately promoted.
- `q_exact`, `q_perf`, and `q4` results are separate parameter lines.
