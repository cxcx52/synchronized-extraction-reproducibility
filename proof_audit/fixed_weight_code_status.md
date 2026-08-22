# Fixed-weight implementation audit

Date: 2026-08-23

## Status

The deterministic two-run boundary regression now passes under the exact
fixed-Hamming-weight sampler.  No manuscript LaTeX was edited during this
audit.  The protocol implementation is algebraically consistent, but the
current p28 parameter line fails the corrected extraction-hardness audit below.

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

## Corrected hardness audit: blocking result for the performance code line

This result concerns the repository's quadratic-splitting performance line
`q_perf = 2^50 - 2687` and its internal RoKoko p28 commitment chain.  It does
not invalidate the separate formula-derived Cyclo theorem table on
`q_exact = 2^50 - 351`.  The supplied one-star/fixed-weight resolution note
explicitly separates those two modulus and norm interfaces.  The finding
blocks any statement that the current Rust p28 chain is already certified at
128 classical bits; it is not a counterexample to the exact-strong
coefficient-infinity theorem.

The legacy `debug-hardness` path treated `decomposition_base_log` as if it
were the radix.  It multiplied a decomposed Euclidean norm by
`base_log^(chunks-1)`.  Balanced recomposition actually has weights

    1, 2^base_log, ..., 2^(base_log * (chunks-1)),

so its exact Euclidean operator norm is the square root of the sum of the
squared weights.  The old expression underestimated the recomposition factor
by orders of magnitude.  The same bug appeared in the folded-witness and
projection paths.

The corrected audit also isolates the folded-witness and projection regions by
their generated layout prefixes.  It no longer charges unrelated packed
opening/commitment data to those two bounds.  An explicit test-only exhaustive
mode (`ROKOKO_AUDIT_HARDNESS=1`) reports every layer while leaving production
checks unchanged.

For the deterministic medium, `tau=32` execution, the pinned classical
Euclidean-SIS MATZOV/GSA port now reports:

| Layer | Current rank | Corrected observed extraction bound | Current bits | Observed min. rank for >=128 bits |
|---|---:|---:|---:|---:|
| root | 6 | 662371544518619.5 | trivial (`B >= (q-1)/2`) | none |
| p1 | 5 | 78126653221.73138 | 79 | 8 (128 bits) |
| p2 | 5 | 88595099476.23438 | 78 | 9 (145 bits) |
| p3 | 4 | 53980621254.51988 | 65 | 8 (133 bits) |
| p4 | 4 | 18946047940.875683 | 71 | 8 (146 bits) |
| p5 | 3 | 13454245035.26238 | 55 | 7 (130 bits) |
| simple | 4 | 310021936.4924724 | 104 | 5 (132 bits) |

The root bound exceeds `(q-1)/2 = 562949953419968`, so increasing its rank
cannot repair the current line.  The p1, p2, and p3 certified projection
recomposition bounds also fail the centered uniqueness gate; p4 and p5 pass.
All five *observed honest* recomposed projection norms pass, which shows that
the benchmark execution is much shorter than the worst direction permitted by
the current digit interface.  Honest observations cannot replace a malicious-
prover bound.

The recursive-commitment scan finds one additional sub-target path: the p1
depth-0 commitment, opening, and projection-image commitments each estimate to
122 bits at rank 2 under the audit's shared observed length bound.  The other
reported recursive paths range from 131 to 165 bits.  The strict
`debug-hardness` mode now treats an estimator error or any estimate below 128
bits as a failure; it no longer merely prints those results.

The rank column above is deliberately labelled observed: it scans the exact
MATZOV/GSA port at the deterministic run's measured norm.  It is not a final
parameter recommendation.  A theorem-level rank table must instead use proved
norm budgets, and the root plus projection-gate failures require a coordinated
parameter/proof redesign before ranks are changed.

Reproduction artifacts:

- `proof_audit/fixed_weight_hardness_audit.py` recomputes every algebraic bound;
- `proof_audit/generated/fixed_weight_hardness_audit.json` records the full
  ledger and provenance;
- the ignored Rust test `audit_medium_observed_minimum_ranks` reruns the pinned
  estimator rank scan.

## Outstanding before implementation claims are integrated into the manuscript

1. Recalibrate the historical p-26, p-30, and exact-norm `NB_P_*` tables for
   the fixed-weight sampler.  Only their deterministic decomposition capacity
   has been audited here.
2. Redesign the p28 extraction parameter line so the root bound is below
   `(q-1)/2`, every projection uniqueness gate is certified, and every basic
   plus recursive commitment meets the target under proved norm budgets.  Old
   rank/security numbers must not be carried forward.
3. Supply a fixed-weight CRT anti-concentration/nonunit argument before treating
   the repository's quadratic-splitting modulus as the exact-strong theorem
   line.  The repository remains a performance implementation on that modulus.
4. Convert the empirical medium norm ledger into the intended theorem-level
   completeness/tail statement, or label it strictly as benchmark data.
