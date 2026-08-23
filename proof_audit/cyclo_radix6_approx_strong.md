# Rigorous radix-six approximate-strong sampling for a quadratic Cyclo split

**Status: `proved`.**

This audit gives an explicit, heuristic-free degree-two instantiation of
Cyclo's approximate-strong challenge interface.  It is a separate theorem
line.  It does not preserve Cyclo's original benchmark modulus and does not
change the exact-strong flagship line.

## 1. Explicit parameters

Let

\[
 R=\mathbb Z[X]/(X^{128}+1),\qquad
 q=447183309836853377,
\]

and let

\[
 \mathcal A=\{-3,-2,-1,0,1,2\},\qquad
 \mathcal D_6=\left\{\sum_{i=0}^{127}s_iX^i:s_i\in\mathcal A\right\}
 \subset R_q.
\]

The challenge law samples all 128 coefficients independently and exactly
uniformly from `A`.  Consequently

\[
 |\mathcal D_6|=6^{128}
 \quad\text{and}\quad
 \Pr[s=s']=6^{-128}
\]

for independent challenges.  Distinct coefficient vectors give distinct
elements of `R_q`, since every coefficient difference is strictly between
`-q` and `q`.

## 2. Machine-checked modulus arithmetic

The standard-library-only verifier
`proof_audit/verify_cyclo_radix6.py` checks a complete recursive Pocklington
certificate for `q` and writes the certificate and all derived values to
`proof_audit/generated/cyclo_radix6_certificate.json`.  It checks

\[
\begin{aligned}
6^{64}+1
 &=4926056449\cdot q\cdot28753787197056661026689,\\
q&\equiv129\pmod {256},\\
6^{64}&\equiv-1\pmod q,\\
6^{128}&\equiv1\pmod q,
\end{aligned}
\]

and checks that no proper divisor of 128 is the order of 6.  Thus 6 has
multiplicative order 128 in `F_q`.

The certificate gives the complete factorization

\[
 q-1=2^7\cdot15313\cdot19801\cdot11522009
\]

and recursively certifies every prime in the factor tree.  For every certified
integer `n`, the verifier checks that the listed prime powers multiply to
`n-1`, recursively verifies their primality, and checks the Pocklington
congruence and gcd condition for each listed witness.  Hence the JSON is a
machine-checkable primality certificate, not a probable-prime transcript.

For every odd `u` modulo 128 put `omega_u=6^u`.  These are the 64 distinct
primitive 128-th roots in `F_q`.  Since `v_2(q-1)=7`, every `omega_u` is a
nonsquare.  Therefore every `X^2-omega_u` is irreducible and

\[
 X^{128}+1=\prod_{\substack{1\le u<128\\u\text{ odd}}}
 (X^2-\omega_u)\pmod q.
\]

The verifier additionally multiplies all 64 displayed quadratic factors and
checks this polynomial identity coefficient by coefficient.

## 3. Twenty-two-digit radix-six anti-concentration

Fix an odd `u` and work in the quadratic slot

\[
 F_q[Y]/(Y^2-\omega_u),\qquad \omega_u=6^u.
\]

The residue of `s` is

\[
 s(Y)=A_u+YB_u,
 \quad
 A_u=\sum_{j=0}^{63}s_{2j}\omega_u^j,
 \quad
 B_u=\sum_{j=0}^{63}s_{2j+1}\omega_u^j.
\]

For each `t` in `{0,...,21}`, let `r_t` be the representative in
`{0,...,127}` of `u^{-1}t mod 128`.  Set

\[
(j_t,\epsilon_t)=
\begin{cases}
(r_t,+1),&r_t<64,\\
(r_t-64,-1),&r_t\ge64.
\end{cases}
\]

Because `u` is odd and `6^64=-1 mod q`, this gives

\[
 \omega_u^{j_t}=\epsilon_t6^t.
\]

The 22 indices `j_t` are distinct.  Indeed, equality of two indices would
give `t-t'=0` or `64 mod 128`, neither of which is possible for distinct
`t,t'` in `{0,...,21}`.  The verifier checks this reindexing identity and
distinctness for all 64 slots.

Fix all coefficients outside these 22 selected even positions.  The remaining
part of `A_u` has the form

\[
 C+\sum_{t=0}^{21}\epsilon_td_t6^t,
 \qquad d_t\mathrel{\$\leftarrow}\mathcal A.
\]

This map from `A^22` to `F_q` is injective.  If two blocks collide, their
integer difference `D` satisfies

\[
 |D|\le5\sum_{t=0}^{21}6^t=6^{22}-1<q.
\]

Thus modular equality implies integer equality.  Reducing the integer equality
modulo 6 forces the zeroth digit difference to vanish because it lies in
`[-5,5]`; division by 6 and induction force all 22 digit differences to
vanish.  The use of 22 digits is maximal for this direct argument:

\[
 6^{22}-1=131621703842267135<q
 <789730223053602815=6^{23}-1.
\]

It follows, pointwise and after fixing arbitrary remaining coefficients, that

\[
 \max_a\Pr[A_u=a]\le6^{-22}.
\]

The analogous block for `B_u` uses 22 odd coefficients, disjoint from the even
block.  Independence of all coefficients therefore gives the joint slot bound

\[
 \max_{(a,b)\in F_q^2}\Pr[(A_u,B_u)=(a,b)]\le6^{-44}.
\]

No Fourier approximation, random-root model, or empirical distributional fit
is used.

## 4. Distinct-challenge nonunit bound

Let `s,s'` be independent samples from `D_6`.  In a fixed quadratic slot,

\[
 \Pr[s\bmod(X^2-\omega_u)=s'\bmod(X^2-\omega_u)]
 \le6^{-44},
\]

because collision probability is at most the largest point mass.  The
difference `s-s'` is a nonunit precisely when it is zero in at least one CRT
slot.  A union bound over the 64 slots yields

\[
 \Pr[s-s'\notin R_q^\times]\le64\cdot6^{-44}.
\]

Cyclo's approximate-strong interface applies after excluding a repeated
challenge.  Since equality implies noninvertibility, the sharper conditioned
bound is

\[
\begin{aligned}
 \kappa_{\rm nu}
 &:={\Pr}[s-s'\notin R_q^\times\mid s\ne s']\\
 &\le
 \frac{64\cdot6^{-44}-6^{-128}}{1-6^{-128}}.
\end{aligned}
\]

The verifier computes

\[
 -\log_2\kappa_{\rm nu}
 \ge107.738350031731.
\]

This is a rigorous approximately 107.74-bit nonunit bound.  It is not a
128-bit nonunit claim.

## 5. Operator norm

For canonical coefficient representatives, negacyclic convolution and the
triangle inequality give, for every `t`,

\[
 \|t s\bmod(X^{128}+1,q)\|_\infty
 \le \|t\|_\infty\|s\|_1.
\]

Every coefficient of `s` has absolute value at most 3, so uniformly

\[
 \|s\|_{\rm op}\le\|s\|_1\le128\cdot3=384.
\]

Thus `D_6` has Cyclo challenge-norm parameter `gamma=384`; in a fork,
`||s-s'||_op<=768`.  This is a uniform upper bound.  No equality claim is
needed or made.

## 6. Cyclo theorem interface

Cyclo Lemma 9 labels its quadratic-splitting argument heuristic and estimates
the nonunit probability by extending earlier distributional calculations.
The lemma above is an explicit rigorous replacement for one degree-two
parameter set.

Cyclo Theorem 3 keeps the repeat term and nonunit term separate:

\[
 \frac{L}{|\mathcal D|}+L\kappa_{\rm nu}.
\]

For this sampler they become

\[
 L6^{-128}
 +L\frac{64\cdot6^{-44}-6^{-128}}{1-6^{-128}}.
\]

The first term charges an identical coordinate challenge.  Conditional on
distinct challenges, the second term is exactly the failure mode bounded
above.  Hence the conditioning matches the theorem interface rather than
silently counting repeats twice.

## 7. Claim boundary and implementation trade-off

The proved claims are:

> There exists an explicit heuristic-free quadratic-splitting instantiation
> of Cyclo's approximate-strong challenge interface.

> It rigorously replaces the heuristic reasoning of Cyclo Lemma 9 for an
> explicit degree-2 parameter set.

The following are not claimed:

- preservation of Cyclo's original benchmark;
- a drop-in rigorousization of the original 50-bit parameter line;
- 128-bit nonunit security;
- any improvement to the separate exact-strong flagship radius line.

The modulus `q` has 59 bits.  It lies outside the 50-bit arithmetic regime used
by the original AVX-512/IFMA benchmark, so those measurements do not carry
over.  No benchmark was run for this audit.

For exact implementation sampling, consume XOF bytes, reject values in
`{252,253,254,255}`, and map an accepted byte `b` to the alphabet element
`(b mod 6)-3`.  The accepted range has size 252, a multiple of 6, so this is
exactly uniform.  Applying `% 6` without the rejection step would be biased and
is not an admissible implementation of the mathematical distribution.

## 8. Design-frontier observation, not a general lower bound

For this same radix proof, `d` digits per coordinate give the unconditioned
64-slot bound `64*6^(-2d)`.  Including the negligible distinct-conditioning
correction, the smallest integer reaching 128 bits is

\[
 d=26,
\]

for which the verifier obtains 128.418050037500 bits.  Injectivity would then
require

\[
 q>6^{26}-1,
\]

so this argument needs a modulus of at least 68 bits.  This is only a design
trade-off for the stated radix construction; it is not a general lower bound
on approximate-strong sampling or on residue degree.

## 9. Reproduction

From the repository root run:

```text
python proof_audit/verify_cyclo_radix6.py
```

Expected summary:

```text
q=447183309836853377 bits=59 factors=64xdegree-2
conditioned_nonunit_bits=107.738350031731
frontier_digits=26 frontier_modulus_bits=68
```

The generated JSON contains the complete Pocklington factor/witness tree,
every explicit quadratic factor, exact rational probability bounds, operator
norms, and the design-frontier calculation.
