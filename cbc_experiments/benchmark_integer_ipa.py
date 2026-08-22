#!/usr/bin/env python3
import math
import os
import platform
import random
import statistics
import time
import json
import pandas as pd
import matplotlib.pyplot as plt
from pathlib import Path

OUT = Path(__file__).resolve().parent / "results"
OUT.mkdir(exist_ok=True)

def fu_old_logq(r, lam=128, logB=64):
    return (
        (2*r + 16 + 8*math.log2(r))*lam
        + 64*r*r + logB + 9
    )

def fu_new_logq(r, lam=128, logB=64):
    return (
        (2*r + 8 + 3*math.log2(r) + math.log2(r-1))*lam
        + 32*r*r - 16*r + 11 + logB
    )

def median_pow_time(exp_bits, mod_bits, reps=5, seed=1):
    rnd = random.Random(
        seed + exp_bits*131 + mod_bits
    )
    mod = (
        (1 << (mod_bits-1))
        | rnd.getrandbits(mod_bits-1)
        | 1
    )
    base = 5
    exps = [
        (1 << (exp_bits-1))
        | rnd.getrandbits(exp_bits-1)
        for _ in range(reps)
    ]
    pow(base, exps[0], mod)  # warm-up
    ts = []
    for e in exps:
        t0 = time.perf_counter()
        pow(base, e, mod)
        ts.append(time.perf_counter()-t0)
    return statistics.median(ts)

def horner_encoding_time(n, q_bits, reps=3, seed=7):
    rnd = random.Random(seed+n*17+q_bits)
    q = (
        (1 << (q_bits-1))
        | rnd.getrandbits(q_bits-1)
        | 1
    )
    digits = [
        rnd.randrange(-(1<<15), (1<<15))
        for _ in range(n)
    ]
    ts = []
    outbits = 0
    for _ in range(reps):
        t0 = time.perf_counter()
        acc = 0
        for x in reversed(digits):
            acc = acc*q + x
        ts.append(time.perf_counter()-t0)
        outbits = abs(acc).bit_length()
    return statistics.median(ts), outbits

def main():
    rows = []
    for r in range(2, 9):
        n = 2**r
        old_bits = math.ceil(fu_old_logq(r))
        new_bits = math.ceil(fu_new_logq(r))

        h_old, out_old = horner_encoding_time(
            n, old_bits
        )
        h_new, out_new = horner_encoding_time(
            n, new_bits
        )

        row = {
            "r": r,
            "n": n,
            "old_log2q_bits": old_bits,
            "new_log2q_bits": new_bits,
            "horner_old_seconds": h_old,
            "horner_new_seconds": h_new,
            "horner_speedup": h_old/h_new,
            "horner_reduction_pct":
                100*(1-h_new/h_old),
            "encoded_integer_bits_old": out_old,
            "encoded_integer_bits_new": out_new,
        }

        for mod_bits in (2048, 3072):
            p_old = median_pow_time(
                old_bits, mod_bits, reps=5, seed=11
            )
            p_new = median_pow_time(
                new_bits, mod_bits, reps=5, seed=11
            )
            row[f"powmod_{mod_bits}_old_seconds"] = p_old
            row[f"powmod_{mod_bits}_new_seconds"] = p_new
            row[f"powmod_{mod_bits}_speedup"] = p_old/p_new
            row[f"powmod_{mod_bits}_reduction_pct"] = (
                100*(1-p_new/p_old)
            )
            # Projected sequential preprocessing:
            # q^i g, q^i h via 2(n-1) q-bit scalar steps.
            row[
                f"precomp_projected_{mod_bits}_old_seconds"
            ] = 2*(n-1)*p_old
            row[
                f"precomp_projected_{mod_bits}_new_seconds"
            ] = 2*(n-1)*p_new

        rows.append(row)

    df = pd.DataFrame(rows)
    df.to_csv(
        OUT / "integer_ipa_microbenchmark.csv",
        index=False
    )
    print(df.to_string(index=False))

    # Direct no-precomputation proxy for manageable r.
    direct_rows = []
    for r in range(2, 5):
        n = 2**r
        old_q = math.ceil(fu_old_logq(r))
        new_q = math.ceil(fu_new_logq(r))
        for mod_bits in (2048, 3072):
            old_t = median_pow_time(
                n*old_q, mod_bits, reps=3, seed=23
            )
            new_t = median_pow_time(
                n*new_q, mod_bits, reps=3, seed=23
            )
            direct_rows.append({
                "r": r,
                "n": n,
                "modulus_bits": mod_bits,
                "encoded_scalar_bits_old": n*old_q,
                "encoded_scalar_bits_new": n*new_q,
                "old_seconds": old_t,
                "new_seconds": new_t,
                "speedup": old_t/new_t,
                "reduction_pct":
                    100*(1-new_t/old_t),
            })

    ddf = pd.DataFrame(direct_rows)
    ddf.to_csv(
        OUT / "integer_ipa_direct_no_precomp_proxy.csv",
        index=False
    )

    env = {
        "python": platform.python_version(),
        "platform": platform.platform(),
        "logical_cpus": os.cpu_count(),
        "note": (
            "Python arbitrary-precision integers and "
            "pow(base, exponent, modulus) are used. "
            "Modular exponentiation is a group-scalar-cost proxy, "
            "not a Fu protocol implementation."
        ),
    }
    try:
        with open("/proc/cpuinfo", "r") as f:
            for line in f:
                if line.lower().startswith("model name"):
                    env["cpu_model"] = (
                        line.split(":", 1)[1].strip()
                    )
                    break
    except Exception:
        pass

    with open(
        OUT / "benchmark_environment.json",
        "w", encoding="utf-8"
    ) as f:
        json.dump(env, f, indent=2, ensure_ascii=False)

    plt.figure(figsize=(7.2, 4.8))
    plt.plot(
        df["r"], df["powmod_2048_speedup"],
        marker="o", label="2048-bit modulus"
    )
    plt.plot(
        df["r"], df["powmod_3072_speedup"],
        marker="o", label="3072-bit modulus"
    )
    plt.xlabel("Recursion depth r")
    plt.ylabel("Measured scalar-multiplication proxy speedup")
    plt.title(
        "Integer IPA: shorter q in modular-exponentiation microbenchmark"
    )
    plt.legend()
    plt.tight_layout()
    plt.savefig(
        OUT / "integer_ipa_group_proxy_speedup.png",
        dpi=180
    )
    plt.close()

    plt.figure(figsize=(7.2, 4.8))
    plt.plot(
        df["r"], df["horner_speedup"],
        marker="o"
    )
    plt.xlabel("Recursion depth r")
    plt.ylabel("Measured speedup")
    plt.title(
        "Integer IPA: base-q big-integer encoding speedup"
    )
    plt.tight_layout()
    plt.savefig(
        OUT / "integer_ipa_horner_speedup.png",
        dpi=180
    )
    plt.close()

if __name__ == "__main__":
    main()
