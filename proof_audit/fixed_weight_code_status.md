# Fixed-weight implementation audit

Date: 2026-08-23

## Status

The deterministic two-run boundary regression now passes under the exact
fixed-Hamming-weight sampler.  No manuscript LaTeX was edited during this
audit.

## Root cause

The original combined boundary test executes `cut=3` and then `cut=4` without
reseeding the global test RNG.  Splitting it into two tests reset the RNG and
masked a deterministic failure in the second stream.

The failing top-level commitment-fold claims were not caused by sparse
sum-check truncation or by the optimized polynomial implementation:

- configured and actual sparse boundaries agreed at every layer;
- optimized and full pointwise claims agreed for all seven commitment rows;
- each gadget LHS/RHS agreed with an independent direct summation;
- the direct LHS and RHS disagreed before entering sum-check.

The fixed-weight challenge increased coefficient growth beyond the old
balanced-decomposition windows.  `decompose` silently retained only the low
`base_log * radix` bits, so the folded witness no longer recomposed to the
value whose commitment appeared on the other side of the constraint.

## Implemented fixes

1. Balanced decomposition now reduces the large centering shift modulo `q`,
   checks arithmetic overflows, supports windows up to 64 bits, and rejects an
   input whose shifted representative does not fit the configured window.
2. Added roundtrip coverage for the large `base_log=7, radix=8` shift and a
   negative test for an out-of-window input.
3. Resized the fixed-weight witness-decomposition bases without changing chunk
   counts or composed geometry:

       exact-norm root: 6
       integer bridge: 10
       plain root:      8
       p1..p5:          10, 11, 11, 10, 10

4. Added a deterministic capacity audit for every plain and exact-norm chain.
   It checks `width * tau * input_bound` against the balanced encoding window
   and passes under both `tau=32` and the `challenge-weight-34` feature.
5. Restored the original combined boundary test so its second RNG stream
   remains covered.
6. Added an ignored, full-chain norm-calibration utility.  Calibration mode is
   compiled only in tests; production verifier checks cannot be bypassed by an
   environment variable.
7. Refreshed the medium (`NB_P_28`) empirical ledger using coordinatewise
   maxima from the deterministic `tau=32` and `tau=34` calibration runs.  The
   existing 2% margin is applied when the config is built.

## Medium calibration maxima before the 2% runtime margin

| Layer | Combined/folded norm | Most-inner/projection norm |
|---:|---:|---:|
| 0 | 165485.91228258677 | 2658.232307380226 |
| 1 | 349654.6450456507 | 3229.8636194118167 |
| 2 | 203149.2668679855 | 3728.4597356012846 |
| 3 | 116064.08110177757 | 3723.4995635826253 |
| 4 | 81690.41117536378 | 3731.1539769888886 |
| 5 | 239997.9632767745 | 234445.91077261296 |
| 6 | 1212672.9415246306 | 2917358.674810658 |

These are reproducible benchmark acceptance measurements, not a proved tail
bound.

## Verification

- default full Rust library suite: 150 passed, 0 failed, 1 ignored;
- original combined boundary regression: passed with real verifier bounds;
- default full seven-layer execution: passed with real verifier bounds;
- full seven-layer calibration executions: passed for `tau=32` and `tau=34`;
- balanced-decomposition tests: 11 passed;
- fixed-weight capacity audits: passed for `tau=32` and `tau=34`.

## Outstanding before manuscript integration

1. Recalibrate the historical p-26, p-30, and exact-norm `NB_P_*` tables for
   the fixed-weight sampler.  Only their deterministic decomposition capacity
   has been audited here.
2. Rerun every SIS/lattice-estimator output that depends on the revised radix
   or norm ledger.  Old estimator numbers must not be carried forward.
3. Supply a fixed-weight CRT anti-concentration/nonunit argument before treating
   the repository's quadratic-splitting modulus as the exact-strong theorem
   line.  The repository remains a performance implementation on that modulus.
4. Convert the empirical medium norm ledger into the intended theorem-level
   completeness/tail statement, or label it strictly as benchmark data.

