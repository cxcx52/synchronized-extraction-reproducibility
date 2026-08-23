# Fixed-weight CRT obstruction on the performance modulus

Date: 2026-08-23

This audit concerns only the repository performance modulus
`q_perf = 2^50 - 2687` and a whole-ring, single-fork requirement that the
challenge difference be a unit.  It does not modify or import assumptions
from the separate `q_exact = 2^50 - 351` exact-strong theorem line.

## Checked algebra

`q_perf` is prime, `q_perf = 129 mod 256`, and
`v_2(q_perf - 1) = 7`.  Consequently `X^128 + 1` factors over `F_q` as 64
distinct irreducible quadratics `X^2 - omega`, where `omega` ranges over the
primitive 128-th roots in `F_q`.

For any one fixed quadratic factor, reduction maps a challenge into a set of
size at most `q^2`.  If the fixed-weight support has size

    M_tau = binom(128, tau) 2^tau,

then Cauchy--Schwarz gives component-collision probability at least `q^-2`.
After removing the trivial equality event and conditioning on distinct forks,
the rigorous lower bound is

    (M_tau - q^2) / (q^2 (M_tau - 1)).

This corrects a reciprocal ambiguity in the supplied draft formula.

## Mechanical results

| weight | log2 support | -log2 distinct lower bound |
|---:|---:|---:|
| 32 | 132.221300637 | 100.000000000 |
| 34 | 137.244261786 | 100.000000000 |

Because a collision in one factor already makes the ring difference a
nonunit, the actual nonunit probability may be larger.  Averaging the
conditional probability over the first challenge shows that at least one
first challenge has conditional nonunit probability at least the displayed
lower bound.  Therefore a uniform pointwise upper bound below `2^-128` is
information-theoretically impossible for the current interface.

## Consequence

This task cannot be closed by an NTT heuristic, a larger commitment rank, or
an asserted anti-concentration upper bound: the required upper bound is false.
At a roughly 50-bit modulus, factor degree at least three is necessary merely
to remove the codomain-size obstruction; it is not sufficient without a real
upper-bound proof.  Other possible protocol changes are a rigorously joint
multi-fork interface or a separately proved component-wise extraction
relation.

Reproduce with `python proof_audit/fixed_weight_crt_obstruction.py`.
