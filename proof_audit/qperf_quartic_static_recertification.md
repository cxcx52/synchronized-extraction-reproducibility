# Static transfer screen at the quartic candidate modulus

Status: **redesign required**.  Every pinned SIS/commitment estimator entry remains at least 128 bits, but existing installed geometry fails centered uniqueness in several rounds.

This is a static screen over current verifier-enforced bounds at `q_4`; it is not a quartic-backend completeness calibration or benchmark.

- `q_4 = 926510094425921`
- `q_4/2 = 463255047212960.5`
- security: 297/297 entries at least 128 bits
- centered gates: 35/42 pass

## Per-line summary

| line | centered gates | failing rounds | SIS/commitment entries | minimum bits |
|---|---:|---|---:|---:|
| `exact-p26` | 7/8 | 7 | 54/54 | 130 |
| `exact-p28` | 6/8 | 3, 7 | 54/54 | 130 |
| `exact-p29` | 8/8 | none | 54/54 | 130 |
| `p26` | 5/6 | 6 | 45/45 | 130 |
| `p28` | 5/6 | 6 | 45/45 | 130 |
| `p30` | 4/6 | 1, 6 | 45/45 | 130 |

## Failing centered gates

| line/round | current width | lhs | q4/2 | lhs/rhs | largest power-of-two width at same bound |
|---|---:|---:|---:|---:|---:|
| `p26/r6` | 4 | 524782523278880.437500 | 463255047212960.5 | 1.132816 | 2 |
| `p28/r6` | 4 | 555290410208102.500000 | 463255047212960.5 | 1.198671 | 2 |
| `p30/r1` | 32 | 543044077889662.000000 | 463255047212960.5 | 1.172236 | 16 |
| `p30/r6` | 4 | 520068198812393.437500 | 463255047212960.5 | 1.122639 | 2 |
| `exact-p26/r7` | 4 | 519276105607047.750000 | 463255047212960.5 | 1.120929 | 2 |
| `exact-p28/r3` | 16 | 472953850901292.250000 | 463255047212960.5 | 1.020936 | 8 |
| `exact-p28/r7` | 4 | 554758566778591.625000 | 463255047212960.5 | 1.197523 | 2 |

The width column is only a necessary static repair at the same projection bound.  Capacity must be restored by increasing height, and all downstream geometry, empirical completeness bounds, gates, and estimator entries must then be regenerated.  Increasing rank cannot repair any centered-gate failure.

`exact-p29/r2` passes but is close to the boundary; its static lhs/rhs ratio is recorded in the JSON and must not be treated as empirical quartic-backend headroom.
