# Final fixed-weight implementation and proof audit

Date: 2026-08-23

No manuscript LaTeX was edited in this audit. The repository performance line
uses

    q_perf = 2^50 - 2687 = 1125899906839937.

The paper's separate exact-strong theorem line uses

    q_exact = 2^50 - 351.

No challenge-algebra property or numerical certificate is transferred between
these two moduli. In particular, the repository configurations named
`exact-p26` and `exact-p28` are exact-*norm* protocol chains over `q_perf`; they
are not executions of the paper's `q_exact` theorem line.

## Outcome

| configuration | balanced capacity | empirical completeness calibration | centered gates | >=128-bit SIS/commitments | whole-ring single-fork extraction ledger |
|---|---|---|---|---|---|
| performance p28 | pass | pass, tau=32/tau=34 plus combined boundary stream | pass | pass | blocked by the q_perf CRT obstruction |
| performance p30 | pass | pass, tau=32/tau=34 | pass | pass | blocked by the q_perf CRT obstruction |
| performance p26 | pass | pass, tau=32/tau=34 | **fail at r1 and r2** | all estimator entries pass, but rank cannot repair the gates | blocked |
| exact-norm p26 | pass | pass, tau=32/tau=34 | pass | pass | q_perf CRT obstruction still applies to a whole-ring single-fork claim |
| exact-norm p28 | pass | pass, tau=32/tau=34 | pass | pass | q_perf CRT obstruction still applies to a whole-ring single-fork claim |
| exact-norm p29 | candidate decomposition window passes | not closed; the full run is not available | not certifiable with `PB/FB = infinity` | not certifiable | blocked; existing run is documented as OOM even on 64 GiB |
| exact-norm p30 | uncalibrated | not closed | not closed | not closed | not closed; configured as an OOM-only line |

Thus p28, p30, exact-norm p26, and exact-norm p28 close the requested three
layers of (1) capacity, (2) empirical completeness calibration, and (3)
centered/SIS certification. This is not a full 128-bit extraction theorem for
`q_perf`: the fixed-weight unit-difference ledger has a separate rigorous
roughly 100-bit obstruction below.

## Final registered geometry

All listed basic commitments have rank 7. Ordinary recursive layers use rank
4 followed by rank 1; the terminal recursive layer uses rank 2. The geometry
is shown as `height x width` in execution order.

| configuration | registered chain geometry |
|---|---|
| performance p26 | `8192x128 -> 8192x4 -> 1024x32 -> 512x16 -> 512x8 -> 512x8 -> 1024x4` |
| performance p28 | `16384x256 -> 8192x8 -> 2048x16 -> 512x16 -> 512x8 -> 512x8 -> 1024x4` |
| performance p30 | `32768x512 -> 16384x8 -> 2048x32 -> 512x16 -> 512x8 -> 512x8 -> 1024x4` |
| exact-norm p26 | `8192x128 -> 16384x8 -> 8192x8 -> 1024x32 -> 512x16 -> 512x8 -> 512x8 -> 1024x4` |
| exact-norm p28 | `16384x256 -> 32768x16 -> 8192x16 -> 2048x16 -> 512x16 -> 512x8 -> 512x8 -> 1024x4` |
| exact-norm p29, unclosed | `32768x256 -> 65536x16 -> 16384x16 -> 2048x32 -> 512x16 -> 512x8 -> 512x8 -> 1024x4` |

The full-capacity generic projection source is two 25-bit centered digits.
The final sumcheck projection uses eight 7-bit digits for its constant term
and four 13-bit digits for its batched part. The final simple layer is
`1024x4` with projection ratio `2^9`. These are structural parameters; norm
thresholds remain the empirical maxima plus the registered 2% margin.

## Provenance of every norm use

There are three distinct levels.

1. **Capacity:** deterministic arithmetic checks prove that every value
   allowed by the current verifier threshold fits the balanced decomposition
   window. This is a proved property of the registered threshold and layout.
2. **Completeness calibration:** the raw `NB/PB/FB` entries are coordinatewise
   maxima from deterministic honest executions. The 2% margin is empirical;
   it is not a tail-probability theorem.
3. **Malicious-prover SIS certification:** once installed, the threshold is a
   verifier predicate. The extracted bound used by `debug-hardness` is
   therefore a deterministic accepted-witness bound, not the honest-run norm.
   The bit values below are estimator-derived using the pinned classical
   Euclidean-SIS MATZOV/GSA port at the exact registered dimensions and bounds.

## Every centered/uniqueness gate

For every row below, the right-hand side is

    q_perf / 2 = 562949953419968.5.

The left-hand side is the verifier-bound expression `width * B_argued^2`.
The performance roots have `Projection::Skip` and therefore no root gate.

| configuration | round | width | gate lhs | result |
|---|---:|---:|---:|---|
| p26 | r1 | 32 | 873525941055930.1 | **fail** |
| p26 | r2 | 16 | 565046036068864.9 | **fail** |
| p26 | r3 | 8 | 286332637421228.9 | pass |
| p26 | r4 | 8 | 178790737241685.22 | pass |
| p26 | r5 | 4 | 127322016558406.63 | pass |
| p26 | r6 simple | 4 | 464785626019109.56 | pass |
| p28 | r1 | 16 | 318590592802478.25 | pass |
| p28 | r2 | 16 | 344242224163225.06 | pass |
| p28 | r3 | 8 | 123472489286132.58 | pass |
| p28 | r4 | 8 | 221767071991389.1 | pass |
| p28 | r5 | 4 | 134086228128952.48 | pass |
| p28 | r6 simple | 4 | 555290410208102.5 | pass |
| p30 | r1 | 32 | 543044077889662.0 | pass |
| p30 | r2 | 16 | 243588960944762.97 | pass |
| p30 | r3 | 8 | 229210605732554.5 | pass |
| p30 | r4 | 8 | 147967189968197.78 | pass |
| p30 | r5 | 4 | 69224196767451.12 | pass |
| p30 | r6 simple | 4 | 520068198812393.44 | pass |
| exact-norm p26 | r0 | 8 | 25723183567253.613 | pass |
| exact-norm p26 | r1 | 8 | 966395092602.3109 | pass |
| exact-norm p26 | r2 | 32 | 130775194940491.89 | pass |
| exact-norm p26 | r3 | 16 | 314586210107716.3 | pass |
| exact-norm p26 | r4 | 8 | 239921968532749.4 | pass |
| exact-norm p26 | r5 | 8 | 139070240803757.84 | pass |
| exact-norm p26 | r6 | 4 | 122366522906976.11 | pass |
| exact-norm p26 | r7 simple | 4 | 519276105607047.75 | pass |
| exact-norm p28 | r0 | 16 | 205926443781684.4 | pass |
| exact-norm p28 | r1 | 16 | 7347772748561.7705 | pass |
| exact-norm p28 | r2 | 16 | 251304324270026.8 | pass |
| exact-norm p28 | r3 | 16 | 472953850901292.25 | pass |
| exact-norm p28 | r4 | 8 | 80410547243733.66 | pass |
| exact-norm p28 | r5 | 8 | 291620809443330.0 | pass |
| exact-norm p28 | r6 | 4 | 125058850220548.53 | pass |
| exact-norm p28 | r7 simple | 4 | 554758566778591.6 | pass |

The p28 simple gate is intentionally tight but passing. The combined boundary
stream raised the p28 r2 empirical `NB`, `FB`, and `PB` maxima; after that
update its gate is still only `3.4424e14`.

## Every SIS/commitment estimate

Vectors are in round order. `basic` includes the final simple commitment.
For sumcheck rounds, commitment, opening, coarse projection, and both fine
projection recursion paths have the same rank/bound profile, so the displayed
recursive bit value applies to every such path at that round. A dash means the
round has no recursive commitment at that depth.

| configuration | basic bits by round | recursive outer bits by sumcheck round | recursive inner bits by sumcheck round |
|---|---|---|---|
| p26 | `[137,148,140,136,137,136,136]` | `[158,136,134,135,139,131,-]` | `[149,141,136,136,136,-,-]` |
| p28 | `[138,138,137,137,138,146,136]` | `[151,137,137,142,137,131,-]` | `[149,141,136,136,136,-,-]` |
| p30 | `[155,136,137,136,142,137,136]` | `[144,136,139,137,140,131,-]` | `[149,141,136,136,136,-,-]` |
| exact-norm p26 | `[164,198,162,135,140,155,140,136]` | `[312,226,153,139,136,141,131,-]` | `[141,141,141,136,136,136,-,-]` |
| exact-norm p28 | `[152,183,147,136,136,146,136,136]` | `[275,202,141,135,145,134,131,-]` | `[141,141,141,136,136,136,-,-]` |

All displayed SIS estimates are at least 128 bits. This does not rescue p26:
its r1/r2 failure is a centered-modulus failure, which is independent of rank.

## Minimal p26 repair interface

At r1 the current width-32 gate exceeds `q/2` by a factor of about 1.552. The
largest integral width allowed by the current projection bound is 20, hence a
power-of-two layout needs width at most 16. Capacity then requires doubling
the corresponding height. At r2, the current width-16 gate misses by about
0.37%; with the current bound it needs either:

- a proved projection bound smaller by about 0.19%; or
- a power-of-two width reduction to 8.

Changing r1 alters the input geometry and empirical norms of downstream
rounds, so the minimal honest implementation change is a Small-specific
`p2`/downstream layout followed by complete tau=32/tau=34 recalibration. Rank
changes alone are not a repair.

## q_perf fixed-weight CRT obstruction

`q_perf` is prime, `q_perf = 129 mod 256`, and `v2(q_perf-1)=7`. Therefore
`X^128+1` is the product of 64 distinct irreducible quadratics. Reduction
modulo any one factor maps the fixed-weight challenge support into a set of
size `q_perf^2`.

For support size

    M_tau = binom(128,tau) 2^tau,

Cauchy--Schwarz and removal of the equality event give the rigorous
distinct-fork lower bound

    (M_tau - q_perf^2) / (q_perf^2 (M_tau - 1)).

| weight | log2 M_tau | -log2 lower bound |
|---:|---:|---:|
| 32 | 132.221300637 | 100.000000000281 |
| 34 | 137.244261786 | 100.000000000002 |

Averaging over the first challenge yields at least one first challenge whose
conditional nonunit probability is at least this value. Consequently no
uniform pointwise whole-ring single-fork upper bound below `2^-128` can be
true on the current interface. This is a proved information-theoretic
obstruction, not an NTT or frequency-domain heuristic.

Factor degree at least three is necessary at this modulus size merely to
remove this codomain-size obstruction; it is not sufficient without an
upper-bound proof. Viable changes are a different modulus, a genuinely joint
multi-fork proof under the conditional fork law, or a rigorously changed
component-wise extraction relation. The separate `q_exact` exact-strong
theorem line is unaffected.

## Regression and reproduction

- Default full Rust library suite: 152 passed, 0 failed, 2 ignored
  (`341.74 s`).
- Workspace targets and examples compile under `cargo test --workspace
  --no-run`; the `claims` example also executes and verifies all four claims.
- Rustdoc suite: pass (the two displayed challenge formulas are explicitly
  marked as text rather than accidental Rust doctests).
- Formal-threshold combined boundary regression: passed (`311.23 s`).
- Static p28/p30/exact-norm-p26/exact-norm-p28 centered/SIS certification:
  passed.
- Explicit non-asserting p26 diagnostic: traversed all rounds and recorded the
  two centered failures above; every estimator entry was at least 131 bits.
- Parameter/capacity tests under default, `challenge-weight-34`, `p-26`,
  `p-29`, and `p-30`: 7 passed for each feature set.
- Long full-chain empirical calibration: p26, p28, p30, exact-norm p26, and
  exact-norm p28 passed under both tau=32 and tau=34.
- `fixed_weight_crt_obstruction.py`: passed and regenerated the exact integer
  ledger.
- `fixed_weight_hardness_audit.py`: passed and regenerated the current p28
  verifier-bound ledger.
- `integer_ipa_parameter_audit.py`: passed its r=2..32 and cubic-denominator
  checks.

Exact-norm p29 was not rerun on this 15.6 GiB machine. Its source records that
the same full-chain calibration OOMs on 64 GiB, and its projection/folded
tables remain infinite. It is deliberately excluded from the passing static
certificate rather than being assigned extrapolated bounds.

## Benchmark decision

Publication benchmarks must be rerun for p28 and p30 because the commitment
ranks, recursive layout, sumcheck relations, projection widths, and norm
checks changed. The long calibration timings are correctness diagnostics and
must not be reported as optimized benchmark timings. Exact-norm p26/p28 also
need reruns if their performance is reported. p26 should be benchmarked only
after its centered-gate redesign and recalibration. Exact-norm p29 needs a
streaming/low-memory calibration implementation or a sufficiently large
machine before either correctness closure or benchmarking.

Reproduction artifacts:

- `proof_audit/generated/fixed_weight_hardness_audit.json`
- `proof_audit/generated/fixed_weight_crt_obstruction.json`
- `proof_audit/fixed_weight_hardness_audit.py`
- `proof_audit/fixed_weight_crt_obstruction.py`
- Rust static certifier in `rokoko/src/protocol/parties/debug_hardness.rs`
