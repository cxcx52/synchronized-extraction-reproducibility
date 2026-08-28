# Reproducibility Guide

This guide separates lightweight deterministic checks from machine-dependent
experiments and high-memory protocol runs.

## 1. Repository integrity

The following command performs no experiment.  It checks the expected layout,
parses tracked JSON evidence, validates the metric-normalized radius CSV, scans
text artifacts for accidental local absolute paths, and reports oversized
tracked files.

```bash
python scripts/verify_repository_layout.py
```

## 2. Manuscript calculations

Install the Python dependencies from the repository root:

```bash
python -m pip install -r requirements.txt
```

Run all deterministic manuscript-table calculations:

```bash
python concentrationfold_reproducibility/run_all.py
```

This regenerates files under `concentrationfold_reproducibility/generated/`.
The key radius generator checks both equivalent forms of the arity-two
separation:

```text
coefficient metric:  log2(B_sync) < log2((q-1)/2) < log2(B_prod)
Euclidean display:   log2(||z_sync||_2) < shifted gate < log2(||z_prod||_2)
```

The coefficient comparison determines centered uniqueness.  The Euclidean
radius is passed to the classical MATZOV/GSA estimator port.

## 3. Standalone proof audits

The proof-audit scripts use the Python standard library unless their source
header says otherwise.  The central lightweight commands are:

```bash
python proof_audit/verify_cyclo_radix6.py
python proof_audit/fixed_weight_crt_obstruction.py
python proof_audit/verify_qperf_quartic_exact_strong.py
python proof_audit/integer_ipa_parameter_audit.py
python proof_audit/simple_projection_batching_reference.py
```

Some parameter/backend audit scripts inspect or invoke the Rust tree.  Their
reports and selected frozen outputs are indexed in `proof_audit/README.md`.

## 4. CBC and integer-IPA experiments

```bash
python cbc_experiments/run_all.py
```

Results are written to the ignored `cbc_experiments/results/` directory.  The
integer-IPA timing is a local microbenchmark, not an end-to-end Fu protocol
benchmark.  Report the environment together with any timing result.

## 5. Rust implementation

The portable backend requires Rust nightly:

```bash
cd rokoko
cargo +nightly test --features incomplete-rexl
```

Feature-specific builds are described in `rokoko/README.md`.  Full-chain
calibration and high-memory parameter runs are intentionally not included in a
default all-in-one command.

### Optional HEXL backend

From `rokoko/`:

```bash
git clone https://github.com/IntelLabs/hexl.git hexl-bindings/hexl
make hexl
make wrapper
export LD_LIBRARY_PATH=./hexl-bindings/hexl/build/hexl/lib:$(pwd)
cargo +nightly run --release
```

### Optional legacy Sage estimator

The standard-Python estimator port is sufficient for the manuscript tables.
For the legacy Sage cross-check only, clone the pinned external estimator into
`rokoko/lattice-estimator/`:

```bash
git clone https://github.com/malb/lattice-estimator.git rokoko/lattice-estimator
git -C rokoko/lattice-estimator checkout 352ddaf4a288a0543f5d9eb588d2f89c7acec463
```

## 6. Pre-push checklist

Before creating the public GitHub repository:

1. Run `python scripts/verify_repository_layout.py`.
2. Run `git status --short` and review every source modification.
3. Confirm no secrets, usernames, private hostnames, or absolute local paths
   occur in tracked files.
4. Confirm `git ls-files` contains no `target/`, profiler, CRS, or local result
   tree.
5. Decide whether to add a repository-wide license for files outside
   `rokoko/`.
6. Create the GitHub remote and push this repository root, not its parent
   workspace.
