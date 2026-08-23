# exact-p29 full-chain calibration and certification

Status: **closed** for capacity, empirical completeness calibration, centered uniqueness, and pinned classical Euclidean-SIS certification.

This is the repository `q_perf` exact-norm p29 line.  It remains separate from the `q_exact` exact-strong theorem line, and it does not by itself close the separate `q_perf` extraction ledger.

## Completed run

- Remote task: `/hy-tmp/codex-exact-p29.0nCePo` on `gpushare`.
- Test: `calibrate_exact_norm_chain`, `p-29`, `ROKOKO_CALIBRATE_NORMS=1`.
- Result: 1 passed; elapsed 2387.95 s (peak monitor 2389.842 s).
- Peak RSS: aggregate 71,525,028 KiB; largest process 71,486,768 KiB; two processes at aggregate peak; 0.1 s sampling.
- Remote HEAD: `16b05f32496f91442607f8cb21482b52990f505a` with the recorded parameter and initial multi-block verifier changes uncommitted.
- Local branch/commits: `audit/code-fixes`; exact-p29 parameters `17685d8`; final Simple validator `5bb6e5c`.
- Normalized captured stdout: `proof_audit/generated/exact_p29_calibration_captured_stdout.log`.
- Cloud-only audit: no remote calibration log/JSON/CSV exists; the peak monitor is byte-identical to the committed local copy (SHA-256 `0fed1e2d...9492f0`), and the remote source diff is superseded by the two local commits.

## Raw empirical norm ledger

`NORM_MARGIN=1.02` is applied only when the raw values are installed.  These are deterministic tau=32 completeness-calibration observations, not malicious-prover norm theorems.

| round | kind | geometry | raw NB | raw inner NB | raw FB | raw PB |
|---:|---|---:|---:|---:|---:|---:|
| 0 | sumcheck | 32768x256 | 449095.42486424866 | 4573.127048311691 | 14054360.108911611 | 27241906.0563594 |
| 1 | sumcheck | 65536x16 | 2541655.0585606615 | 4586.439795745716 | 2535673.1294317096 | 5092534.207931646 |
| 2 | sumcheck | 16384x16 | 31410872.07525025 | 4567.333357660682 | 18808604.998299476 | 28742421.257555842 |
| 3 | sumcheck | 4096x16 | 6615918.423300351 | 5264.4899718861649 | 23874083.661871046 | 28177438.972140476 |
| 4 | sumcheck | 2048x8 | 31244341.336710155 | 5297.1511687463745 | 30857943.832717095 | 29436314.23636514 |
| 5 | sumcheck | 1024x8 | 28157775.739574444 | 5272.1877022479381 | 21531674.06203266 | 26968264.736921042 |
| 6 | sumcheck | 512x8 | 6186013.78539969 | 233106.229238825795 | 15447729.974848181 | 22645685.164770506 |
| 7 | simple | 2048x2 | 35134549.260899948 | 72395534.868896506 | -- | -- |

All 16 NB components exceeded the provisional installed bounds; the first was round 0 `norm` (449095.42486424866 > 301286.0855424053).  PB and FB had previously been infinite placeholders.  Every installed PB/FB/NB value is now finite.

## Centered-uniqueness gates

Every gate below uses the verifier-enforced installed projection bound, not an honest-run measured witness norm.

| round | gate width | lhs | q/2 | pass |
|---:|---:|---:|---:|:---:|
| 0 | 16 | 411788347725378.56 | 562949953419968.5 | yes |
| 1 | 16 | 14390205017160.398 | 562949953419968.5 | yes |
| 2 | 16 | 458401227545903.3 | 562949953419968.5 | yes |
| 3 | 8 | 220278500516442.8 | 562949953419968.5 | yes |
| 4 | 8 | 240400815544864.88 | 562949953419968.5 | yes |
| 5 | 8 | 201778589322307.62 | 562949953419968.5 | yes |
| 6 | 2 | 35569684644521.06 | 562949953419968.5 | yes |
| 7 | 2 | 363523630206622.5 | 562949953419968.5 | yes |

## SIS and commitment certification

All entries use the pinned classical Euclidean-SIS MATZOV/GSA Rust port at the exact registered dimension, rank, and verifier-enforced installed bound.  The minimum is 131 bits.

| scope | m | rank | certified bound | classical bits |
|---|---:|---:|---:|---:|
| `exact-p29/r0/commitment recursive depth 0` | 16384 | 4 | 458077.33336153365 | 258.0 |
| `exact-p29/r0/commitment recursive depth 1` | 32 | 1 | 4664.589589277925 | 141.0 |
| `exact-p29/r0/opening recursive depth 0` | 2048 | 4 | 458077.33336153365 | 258.0 |
| `exact-p29/r0/opening recursive depth 1` | 32 | 1 | 4664.589589277925 | 141.0 |
| `exact-p29/r0/projection recursive depth 0` | 524288 | 4 | 458077.33336153365 | 258.0 |
| `exact-p29/r0/projection recursive depth 1` | 32 | 1 | 4664.589589277925 | 141.0 |
| `exact-p29/r0/basic` | 32768 | 7 | 3669874511.639 | 148.0 |
| `exact-p29/r1/commitment recursive depth 0` | 1024 | 4 | 2592488.1597318747 | 193.0 |
| `exact-p29/r1/commitment recursive depth 1` | 32 | 1 | 4678.16859166063 | 141.0 |
| `exact-p29/r1/opening recursive depth 0` | 256 | 4 | 2592488.1597318747 | 193.0 |
| `exact-p29/r1/opening recursive depth 1` | 32 | 1 | 4678.16859166063 | 141.0 |
| `exact-p29/r1/projection recursive depth 0` | 32768 | 4 | 2592488.1597318747 | 193.0 |
| `exact-p29/r1/projection recursive depth 1` | 32 | 1 | 4678.16859166063 | 141.0 |
| `exact-p29/r1/basic` | 65536 | 7 | 662114967.5572081 | 177.0 |
| `exact-p29/r2/commitment recursive depth 0` | 1024 | 4 | 32039089.516755253 | 135.0 |
| `exact-p29/r2/commitment recursive depth 1` | 32 | 1 | 4658.680024813895 | 141.0 |
| `exact-p29/r2/opening recursive depth 0` | 256 | 4 | 32039089.516755253 | 135.0 |
| `exact-p29/r2/opening recursive depth 1` | 32 | 1 | 4658.680024813895 | 141.0 |
| `exact-p29/r2/projection recursive depth 0` | 16384 | 4 | 32039089.516755253 | 135.0 |
| `exact-p29/r2/projection recursive depth 1` | 32 | 1 | 4658.680024813895 | 141.0 |
| `exact-p29/r2/basic` | 16384 | 7 | 4911302937.155959 | 143.0 |
| `exact-p29/r3/commitment recursive depth 0` | 1024 | 4 | 6748236.791766359 | 167.0 |
| `exact-p29/r3/commitment recursive depth 1` | 32 | 1 | 5369.779771323889 | 136.0 |
| `exact-p29/r3/opening recursive depth 0` | 256 | 4 | 6748236.791766359 | 167.0 |
| `exact-p29/r3/opening recursive depth 1` | 32 | 1 | 5369.779771323889 | 136.0 |
| `exact-p29/r3/projection-constant recursive depth 0` | 512 | 4 | 6748236.791766359 | 167.0 |
| `exact-p29/r3/projection-constant recursive depth 1` | 32 | 1 | 5369.779771323889 | 136.0 |
| `exact-p29/r3/projection-batched recursive depth 0` | 256 | 4 | 6748236.791766359 | 167.0 |
| `exact-p29/r3/projection-batched recursive depth 1` | 32 | 1 | 5369.779771323889 | 136.0 |
| `exact-p29/r3/basic` | 4096 | 7 | 6234000725.787767 | 140.0 |
| `exact-p29/r4/commitment recursive depth 0` | 512 | 4 | 31869228.16344436 | 136.0 |
| `exact-p29/r4/commitment recursive depth 1` | 32 | 1 | 5403.0941921213025 | 136.0 |
| `exact-p29/r4/opening recursive depth 0` | 128 | 4 | 31869228.16344436 | 136.0 |
| `exact-p29/r4/opening recursive depth 1` | 32 | 1 | 5403.0941921213025 | 136.0 |
| `exact-p29/r4/projection-constant recursive depth 0` | 1024 | 4 | 31869228.16344436 | 136.0 |
| `exact-p29/r4/projection-constant recursive depth 1` | 32 | 1 | 5403.0941921213025 | 136.0 |
| `exact-p29/r4/projection-batched recursive depth 0` | 128 | 4 | 31869228.16344436 | 136.0 |
| `exact-p29/r4/projection-batched recursive depth 1` | 32 | 1 | 5403.0941921213025 | 136.0 |
| `exact-p29/r4/basic` | 2048 | 7 | 8057626293.599088 | 137.0 |
| `exact-p29/r5/commitment recursive depth 0` | 512 | 4 | 28720931.254365932 | 137.0 |
| `exact-p29/r5/commitment recursive depth 1` | 32 | 1 | 5377.631456292897 | 136.0 |
| `exact-p29/r5/opening recursive depth 0` | 128 | 4 | 28720931.254365932 | 137.0 |
| `exact-p29/r5/opening recursive depth 1` | 32 | 1 | 5377.631456292897 | 136.0 |
| `exact-p29/r5/projection-constant recursive depth 0` | 512 | 4 | 28720931.254365932 | 137.0 |
| `exact-p29/r5/projection-constant recursive depth 1` | 32 | 1 | 5377.631456292897 | 136.0 |
| `exact-p29/r5/projection-batched recursive depth 0` | 128 | 4 | 28720931.254365932 | 137.0 |
| `exact-p29/r5/projection-batched recursive depth 1` | 32 | 1 | 5377.631456292897 | 136.0 |
| `exact-p29/r5/basic` | 1024 | 7 | 5622350731.077968 | 141.0 |
| `exact-p29/r6/commitment recursive depth 0` | 512 | 2 | 237768.35382360232 | 131.0 |
| `exact-p29/r6/opening recursive depth 0` | 128 | 2 | 237768.35382360232 | 131.0 |
| `exact-p29/r6/projection-constant recursive depth 0` | 512 | 2 | 237768.35382360232 | 131.0 |
| `exact-p29/r6/projection-batched recursive depth 0` | 64 | 2 | 237768.35382360232 | 131.0 |
| `exact-p29/r6/basic` | 512 | 7 | 4033711251.032357 | 146.0 |
| `exact-p29/r7/simple-basic` | 2048 | 7 | 9174333503.006197 | 135.0 |

## Regression and rerun decision

- Balanced-decomposition capacity: pass.
- p29 chain dimensions: pass.
- p29 front-end witness size: pass.
- Static centered/SIS regression: pass (8/8 gates, 54/54 SIS/commitment entries).
- The completed remote binary already contained the honest multi-block `c_0` reconstruction and constant-term fix.  Commit `5bb6e5c` adds only fail-closed geometry/canonical checks and tests; it does not change the honest transcript or these norm values.
- No second calibration is required for those validation-only additions.  Publication benchmarks still require a fresh run after exact-p29, p26, and the `q_perf` extraction ledger are all closed; the 2387.95 s calibration timing is not a publication benchmark.
