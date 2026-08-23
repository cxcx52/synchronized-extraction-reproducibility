# q_perf quartic protocol wiring audit

## Scope and boundary

This audit ports the already-certified quartic candidate

`q_4 = 926510094425921`, `EXTENSION_DEGREE = 4`, `NUM_SLOTS = 32`

through the repository protocol implementation.  The default build remains

`q_perf = 1125899906839937`, `EXTENSION_DEGREE = 2`, `NUM_SLOTS = 64`.

The historical Rust name `QuadraticExtension` is intentionally retained to
limit the diff.  Its storage, arithmetic, transcript encoding, and protocol
loops are degree-parameterized; no quartic claim uses the quadratic identity
`x * x^q in F_q`.

## Recovered invariants

1. Coefficient position `EXTENSION_DEGREE * digit + residue` is stored in
   residue block `residue`, offset `digit`.
2. Each residue block receives an independent size-`NUM_SLOTS` negacyclic NTT.
3. A raw quartic slot `F_q[X]/(X^4-zeta_i)` is mapped into the common
   presentation `F_q[T]/(T^4-beta)` by the certified odd exponent satisfying
   `zeta_i = beta^e_i` and the substitution `X -> T^e_i`.
4. Homogeneous layout is coefficient-major:
   `layout[degree * NUM_SLOTS + slot]`.
5. Ring conjugation means the ring Galois automorphism `X -> X^{-1}`.  The
   quartic NTT-domain permutation and factors are derived from this coefficient
   reference map and checked against it; they are not a quartic-field
   Frobenius shortcut.
6. Extension transcript encoding absorbs every base-field limb in increasing
   basis degree using canonical little-endian `u64` encoding.

## Degree-two dependencies removed or isolated

- four-way coefficient blocking and NTT transforms replace two hard-coded
  even/odd transforms under `quartic-q`;
- raw-slot homogenization and its inverse use the explicit quartic isomorphism;
- raw and homogeneous multiplication use four-limb field arithmetic;
- quartic inverse is computed slotwise in the certified field;
- ring conjugation and constant-term recovery are valid independently of CRT
  residue degree;
- field/ring embeddings, sum-check constructors, verifier loaders, and field
  split/combine loops use the configured extension degree;
- scalar fast-path detection rejects any nonzero nonconstant limb, rather than
  checking only limb one;
- coarse projection transforms all residue streams;
- the projection-sumcheck AVX-512 helper remains the default quadratic fast
  path, while `quartic-q` is explicitly routed through the degree-generic
  reference evaluator;
- `quartic-q,challenge-weight-34` fails at compile time, because the certified
  exact-strong interface is `D_32` and the audit contains explicit weight-34
  CRT collisions.

## Validation gates

The following gates are recorded by the commit that includes this file:

- compile matrix: default, `p-26`, `quartic-q`, `quartic-q,p-29`, and
  `quartic-q,p-26`;
- isolated quartic arithmetic and raw/homogeneous layout differentials;
- main `RingElement` multiplication, inverse, conjugation, norm, and
  constant-term tests under `quartic-q`;
- canonical extension encoding and all-limb transcript binding;
- protocol sum-check/SNARK/projection unit tests;
- full prover/verifier boundary smoke at cuts 3 and 4.

The ordinary quartic test run produced `158 passed / 1 failed / 2 ignored`.
The sole failure was the old p28 empirical completeness assertion
`28615442.005693953 <= 28135160.575741395`; it occurred after the prover had
reached cut 3 and is not an algebraic or transcript mismatch.  Re-running the
same cut-3/cut-4 test with `ROKOKO_CALIBRATE_NORMS=1` (which logs rather than
asserts old empirical thresholds) completed both prover/verifier boundaries:
`1 passed / 0 failed` in 735.15 seconds.  The raw timings and observed norm
values are recorded in `generated/qperf_quartic_protocol_smoke.json` and are
not publication benchmarks or installed bounds.

This audit establishes protocol typing and algebraic correctness only.  It does
not transfer completeness calibration, centered-gate certification, SIS
security, or benchmark timings to the quartic line.
