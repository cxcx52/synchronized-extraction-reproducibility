# Integer IPA proof audit status

This note records technical status only. It is not manuscript prose and does
not modify any LaTeX source.

## Mechanically checked

- The local cubic-denominator counterexample is exact: child normalized
  denominators can be at most `p` while the parent inner product has reduced
  denominator `p^3`.
- The expanded CBC modulus expression agrees with
  `4 lambda + 3 + 3 S_r + S_(r-1) + E_r + C_r`.
- For every checked `r >= 2`, the CBC line strictly dominates the local
  Cramer line by
  `[1 + 3 log2(r/(r-1))] lambda + 48 r - 25`.
- For `r=2`, the stated `m=1` CBC, outer-consistency, and pre-`y` envelopes
  are respectively `15 lambda + 107 + log2 B'`,
  `11 lambda + 100 + log2 B'`, and
  `13 lambda + 131 + log2 B'`.  CBC dominates pre-`y` when `lambda >= 13`.

## Proof repairs required before manuscript editing

1. The final outer-gamma argument must not write a rational encoding as a
   group exponent. Keep both raw integer opening equations, cross-multiply,
   use DLOG to obtain integer coefficient equalities, cancel nonzero scales
   only in `Q`, and then conclude `v=<x,y>`. This repairs the current conflict
   with the proof's own no-division-in-the-group rule without changing the
   modulus claim.
2. The theorem must state `r >= 2`; the displayed CBC formula contains
   `log2(r-1)`. The terminal budget must be piecewise (`D_r=1`, with `S_mu`
   used only for `mu>=1`) rather than invoking undefined `S_0`.
3. The almost-special-soundness interface must remain type-correct. Either
   `Break` returns the conflicting normalized opening expected by the cited
   framework and the new commitment-binding reduction outputs DLOG, or a
   modified framework lemma must be proved. Saying that `Break` directly
   outputs DLOG while invoking the original definition leaves an interface
   gap.
4. The `m=1` fresh-`y` argument is information-theoretic for a uniform vector
   challenge. The unchanged protocol derives that vector through a PRG, so the
   final theorem must include the same PRG hybrid/advantage as Fu's starred
   protocol proof.
5. The `m>1` exact integer kernel in the compressed bases prevents the current
   proof from binding an extracted post-`z` representation to pre-`z` block
   coefficients. This remains a proof gap, not an attack on the protocol.

## Claim boundary

The mechanically verified arithmetic supports a scale-invariant normalized
relation with a `32 r^2` leading term and the optimized `m=1` application,
subject to the four proof-interface repairs above. It does not establish a
literal improvement of Fu's bounded-raw-hint relation, an end-to-end runtime
speedup, or a closed theorem for general `m>1`.
