"""Generate the formula-derived product-clearing versus synchronized comparison.

The formulas are theorem-derived.  This script does not run a lattice
estimator and does not claim a noninteractive security result.
"""

from __future__ import annotations

import csv
import math
from pathlib import Path


ROOT = Path(__file__).resolve().parent
OUT = ROOT / "generated"
OUT.mkdir(exist_ok=True)

PHI = 128
M = 2**20
B = 2**10
T = 64
B_DIGIT = 1
GAMMA = 128
ELL = 11
Q_EXACT = 2**50 - 351
Q_BITS = 50
E = 8
K = 3
A_PRIME = 13
ELL_1 = 31
DIM = M * ELL * PHI
EUCLIDEAN_SHIFT = 0.5 * math.log2(DIM)
GATE = math.log2((Q_EXACT - 1) / 2)
EUCLIDEAN_SCALED_GATE = GATE + EUCLIDEAN_SHIFT


def row(arity: int) -> dict[str, object]:
    beta_hat = B + 2 * T * arity * B_DIGIT * GAMMA
    b_sync = max(2 * beta_hat, 8 * beta_hat * GAMMA, 2 * B_DIGIT)
    b_prod = 2 * (arity + 1) * beta_hat * (2 * GAMMA) ** (2 * arity)
    log_prod_inf = math.log2(b_prod)
    log_sync_inf = math.log2(b_sync)
    log_prod_2 = log_prod_inf + EUCLIDEAN_SHIFT
    log_sync_2 = log_sync_inf + EUCLIDEAN_SHIFT
    base_sync_loss_bits = PHI * math.log2(3) - math.log2(2 * arity)
    communication_bits = Q_BITS * (
        arity * (A_PRIME + 1) * PHI
        + (K + 2) * (arity + 1) * PHI * E
        + (2 + arity * (2 * B_DIGIT + 2)) * ELL_1 * E
    )
    return {
        "L": arity,
        "beta_hat": beta_hat,
        "B_prod": b_prod,
        "B_sync": b_sync,
        "log2_B_prod": log_prod_inf,
        "log2_B_sync": log_sync_inf,
        "log2_l2_prod": log_prod_2,
        "log2_l2_sync": log_sync_2,
        "base_sync_loss_bits": base_sync_loss_bits,
        "communication_KiB": communication_bits / (8 * 1024),
        "log2_centered_gate": GATE,
        "log2_euclidean_scaled_gate": EUCLIDEAN_SCALED_GATE,
        "product_gate": "nontrivial" if log_prod_inf < GATE else "trivial",
        "sync_gate": "nontrivial" if log_sync_inf < GATE else "trivial",
    }


rows = [row(L) for L in (1, 2, 4, 8)]

# The arity-two separation must be checked in one metric at a time.
arity_two = next(item for item in rows if item["L"] == 2)
assert arity_two["log2_B_sync"] < GATE < arity_two["log2_B_prod"]
assert (
    arity_two["log2_l2_sync"]
    < EUCLIDEAN_SCALED_GATE
    < arity_two["log2_l2_prod"]
)

with (OUT / "product_vs_sync.csv").open("w", newline="", encoding="utf-8") as f:
    writer = csv.DictWriter(f, fieldnames=list(rows[0]))
    writer.writeheader()
    writer.writerows(rows)

tex = [
    r"\begin{table}[t]",
    r"\centering",
    r"\caption{Formula-derived product-clearing and synchronized bounds on the pinned exact-strong parameter line.  The last two radius columns apply $\|z\|_2\le\sqrt{m\ell\phi}\|z\|_\infty$ for Euclidean-SIS estimation.  The gate columns compare the coefficient bounds $B_{\rm prod},B_{\rm sync}$ with the coefficientwise centered threshold $(q_{\rm exact}-1)/2$.}",
    r"\label{tab:product-sync}",
    r"\scriptsize",
    r"\begin{tabular}{r@{\quad}r@{\quad}r@{\quad}r@{\quad}r@{\quad}c@{\quad}c}",
    r"\toprule",
    r"$L$ & $\log_2 B_{\rm prod}$ & $\log_2 B_{\rm sync}$ & $\log_2\|z_{\rm prod}\|_2$ & $\log_2\|z_{\rm sync}\|_2$ & product & sync\\",
    r"\midrule",
]
for r in rows:
    prod = "nontriv." if r["product_gate"] == "nontrivial" else "triv."
    sync = "nontriv." if r["sync_gate"] == "nontrivial" else "triv."
    tex.append(
        f'{r["L"]}&{r["log2_B_prod"]:.4f}&{r["log2_B_sync"]:.4f}'
        f'&{r["log2_l2_prod"]:.4f}&{r["log2_l2_sync"]:.4f}'
        f'&{prod}&{sync}\\\\'
    )
tex.extend([r"\bottomrule", r"\end{tabular}", r"\end{table}", ""])
(OUT / "product_vs_sync_table.tex").write_text("\n".join(tex), encoding="utf-8")

for r in rows:
    print(r)
