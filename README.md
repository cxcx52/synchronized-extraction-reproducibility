# Synchronized Extraction: Code and Reproducibility

This repository collects the code, replayable calculations, proof-audit
scripts, and selected frozen outputs supporting the synchronized-extraction
manuscript.  It is organized as a monorepo so that the complete code release
can be uploaded to GitHub in one push without bundling generated binaries,
Cargo build trees, or local calibration workspaces.

## Paper overview

The manuscript, *Avoiding Product-Denominator Blowup in Lattice-Folding
Extraction*, studies a concrete loss that appears when coordinate-wise
extraction produces normalized or fractional openings but the final lattice
reduction requires a short integral SIS witness.  Clearing every coordinate
denominator independently in two forked branches can multiply the local
shortness losses, producing a radius that grows like a product across the
folding arity.

The paper changes the order of these operations.  It synchronizes two
success-selected coordinate stars at the same numerical root, compares their
normalized openings locally, and clears only the denominators needed for that
comparison.  Acceptance-weighted shared-root resampling and coordinate-fiber
cancellation realize this comparison against malicious provers with additive
raw extraction loss and linear unconditional expected retry complexity.

The two main contributions are:

1. **Synchronized extraction without product-denominator blowup.**  The paper
   formalizes an anchored affine extraction interface and derives a local
   comparison radius that depends on the largest local challenge slack rather
   than the product of all coordinate denominators.  The construction includes
   the probability accounting needed to synchronize two success-selected
   coordinate stars around one inherited root.
2. **A concrete consequence for Cyclo.**  The interface is instantiated for a
   Cyclo-compatible fold.  At arity two, the synchronized coefficient-radius
   exponent is `25.0444`, the coefficientwise centered-modulus threshold is
   approximately `49`, and the product-clearing exponent is `49.6294`.
   Consequently, synchronized extraction remains in the centered regime while
   the product-clearing bound crosses the threshold.

The accompanying results discharge the algebraic and compilation obligations
of this application.  Fixed-weight challenge families are analyzed through
their CRT components, including an exact-strong degree-eight line, the
quadratic support obstruction, and a quartic exact-strong construction.  A
one-fold classical-ROM theorem compiles the synchronized extractor, while the
integer-IPA audit records how the same compare-before-clearing principle
changes an independent capacity calculation.  The scripts and frozen evidence
for these statements are indexed under `proof_audit/` and
`concentrationfold_reproducibility/`.

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
