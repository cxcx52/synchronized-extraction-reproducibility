# Static quartic geometry screen

Status: **static geometry screen passed; quartic completeness calibration remains required**.

This is a static screen of the quartic-only geometry over the previously installed verifier-enforced bounds at `q_4`; it is not a completeness calibration, final parameter certification, or benchmark.

- `q_4 = 926510094425921`
- `q_4/2 = 463255047212960.5`
- security: 297/297 entries at least 128 bits
- centered gates: 42/42 pass

## Quartic-only geometry changes

| scope | old geometry | quartic geometry | reason |
|---|---:|---:|---|
| `p30/p_2` | `2048 x 32` | `4096 x 16` | close r1 centered gate at unchanged input capacity |
| `p30/p_3` | `512 x 16` | `1024 x 16` | carry the enlarged p2 composed image without widening |
| `p30/p_4` | `512 x 8` | `1024 x 8` | carry the enlarged p3 composed image without widening |
| `exact-p28/p_3` | `512 x 16` | `1024 x 8` | close r3 centered gate at unchanged input capacity |
| terminal (`p26`, `p28`, `p30`, `exact-p26`, `exact-p28`) | `1024 x 4` | `2048 x 2` | close the final centered gate at unchanged input capacity |

The exact-p29 terminal was already `2048 x 2`; no q4 override changes it.

## Per-line summary

| line | centered gates | failing rounds | SIS/commitment entries | minimum bits |
|---|---:|---|---:|---:|
| `exact-p26` | 8/8 | none | 54/54 | 130 |
| `exact-p28` | 8/8 | none | 54/54 | 130 |
| `exact-p29` | 8/8 | none | 54/54 | 130 |
| `p26` | 6/6 | none | 45/45 | 130 |
| `p28` | 6/6 | none | 45/45 | 130 |
| `p30` | 6/6 | none | 45/45 | 130 |

## Failing centered gates

None in the post-wiring static screen.

Every changed layout preserves the predecessor capacity required by the generated chain.  The old bounds used here are only a static redesign screen: quartic full-chain calibration must regenerate PB/FB/NB before the gates and estimator rows become final certificates.  Increasing rank cannot repair centered uniqueness.

`exact-p29/r2` passes but is close to the boundary; its static lhs/rhs ratio is recorded in the JSON and must not be treated as empirical quartic-backend headroom.
