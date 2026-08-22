# Fixed-weight challenge: theorem line vs performance line

This branch implements the manuscript-adapted exact-Hamming-weight signed ternary sampler.

## Sampler

The distribution is exactly uniform on

    D_tau = { c in {-1,0,1}^128 : wt(c) = tau }.

Partial Fisher--Yates samples the support without modulo bias, and independent sign bits sample the signs.  No spectral rejection is performed.  Therefore the deterministic coefficient-l_infinity multiplication norm is

    gamma_infinity = ||M_c||_{infinity->infinity} = ||c||_1 = tau.

Default: tau=32.  Feature `challenge-weight-34`: tau=34.

## Security-theorem modulus line

The manuscript's rigorous fixed-weight unit-difference statement uses the separate exact-strong modulus

    q_exact = 2^50 - 351,
    ord_256(q_exact) = 8.

The power-of-two short-unit criterion gives a degree-8 factorization line and ensures every nonzero difference of two ternary challenges is a unit.  Hence for uniform D_tau,

    kappa_fix = 1 / (binom(128,tau) * 2^tau).

The two-star final-query ROM unit term is

    2 * L * M * kappa_fix,
    M = Q_fin + 1.

Thus a 128-bit unit-loss target requires

    log2(binom(128,tau) * 2^tau) >= 128 + log2(2*L*M).

Examples:

- tau=32: support ~= 132.2213 bits; sufficient for L=8, M=1, but not L=8, M=2.
- tau=34: support ~= 137.2443 bits; sufficient for L=128, M<=2 under the 2LM ledger.

If a different combined theorem uses 4LM rather than 2LM exposures, replace 2LM by 4LM in the sizing formula.

## Performance modulus line

The RoKoko repository itself retains

    MOD_Q = 1125899906839937 = 2^50 - 2687,
    ord_256(MOD_Q) = 2,

because its incomplete-NTT implementation is built around the quadratic-splitting line.  Cyclo's published quadratic-factor approximate-strong heuristic assumes iid biased ternary coefficients.  Exact-Hamming-weight sampling is not a product distribution, so that heuristic cannot be imported unchanged.  The current repository should therefore be described as a performance implementation of the fixed-weight sampler unless/until a fixed-weight CRT anti-concentration/nonunit theorem is supplied for this modulus.

## Exact field challenges

`HashWrapper::sample_u64_mod_q` now uses rejection sampling rather than direct `u64 % MOD_Q`.  This makes the transcript-derived F_q challenge exactly uniform under the XOF model and aligns the implementation with exact-uniform sum-check/ROM statements.  The old direct reduction had total-variation distance about 2^-38.6082 per draw for the current modulus.

## Decomposition capacity and norm calibration

Changing the challenge sampler changes honest-prover coefficient growth.  The
old decomposition bases were sized for the rejected spectral-norm sampler and
can silently truncate a fixed-weight fold.  Balanced decomposition now checks
its exact encoding window before extracting digits, and the fixed-weight
parameter chains use the following witness-decomposition base logs (chunk
counts are unchanged):

    exact-norm root: 6
    integer bridge: 10
    plain root:      8
    p1..p5:          10, 11, 11, 10, 10

The unit tests audit the deterministic coefficient-infinity envelope
`width * tau * input_bound` for every plain and exact-norm chain, under both
`tau=32` and `tau=34` builds.

The `NB_P_28` table was remeasured after this change.  Each entry is the
coordinatewise maximum observed across the deterministic default-weight and
weight-34 full-chain runs, plus the existing 2% runtime margin when installed
in the config.  These remain empirical benchmark acceptance bounds, not a
proof of a tail probability.  Run the ignored
`calibrate_full_chain_norms` test with `ROKOKO_CALIBRATE_NORMS=1` to refresh all
seven entries after any sampler or radix change.

The p-26, p-30, and exact-norm chains have passed the deterministic
decomposition-capacity audit, but their historical `NB_P_*` tables and SIS
estimator outputs have not yet been recalibrated for the fixed-weight sampler.

The medium p28 chain has now been rerun through the corrected hardness path.
The old path incorrectly used `base_log^(chunks-1)` as a recomposition factor;
the radix is `2^base_log`, and the exact Euclidean operator norm is the norm of
the weight vector `(1, 2^base_log, ...)`.  After correcting that formula and
isolating the actual folded-witness/projection regions, the current performance
line does **not** meet its internal 128-bit target: the root extracted bound is
at least `(q-1)/2`, the remaining basic commitments estimate to 55--104 bits,
the p1 depth-0 recursive commitments estimate to 122 bits, and the p1--p3
projection uniqueness gates fail under the observed digit-norm bounds.  See
`../proof_audit/fixed_weight_code_status.md` and the generated JSON ledger.

This p28 finding is confined to the repository's quadratic-splitting
performance line and its internal RoKoko commitment chain.  It does not alter
the separate formula-derived fixed-weight Cyclo theorem on
`q_exact = 2^50 - 351`.  Conversely, the theorem-line table must not be cited
as certification of the current Rust p28 parameters.
