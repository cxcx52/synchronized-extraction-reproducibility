#!/usr/bin/env python3
import math
import pandas as pd
import matplotlib.pyplot as plt
from pathlib import Path

OUT = Path(__file__).resolve().parent / "results"
OUT.mkdir(exist_ok=True)

# Minimal reproduction of the Euclidean SIS path used by the Cyclo
# estimates.ipynb pinned to lattice-estimator commit 352ddaf.
# This script implements the relevant root-Hermite-factor inversion
# and MATZOV reduction-cost formulas directly.

MATZOV_A = 0.29613500308205365
MATZOV_B = 20.387885985467914

def rc_delta(beta: int) -> float:
    small = (
        (2, 1.02190), (5, 1.01862), (10, 1.01616), (15, 1.01485),
        (20, 1.01420), (25, 1.01342), (28, 1.01331), (40, 1.01295),
    )
    if beta <= 2:
        return 1.0219
    if beta < 40:
        for i in range(1, len(small)):
            if small[i][0] > beta:
                return small[i-1][1]
    if beta == 40:
        return small[-1][1]
    return (beta / (2*math.pi*math.e) *
            (math.pi*beta)**(1/beta))**(1/(2*(beta-1)))

def beta_from_delta(delta: float) -> int:
    if rc_delta(40) < delta:
        return 40
    lo, hi = 40, 2**16
    while lo < hi:
        mid = (lo + hi) // 2
        if rc_delta(mid) <= delta:
            hi = mid
        else:
            lo = mid + 1
    return lo

def d4f(beta: int) -> float:
    return max(
        float(beta * math.log(4/3.0) /
              math.log(beta/(2*math.pi*math.e))),
        0.0,
    )

def matzov_log2_cost(beta: int, d: int) -> float:
    C = 1.0 / (1.0 - 2**(-MATZOV_A))
    svp_calls = C * max(d-beta, 1)
    beta_eff = beta - d4f(beta)
    log_gate = math.log2(C) + MATZOV_A*beta_eff + MATZOV_B
    log_sieve = math.log2(svp_calls) + log_gate
    log_lll = 3*math.log2(d)
    mx = max(log_sieve, log_lll)
    return mx + math.log2(
        2**(log_sieve-mx) + 2**(log_lll-mx)
    )

def sis_euclidean(n: int, m: int, q: int, length_bound: float):
    logq = math.log2(q)
    lognu = math.log2(length_bound)

    # Euclidean SIS optimal lattice dimension.
    log_delta_sq = lognu**2 / (4*n*logq)
    dopt = math.sqrt(n*logq / log_delta_sq)
    d = min(math.floor(dopt), int(m))

    target_delta = 2**((lognu - (n/d)*logq)/(d-1))
    beta = beta_from_delta(target_delta)
    reduction_possible = beta <= d
    if not reduction_possible:
        beta = d

    # Lower-bound predicate used by estimator.
    log_A_sq = math.log(n * math.log(q))
    log_B_sq = math.log(d) + 2*(n/d)*math.log(q)
    if log_A_sq <= log_B_sq:
        lb = math.sqrt(n*math.log(q))
    else:
        lb = math.sqrt(d) * math.exp((n/d)*math.log(q))

    if not (length_bound > lb and reduction_possible):
        return {
            "security_bits": math.inf,
            "beta": beta,
            "d": d,
            "delta": rc_delta(beta),
            "target_delta": target_delta,
        }

    return {
        "security_bits": matzov_log2_cost(beta, d),
        "beta": beta,
        "d": d,
        "delta": rc_delta(beta),
        "target_delta": target_delta,
    }

# Cyclo parameters used in the manuscript comparison.
N = 128
WITNESS_M = 2**20
ELL = 11
M_EXT = N * WITNESS_M * ELL
Q_EXACT = 2**50 - 351
T = 64
B_INIT = 1024
B_DIGIT = 1
EUCLID_SHIFT = 0.5 * math.log2(M_EXT)

def old_notebook_sis_break(inf_b, gamma=128, L=1):
    betahat = inf_b*((2*gamma)**L) + L*2*inf_b*((2*gamma)**(L-1))
    delta = (2*gamma)**L
    return 2*delta*betahat

def sanity_check():
    # Cached notebook baseline: kappa=13, L=1, gamma=128 -> ~127.1 bits.
    notebook_inf_b = B_INIT + 2*T*128
    length = (
        old_notebook_sis_break(notebook_inf_b, 128, 1)
        * math.sqrt(M_EXT)
    )
    return sis_euclidean(
        N*13, M_EXT, 2**50, length
    )

def main():
    sanity = sanity_check()
    print("Sanity check:")
    print(sanity)

    rows = []
    for L in range(1, 9):
        gamma_old = 128
        beta_old = B_INIT + 2*T*L*B_DIGIT*gamma_old
        B_old = 8*beta_old*gamma_old
        logR_old = math.log2(B_old) + EUCLID_SHIFT

        gamma_new = 32
        beta_new = B_INIT + 2*T*L*B_DIGIT*gamma_new
        B_new = 8*beta_new*gamma_new
        logR_new = math.log2(B_new) + EUCLID_SHIFT

        old_by_rank = {}
        new_by_rank = {}
        for rank in range(6, 14):
            old_by_rank[rank] = sis_euclidean(
                N*rank, M_EXT, Q_EXACT, 2**logR_old
            )
            new_by_rank[rank] = sis_euclidean(
                N*rank, M_EXT, Q_EXACT, 2**logR_new
            )

        min_old = next(
            r for r in range(6, 14)
            if old_by_rank[r]["security_bits"] >= 128
        )
        min_new = next(
            r for r in range(6, 14)
            if new_by_rank[r]["security_bits"] >= 128
        )

        rows.append({
            "L": L,
            "old_gamma": gamma_old,
            "old_beta_hat": beta_old,
            "old_global_B_inf": B_old,
            "old_log2_euclidean_radius": logR_old,
            "old_min_rank_128": min_old,
            "old_security_at_min_rank":
                old_by_rank[min_old]["security_bits"],
            "old_BKZ_beta_at_min_rank":
                old_by_rank[min_old]["beta"],
            "new_gamma": gamma_new,
            "new_beta_hat": beta_new,
            "new_global_B_inf": B_new,
            "new_log2_euclidean_radius": logR_new,
            "new_min_rank_128": min_new,
            "new_security_at_min_rank":
                new_by_rank[min_new]["security_bits"],
            "new_BKZ_beta_at_min_rank":
                new_by_rank[min_new]["beta"],
            "rank_reduction": min_old-min_new,
            "radius_improvement_bits": logR_old-logR_new,
            "radius_factor": B_old/B_new,
        })

    df = pd.DataFrame(rows)
    df.to_csv(OUT / "cyclo_estimator_rerun.csv", index=False)
    print("\nGlobal scan:")
    print(df.to_string(index=False))

    # Detailed L=2 rank curve.
    L = 2
    B_old = 8*(B_INIT+2*T*L*128)*128
    B_new = 8*(B_INIT+2*T*L*32)*32
    log_old = math.log2(B_old)+EUCLID_SHIFT
    log_new = math.log2(B_new)+EUCLID_SHIFT

    rank_rows = []
    for rank in range(6, 14):
        old = sis_euclidean(
            N*rank, M_EXT, Q_EXACT, 2**log_old
        )
        new = sis_euclidean(
            N*rank, M_EXT, Q_EXACT, 2**log_new
        )
        rank_rows.append({
            "rank": rank,
            "security_old_full_ternary_bits":
                old["security_bits"],
            "BKZ_beta_old": old["beta"],
            "security_new_fixed_weight32_bits":
                new["security_bits"],
            "BKZ_beta_new": new["beta"],
        })
    rdf = pd.DataFrame(rank_rows)
    rdf.to_csv(
        OUT / "cyclo_L2_rank_security.csv",
        index=False
    )

    plt.figure(figsize=(7.2, 4.8))
    plt.plot(
        df["L"], df["old_min_rank_128"],
        marker="o", label="Full ternary"
    )
    plt.plot(
        df["L"], df["new_min_rank_128"],
        marker="o", label="Fixed weight 32"
    )
    plt.xlabel("Number of folded relations L")
    plt.ylabel("Minimum Ajtai rank for >=128-bit SIS estimate")
    plt.title("Cyclo: estimator rerun under rigorous global radii")
    plt.legend()
    plt.tight_layout()
    plt.savefig(
        OUT / "cyclo_min_rank_128.png",
        dpi=180
    )
    plt.close()

    plt.figure(figsize=(7.2, 4.8))
    plt.plot(
        rdf["rank"],
        rdf["security_old_full_ternary_bits"],
        marker="o", label="Full ternary (L=2)"
    )
    plt.plot(
        rdf["rank"],
        rdf["security_new_fixed_weight32_bits"],
        marker="o", label="Fixed weight 32 (L=2)"
    )
    plt.axhline(128, linestyle="--")
    plt.xlabel("Ajtai rank")
    plt.ylabel("Estimated SIS security (bits)")
    plt.title("Cyclo L=2: security versus rank")
    plt.legend()
    plt.tight_layout()
    plt.savefig(
        OUT / "cyclo_L2_security_vs_rank.png",
        dpi=180
    )
    plt.close()

if __name__ == "__main__":
    main()
