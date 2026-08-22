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
