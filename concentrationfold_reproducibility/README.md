# Anonymous reproducibility scripts

Every claim-supporting calculation script used by the current manuscript is
collected here.  All paths are relative, and no file contains author names,
machine usernames, or local absolute paths.  Generated outputs are written to
`generated/`.

Install the single calculation dependency with:

```text
python -m pip install -r requirements.txt
```

Run the complete calculation bundle with:

```text
python run_all.py
```

Run the formula-derived radius, soundness, and communication calculations:

```text
python generate_product_sync_comparison.py
```

Run the estimator-derived columns of Table 2 on the pinned exact-strong
parameter line:

```text
python run_exact_strong_estimator_port.py
```

The second script fixes `q_exact = 2^50 - 351`, records the protocol extension
degree `e = 8`, uses the manuscript's coefficient dimension
`phi * m * ell = 128 * 2^20 * 11`, and evaluates the current synchronized
radius for `L` in `{1, 2, 4, 8}`.  It reports the rank-13 estimate and searches
ranks 1 through 13 for the minimum tested rank whose classical estimate is at
least 128 bits.

The estimator script is a formula-equivalent standard-Python port of the
pinned Cyclo Euclidean SIS path.  It first checks the port against the pinned
notebook baseline, then applies the same MATZOV classical reduction-cost model
to the exact-strong line.  The pinned source commits and the scope limitation
are recorded in the script header.  It is not a complete Sage-notebook,
multi-attack, ring-aware, or quantum-security evaluation.

## Script manifest

- `generate_product_sync_comparison.py` generates the product-versus-sync
  radii, centered-gate status, exact-strong base loss, and communication rows
  used in Section 9 and Appendix F.
- `verify_exact_strong_parameters.py` independently checks primality, the
  degree-eight factorization, exact-strong margin, integrated soundness terms,
  anchor allocations, the secondary heuristic coefficients, and communication.
- `run_exact_strong_estimator_port.py` generates the rank-13 and minimum-rank
  classical security estimates in Table 2.
- `generate_extractable_anchor_tables.py` specializes the extractable-anchor
  radius and per-anchor budget formulas from Section 8 and Appendix E.
- `run_all.py` executes all four scripts in the above order.

Earlier development scripts tied to the quadratic-factor `q=2^50, e=2` line
are intentionally not copied as manuscript artifacts.  The still-relevant
pinned-estimator baseline assertion from that work is incorporated into the
exact-strong estimator script.  This prevents stale diagnostic rows from being
mistaken for inputs to the current exact-strong tables.

The separate `tools/` directory contains the relative-path MiKTeX build helper
and the optional rendered-page contact-sheet utility.  Those scripts support
building and visual QA; they are not evidence for mathematical or security
claims.
