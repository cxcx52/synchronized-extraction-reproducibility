# Synchronized Extraction: Code and Reproducibility

This repository collects the code, replayable calculations, proof-audit
scripts, and selected frozen outputs supporting the synchronized-extraction
manuscript.  It is organized as a monorepo so that the complete code release
can be uploaded to GitHub in one push without bundling generated binaries,
Cargo build trees, or local calibration workspaces.

## Repository map

| Path | Purpose | Default action |
| --- | --- | --- |
| `concentrationfold_reproducibility/` | Formula, soundness, communication, and classical Euclidean-SIS table generation for the manuscript theorem line | `python run_all.py` |
| `proof_audit/` | Standalone arithmetic certificates, parameter audits, implementation audits, and frozen evidence logs | Run the relevant verifier listed in `proof_audit/README.md` |
| `cbc_experiments/` | Cyclo estimator cross-check and integer-IPA microbenchmarks | `python run_all.py` |
| `rokoko/` | Rust protocol implementation and the quadratic/quartic backend work | `cargo test` or a feature-specific command |
| `scripts/` | Repository-integrity checks; no cryptographic experiment is run | `python scripts/verify_repository_layout.py` |

## Claim boundaries

The repository keeps three parameter roles separate:

- `q_exact = 2^50 - 351` is the manuscript's degree-eight exact-strong
  theorem line.
- `q_perf = 2^50 - 2687` is the historical quadratic-splitting performance
  line used by the RoKoko implementation.
- the quartic `q4` line is an algebraic and implementation-audit line with its
  own certificates and protocol smoke evidence.

Outputs from one line must not be used as certification for another.  In
particular, empirical norm calibration is not a formal completeness theorem,
and a Euclidean-SIS estimator output is not a coefficientwise centered-gate
test.  The current manuscript calculation checks the centered gate using
coefficient bounds and uses the Euclidean conversion only for SIS estimation.

## Quick start

Python 3.11 or newer is recommended.

```bash
python -m venv .venv
python -m pip install -r requirements.txt
python scripts/verify_repository_layout.py
python concentrationfold_reproducibility/run_all.py
```

The CBC microbenchmarks are intentionally separate because their absolute
timings depend on the local machine:

```bash
python cbc_experiments/run_all.py
```

For the Rust implementation, the portable pure-Rust backend is the simplest
entry point:

```bash
cd rokoko
cargo +nightly test --features incomplete-rexl
cargo +nightly run --release --features incomplete-rexl
```

Large full-chain calibration and publication benchmarking are not part of the
default commands.  Selected completed logs are retained under
`proof_audit/generated/`; local reruns should be written to ignored local
output directories.

## Generated files

Small deterministic tables and certificates needed to audit manuscript claims
are tracked in Git.  Build products, Python caches, Rust `target/`, profiler
output, and ordinary benchmark results are ignored.  Run
`python scripts/verify_repository_layout.py` before pushing to check JSON
syntax, required files, accidental absolute paths, and oversized tracked files.

## External dependencies and licensing

The default Rust backend is included in `rokoko/incomplete-rexl/`.  The HEXL
backend and the legacy Sage estimator are optional external dependencies; see
`REPRODUCIBILITY.md` and `rokoko/README.md` for checkout paths and pinned
estimator provenance.

`rokoko/` is distributed under the Apache License 2.0 in `rokoko/LICENSE`.
No repository-wide license has been declared for the audit and manuscript
calculation material; choose and add one before inviting third-party reuse.

## GitHub upload

This directory itself is the intended Git repository root.  Do not upload the
surrounding Codex workspace or any `target/`, local result, or temporary build
directory.  See `REPRODUCIBILITY.md` for the pre-push checklist.
