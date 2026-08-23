# Quartic arithmetic backend audit

Status: **proved locally** for the isolated arithmetic backend.

The default quadratic implementation is unchanged.  The feature `quartic-q-audit` enables a separate degree-four backend over `q_4=926510094425921`.

## Construction

- coefficients are split by index modulo four;
- each stream uses a size-32 negacyclic NTT;
- the 32 raw factors `X^4-beta^u` are mapped to one common field `T^4-beta` by `X -> T^u`;
- multiplication is componentwise in the common quartic field;
- inverse NTTs and re-interleaving recover coefficient form.

## Regressions

| check | cases | result |
|---|---:|---|
| coefficient/slot round trip | 256 | pass |
| NTT/slot product vs independent O(128^2) negacyclic product | 256 | pass |
| quartic-slot and whole-ring inverses | 16 | pass |
| NTT slot constants and odd-power factorization | 32 slots | pass |

The reference product is coefficient-domain convolution and does not call the transform or slot multiplication code.

## Boundary

This closes arithmetic correctness in isolation.  It does not yet migrate the protocol's explicit `QuadraticExtension` sum-check interface, establish quartic-backend completeness bounds, repair the seven static centered-gate failures, or provide publication timings.
