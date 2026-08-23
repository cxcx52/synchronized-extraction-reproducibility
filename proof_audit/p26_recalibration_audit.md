# performance-p26 geometry, calibration, and certification

Status: **closed** for balanced capacity, empirical tau=32/tau=34 completeness calibration, centered uniqueness, and pinned classical Euclidean-SIS certification.

This is the repository `q_perf` performance line and remains separate from `q_exact`.  The whole-ring extraction ledger is also separate.

## Frozen geometry

`8192x128 -> 8192x4 -> 2048x16 -> 1024x8 -> 512x8 -> 512x8 -> 1024x4`

The two changed stages preserve their prior element capacities: `1024*32 = 2048*16` and `512*16 = 1024*8`.

## Empirical calibration

Each installed raw entry is the coordinatewise maximum of the completed tau=32 and tau=34 full-chain runs and both deterministic prefixes of the corresponding boundary-regression runs.  The registered verifier bound adds the standard 2% margin.  These values are empirical completeness evidence, not a malicious-prover norm theorem.

| round | geometry | max raw NB | max raw inner NB | max raw FB | max raw PB |
|---:|---:|---:|---:|---:|---:|
| 0 | 8192x128 | 9688355.015184827 | 3732.067657478894 | 31332894.601975046 | 0.0 |
| 1 | 8192x4 | 31955598.685107652 | 4618.282797750697 | 33529894.410260584 | 28870050.818056054 |
| 2 | 2048x16 | 21237279.230148338 | 5290.237707324691 | 30766569.356588393 | 33437465.32426311 |
| 3 | 1024x8 | 30482311.339410435 | 5313.507504464447 | 30884390.879491862 | 28906270.144289907 |
| 4 | 512x8 | 31225883.499218643 | 5250.408079378211 | 32480282.823585525 | 30584859.105609626 |
| 5 | 512x8 | 5628297.382666271 | 230952.77182359167 | 26726407.77466259 | 27562308.781788617 |
| 6 | 1024x4 | 32972167.774389204 | 61506376.39596541 | -- | -- |

The full-chain runs completed in 66.13 s (tau=32) and 61.03 s (tau=34). The two-prefix boundary runs completed in 123.44 s and 119.62 s. All four runs passed their intended prover/verifier paths.

## Centered-uniqueness gates

| round | width | lhs | q/2 | pass |
|---:|---:|---:|---:|:---:|
| 1 | 16 | 462481290421503.7 | 562949953419968.5 | yes |
| 2 | 8 | 310195700383646.5 | 562949953419968.5 | yes |
| 3 | 8 | 231821221541950.53 | 562949953419968.5 | yes |
| 4 | 8 | 259526699790132.22 | 562949953419968.5 | yes |
| 5 | 4 | 105382929645882.86 | 562949953419968.5 | yes |
| 6 | 4 | 524782523278880.44 | 562949953419968.5 | yes |

All six gates pass.  In particular, the former failures are now round 1 `462481290421503.7 < q/2` and round 2 `310195700383646.5 < q/2`.

## SIS and commitment certification

| scope | m | rank | certified bound | classical bits |
|---|---:|---:|---:|---:|
| `p26/r0/commitment recursive depth 0` | 8192 | 4 | 9882122.115488524 | 158.0 |
| `p26/r0/commitment recursive depth 1` | 32 | 1 | 3806.709010628472 | 149.0 |
| `p26/r0/opening recursive depth 0` | 1024 | 4 | 9882122.115488524 | 158.0 |
| `p26/r0/opening recursive depth 1` | 32 | 1 | 3806.709010628472 | 149.0 |
| `p26/r0/basic` | 8192 | 7 | 8181645438.467724 | 136.0 |
| `p26/r1/commitment recursive depth 0` | 256 | 4 | 32594710.658809807 | 135.0 |
| `p26/r1/commitment recursive depth 1` | 32 | 1 | 4710.64845370571 | 141.0 |
| `p26/r1/opening recursive depth 0` | 64 | 4 | 32594710.658809807 | 135.0 |
| `p26/r1/opening recursive depth 1` | 32 | 1 | 4710.64845370571 | 141.0 |
| `p26/r1/projection recursive depth 0` | 2048 | 4 | 32594710.658809807 | 135.0 |
| `p26/r1/projection recursive depth 1` | 32 | 1 | 4710.64845370571 | 141.0 |
| `p26/r1/basic` | 8192 | 7 | 8755326028.407244 | 135.0 |
| `p26/r2/commitment recursive depth 0` | 1024 | 4 | 21662024.814751305 | 142.0 |
| `p26/r2/commitment recursive depth 1` | 32 | 1 | 5396.042461471185 | 136.0 |
| `p26/r2/opening recursive depth 0` | 256 | 4 | 21662024.814751305 | 142.0 |
| `p26/r2/opening recursive depth 1` | 32 | 1 | 5396.042461471185 | 136.0 |
| `p26/r2/projection-constant recursive depth 0` | 2048 | 4 | 21662024.814751305 | 142.0 |
| `p26/r2/projection-constant recursive depth 1` | 32 | 1 | 5396.042461471185 | 136.0 |
| `p26/r2/projection-batched recursive depth 0` | 256 | 4 | 21662024.814751305 | 142.0 |
| `p26/r2/projection-batched recursive depth 1` | 32 | 1 | 5396.042461471185 | 136.0 |
| `p26/r2/basic` | 2048 | 7 | 8033766590.392362 | 137.0 |
| `p26/r3/commitment recursive depth 0` | 512 | 4 | 31091957.566198643 | 136.0 |
| `p26/r3/commitment recursive depth 1` | 32 | 1 | 5419.777654553736 | 136.0 |
| `p26/r3/opening recursive depth 0` | 128 | 4 | 31091957.566198643 | 136.0 |
| `p26/r3/opening recursive depth 1` | 32 | 1 | 5419.777654553736 | 136.0 |
| `p26/r3/projection-constant recursive depth 0` | 512 | 4 | 31091957.566198643 | 136.0 |
| `p26/r3/projection-constant recursive depth 1` | 32 | 1 | 5419.777654553736 | 136.0 |
| `p26/r3/projection-batched recursive depth 0` | 128 | 4 | 31091957.566198643 | 136.0 |
| `p26/r3/projection-batched recursive depth 1` | 32 | 1 | 5419.777654553736 | 136.0 |
| `p26/r3/basic` | 1024 | 7 | 8064532146.452915 | 137.0 |
| `p26/r4/commitment recursive depth 0` | 512 | 4 | 31850401.169203017 | 136.0 |
| `p26/r4/commitment recursive depth 1` | 32 | 1 | 5355.416240965776 | 136.0 |
| `p26/r4/opening recursive depth 0` | 128 | 4 | 31850401.169203017 | 136.0 |
| `p26/r4/opening recursive depth 1` | 32 | 1 | 5355.416240965776 | 136.0 |
| `p26/r4/projection-constant recursive depth 0` | 256 | 4 | 31850401.169203017 | 136.0 |
| `p26/r4/projection-constant recursive depth 1` | 32 | 1 | 5355.416240965776 | 136.0 |
| `p26/r4/projection-batched recursive depth 0` | 128 | 4 | 31850401.169203017 | 136.0 |
| `p26/r4/projection-batched recursive depth 1` | 32 | 1 | 5355.416240965776 | 136.0 |
| `p26/r4/basic` | 512 | 7 | 8481251450.894652 | 136.0 |
| `p26/r5/commitment recursive depth 0` | 512 | 2 | 235571.8272600635 | 131.0 |
| `p26/r5/opening recursive depth 0` | 128 | 2 | 235571.8272600635 | 131.0 |
| `p26/r5/projection-constant recursive depth 0` | 512 | 2 | 235571.8272600635 | 131.0 |
| `p26/r5/projection-batched recursive depth 0` | 64 | 2 | 235571.8272600635 | 131.0 |
| `p26/r5/basic` | 512 | 7 | 6978799598.119896 | 138.0 |
| `p26/r6/simple-basic` | 1024 | 7 | 8609692449.248508 | 136.0 |

All 45 entries pass; the minimum is 131.0 classical bits.

## Benchmark decision

The p26 publication benchmark must be rerun because the registered geometry changed.  The calibration and boundary-regression timings above are correctness diagnostics and must not be reported as optimized performance numbers.
