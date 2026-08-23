//! Exact fixed-Hamming-weight signed ternary challenge sampler for
//! R = Z[X]/(X^N + 1).
//!
//! The manuscript uses the uniform distribution
//!
//! ```text
//! D_tau = { c in {-1,0,1}^N : wt(c) = tau }.
//! ```
//!
//! The sampler below realizes this distribution directly: partial Fisher--Yates
//! chooses a uniform support of size `TAU`, and independent sign bits choose the
//! +/-1 coefficients.  There is deliberately no spectral-norm rejection step.
//!
//! For coefficient-l_infinity analysis, negacyclic multiplication by a signed
//! ternary polynomial c has induced operator norm
//!
//! ```text
//! ||M_c||_{infinity -> infinity} = sum_j |c_j| = wt(c) = TAU.
//! ```
//!
//! The optional spectral routines below are kept only for diagnostics.  Without
//! spectral rejection, the rigorous generic spectral bound is also at most
//! `TAU` by the triangle inequality.

use crate::common::config::{DEGREE, MOD_Q};
use crate::common::hash::HashWrapper;
use crate::common::ring_arithmetic::{Representation, RingElement};
use num::Complex;
use std::sync::LazyLock;

/// Baseline manuscript line: exact weight 32.  This meets a 128-bit
/// two-star unit-loss target for L <= 8 in the interactive case (M = 1).
///
/// In the output-selected final-query ROM theorem the unit term is 2*L*M/|D_tau|,
/// where M = Q_fin + 1.  Thus TAU must be re-sized when M > 1; weight 32 is
/// not a universal 128-bit ROM choice.  Enable `challenge-weight-34` for the
/// large-arity benchmark; at L = 128 it covers the 2*L*M ledger up to M = 2.
#[cfg(not(feature = "challenge-weight-34"))]
pub const TAU: usize = 32;
#[cfg(feature = "challenge-weight-34")]
pub const TAU: usize = 34;

/// Exact coefficient-l_infinity multiplication norm of every sampled challenge.
pub const COEFF_LINF_OP_NORM_BOUND: usize = TAU;

/// Rigorous deterministic l2->l2 spectral bound when no spectral rejection is
/// performed.  This is intentionally separate from the manuscript's
/// coefficient-l_infinity norm to avoid mixing the two metrics.
pub const SPECTRAL_OP_NORM_SAFE_BOUND: f64 = TAU as f64;

/// log2 |D_TAU| = log2(binomial(N, TAU)) + TAU.
///
/// This helper is parameter bookkeeping only; it is not used by the sampler.
pub fn challenge_support_log2() -> f64 {
    let mut bits = TAU as f64; // independent +/- signs
    for i in 0..TAU {
        bits += ((N - i) as f64).log2() - ((i + 1) as f64).log2();
    }
    bits
}

/// Inverse-bit size of the two-star exact-strong outer unit term
/// 2*L*M*kappa_fix when kappa_fix = 1/|D_TAU|.
///
/// This formula is valid on the manuscript's exact-strong modulus line.  The
/// repository's default quadratic-splitting modulus needs a separate
/// fixed-weight CRT anti-concentration argument before this value can be used
/// as a theorem-level unit-failure bound.
pub fn exact_strong_outer_unit_bits(l: usize, m: usize) -> f64 {
    assert!(l > 0 && m > 0);
    challenge_support_log2() - (2.0 * l as f64 * m as f64).log2()
}

const N: usize = DEGREE;
const LOG_N: usize = N.trailing_zeros() as usize;
const PHASE_LEN: usize = 2 * N;
const PHASE_MASK: usize = PHASE_LEN - 1;

static PRE_TWIST: LazyLock<[Complex<f64>; N]> = LazyLock::new(|| {
    let mut arr = [Complex::new(0.0, 0.0); N];
    for j in 0..N {
        let angle = std::f64::consts::PI * (j as f64) / (N as f64);
        arr[j] = Complex::new(angle.cos(), angle.sin());
    }
    arr
});

static PHASE_RE: LazyLock<[f64; PHASE_LEN]> = LazyLock::new(|| {
    let mut arr = [0.0f64; PHASE_LEN];
    for m in 0..PHASE_LEN {
        let angle = std::f64::consts::PI * (m as f64) / (N as f64);
        arr[m] = angle.cos();
    }
    arr
});
static PHASE_IM: LazyLock<[f64; PHASE_LEN]> = LazyLock::new(|| {
    let mut arr = [0.0f64; PHASE_LEN];
    for m in 0..PHASE_LEN {
        let angle = std::f64::consts::PI * (m as f64) / (N as f64);
        arr[m] = angle.sin();
    }
    arr
});

static BIT_REVERSE: LazyLock<[usize; N]> = LazyLock::new(|| {
    let mut arr = [0usize; N];
    for i in 0..N {
        let mut x = i;
        let mut r = 0usize;
        for _ in 0..LOG_N {
            r = (r << 1) | (x & 1);
            x >>= 1;
        }
        arr[i] = r;
    }
    arr
});

fn fft_in_place(a: &mut [Complex<f64>; N]) {
    let br = &*BIT_REVERSE;
    for i in 0..N {
        let j = br[i];
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= N {
        let half = len / 2;
        let theta = 2.0 * std::f64::consts::PI / (len as f64);
        let w_step = Complex::new(theta.cos(), theta.sin());
        let mut start = 0usize;
        while start < N {
            let mut w = Complex::new(1.0_f64, 0.0_f64);
            for j in 0..half {
                let t = w * a[start + half + j];
                let u = a[start + j];
                a[start + j] = u + t;
                a[start + half + j] = u - t;
                w *= w_step;
            }
            start += len;
        }
        len <<= 1;
    }
}

pub fn op_norm(c: &[i8; N]) -> f64 {
    let pt = &*PRE_TWIST;
    let mut a = [Complex::new(0.0, 0.0); N];
    for j in 0..N {
        if c[j] != 0 {
            let s = c[j] as f64;
            a[j] = pt[j] * s;
        }
    }
    fft_in_place(&mut a);
    let mut max_sq = 0.0f64;
    for k in 0..(N / 2) {
        let m = a[k].norm_sqr();
        if m > max_sq {
            max_sq = m;
        }
    }
    max_sq.sqrt()
}

#[inline(always)]
pub fn op_norm_sq_sparse(positions: &[u8; TAU], signs: &[i8; TAU]) -> f64 {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
    unsafe {
        return op_norm_sq_sparse_avx512(positions, signs);
    }
    #[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
    op_norm_sq_sparse_scalar(positions, signs)
}

pub fn op_norm_sq_sparse_scalar(positions: &[u8; TAU], signs: &[i8; TAU]) -> f64 {
    let phase_re: &[f64; PHASE_LEN] = &PHASE_RE;
    let phase_im: &[f64; PHASE_LEN] = &PHASE_IM;

    let mut v_re = [0.0f64; N / 2];
    let mut v_im = [0.0f64; N / 2];

    for k in 0..TAU {
        let p = positions[k] as usize;
        let s = signs[k] as f64;
        let step = (2 * p) & PHASE_MASK;
        let mut idx = p & PHASE_MASK;
        for j in 0..(N / 2) {
            let pr = unsafe { *phase_re.get_unchecked(idx) };
            let pi = unsafe { *phase_im.get_unchecked(idx) };
            v_re[j] += s * pr;
            v_im[j] += s * pi;
            idx = (idx + step) & PHASE_MASK;
        }
    }

    let mut max_sq = 0.0f64;
    for j in 0..(N / 2) {
        let m = v_re[j] * v_re[j] + v_im[j] * v_im[j];
        if m > max_sq {
            max_sq = m;
        }
    }
    max_sq
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
#[inline(always)]
unsafe fn op_norm_sq_sparse_avx512(positions: &[u8; TAU], signs: &[i8; TAU]) -> f64 {
    use std::arch::x86_64::*;

    const NUM_BATCHES: usize = N / 16;
    const _: () = assert!(N % 16 == 0 && NUM_BATCHES * 2 <= 32);

    let phase_re_ptr = PHASE_RE.as_ptr();
    let phase_im_ptr = PHASE_IM.as_ptr();

    let mut vr = [_mm512_setzero_pd(); NUM_BATCHES];
    let mut vi = [_mm512_setzero_pd(); NUM_BATCHES];

    let mask_v = _mm256_set1_epi32(PHASE_MASK as i32);
    let lane_index = _mm256_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7);

    for k in 0..TAU {
        let p = positions[k] as i32;
        let s = signs[k] as f64;
        let s_v = _mm512_set1_pd(s);
        let step = (2 * p) & (PHASE_MASK as i32);

        let step_v = _mm256_set1_epi32(step);
        let lane_offsets = _mm256_mullo_epi32(lane_index, step_v);
        let big_step = _mm256_set1_epi32(8 * step);

        let mut base = _mm256_add_epi32(_mm256_set1_epi32(p), lane_offsets);
        for b in 0..NUM_BATCHES {
            let idx_v = _mm256_and_si256(base, mask_v);
            let pre = _mm512_i32gather_pd::<8>(idx_v, phase_re_ptr);
            let pim = _mm512_i32gather_pd::<8>(idx_v, phase_im_ptr);
            vr[b] = _mm512_fmadd_pd(s_v, pre, vr[b]);
            vi[b] = _mm512_fmadd_pd(s_v, pim, vi[b]);
            base = _mm256_add_epi32(base, big_step);
        }
    }

    let mut mm = _mm512_fmadd_pd(vr[0], vr[0], _mm512_mul_pd(vi[0], vi[0]));
    for b in 1..NUM_BATCHES {
        let m = _mm512_fmadd_pd(vr[b], vr[b], _mm512_mul_pd(vi[b], vi[b]));
        mm = _mm512_max_pd(mm, m);
    }
    _mm512_reduce_max_pd(mm)
}

pub fn op_norm_direct(c: &[i8; N]) -> f64 {
    let mut max_sq = 0.0f64;
    let two_n = 2 * N;
    for j in (1..two_n).step_by(2) {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for k in 0..N {
            if c[k] == 0 {
                continue;
            }
            let angle = std::f64::consts::PI * (j as f64) * (k as f64) / (N as f64);
            let s = c[k] as f64;
            re += s * angle.cos();
            im += s * angle.sin();
        }
        let m = re * re + im * im;
        if m > max_sq {
            max_sq = m;
        }
    }
    max_sq.sqrt()
}

const ATTEMPT_LABEL: &[u8] = b"exact-fixed-weight-ternary-challenge";
const ATTEMPT_BUF_LEN: usize = 128;
const SIGN_BYTES: usize = (TAU + 7) / 8;

const _: () = assert!(N <= 256, "Fisher-Yates uses one byte per index sample");

#[allow(unused_assignments)]
fn sample_attempt(hasher: &mut HashWrapper) -> ([u8; TAU], [i8; TAU]) {
    let mut buf = [0u8; ATTEMPT_BUF_LEN];
    hasher.fill_from_xof(ATTEMPT_LABEL, &mut buf);
    let mut idx: usize = 0;

    macro_rules! next_byte {
        () => {{
            if idx >= ATTEMPT_BUF_LEN {
                hasher.fill_from_xof(ATTEMPT_LABEL, &mut buf);
                idx = 0;
            }
            let b = buf[idx];
            idx += 1;
            b
        }};
    }

    let mut perm: [u8; N] = std::array::from_fn(|i| i as u8);
    for i in 0..TAU {
        let range = (N - i) as u32;
        let cutoff: u32 = 256 - (256 % range);
        let r = loop {
            let b = next_byte!() as u32;
            if b < cutoff {
                break b;
            }
        };
        let j = i + (r % range) as usize;
        perm.swap(i, j);
    }

    let mut sign_bytes = [0u8; SIGN_BYTES];
    for b in &mut sign_bytes {
        *b = next_byte!();
    }

    let mut positions = [0u8; TAU];
    let mut signs = [0i8; TAU];
    for i in 0..TAU {
        positions[i] = perm[i];
        let bit = (sign_bytes[i / 8] >> (i % 8)) & 1;
        signs[i] = if bit == 1 { 1 } else { -1 };
    }
    (positions, signs)
}

/// Sample uniformly from the exact-weight signed ternary family D_TAU.
///
/// The second return value is retained for API compatibility with the original
/// rejection sampler.  It is always 1 because the rigorous manuscript sampler
/// does not perform spectral rejection.
pub fn sample_short_challenge(hasher: &mut HashWrapper) -> ([i8; N], usize) {
    let (positions, signs) = sample_attempt(hasher);
    let mut c = [0i8; N];
    for i in 0..TAU {
        c[positions[i] as usize] = signs[i];
    }
    (c, 1)
}

pub fn sample_short_challenge_into(hasher: &mut HashWrapper, output: &mut RingElement) -> usize {
    let (c, attempts) = sample_short_challenge(hasher);
    output.representation = Representation::Coefficients;
    for j in 0..N {
        output.v[j] = match c[j] {
            -1 => MOD_Q - 1,
            0 => 0,
            1 => 1,
            _ => unreachable!(),
        };
    }
    output.to_representation(Representation::IncompleteNTT);
    attempts
}

pub fn repetition_rate() -> f64 {
    // Kept for compatibility with the existing binary/reporting code.
    // There is no rejection loop in the manuscript-adapted sampler.
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weight(c: &[i8; N]) -> usize {
        c.iter().filter(|&&x| x != 0).count()
    }

    fn fixed_test_vector() -> ([u8; TAU], [i8; TAU]) {
        let mut positions = [0u8; TAU];
        let mut signs = [0i8; TAU];
        for i in 0..TAU {
            positions[i] = ((i * 13 + 1) % N) as u8;
            signs[i] = if i % 3 == 0 { -1 } else { 1 };
        }
        (positions, signs)
    }

    #[test]
    fn fft_op_norm_matches_direct_on_fixed_vector() {
        let (positions, signs) = fixed_test_vector();
        let mut c = [0i8; N];
        for i in 0..TAU {
            c[positions[i] as usize] = signs[i];
        }
        let fft = op_norm(&c);
        let direct = op_norm_direct(&c);
        assert!(
            (fft - direct).abs() < 1e-10,
            "fft={} direct={}",
            fft,
            direct
        );
    }

    #[test]
    fn sparse_op_norm_matches_dense_fft() {
        let (positions, signs) = fixed_test_vector();
        let mut c = [0i8; N];
        for i in 0..TAU {
            c[positions[i] as usize] = signs[i];
        }
        let dense = op_norm(&c);
        let sparse = op_norm_sq_sparse(&positions, &signs).sqrt();
        assert!(
            (dense - sparse).abs() < 1e-10,
            "dense={} sparse={}",
            dense,
            sparse
        );
    }

    #[test]
    fn manuscript_entropy_ledger_matches_selected_weight() {
        let bits = challenge_support_log2();
        #[cfg(not(feature = "challenge-weight-34"))]
        {
            assert!((bits - 132.22130063735855).abs() < 1e-10);
            assert!(exact_strong_outer_unit_bits(8, 1) > 128.0);
            assert!(exact_strong_outer_unit_bits(8, 2) < 128.0);
        }
        #[cfg(feature = "challenge-weight-34")]
        {
            assert!((bits - 137.24426178580188).abs() < 1e-10);
            assert!(exact_strong_outer_unit_bits(128, 2) > 128.0);
            assert!(exact_strong_outer_unit_bits(128, 4) < 128.0);
        }
    }

    #[test]
    fn samples_have_exact_weight_and_rigorous_norm_bounds() {
        let mut hasher = HashWrapper::new();
        for _ in 0..10_000 {
            let (c, attempts) = sample_short_challenge(&mut hasher);
            assert_eq!(attempts, 1);
            assert_eq!(weight(&c), TAU);
            assert_eq!(COEFF_LINF_OP_NORM_BOUND, TAU);
            for &x in &c {
                assert!(x == -1 || x == 0 || x == 1);
            }

            // Diagnostic only: ||M_c||_{2->2} <= ||c||_1 = TAU.
            let spectral = op_norm(&c);
            assert!(
                spectral <= SPECTRAL_OP_NORM_SAFE_BOUND + 1e-10,
                "spectral norm {} > rigorous bound {}",
                spectral,
                SPECTRAL_OP_NORM_SAFE_BOUND
            );
        }
    }

    #[test]
    fn sampling_is_deterministic_and_post_state_matches() {
        let mut h1 = HashWrapper::new();
        let mut h2 = HashWrapper::new();
        for _ in 0..20 {
            let (c1, _) = sample_short_challenge(&mut h1);
            let (c2, _) = sample_short_challenge(&mut h2);
            assert_eq!(c1, c2);
        }
        assert_eq!(h1.sample_bytes(64), h2.sample_bytes(64));
    }

    #[test]
    fn challenge_stream_fingerprint() {
        let mut hasher = HashWrapper::new();
        let mut digest = blake3::Hasher::new();
        for _ in 0..100 {
            let (c, _) = sample_short_challenge(&mut hasher);
            let bytes: [u8; N] = std::array::from_fn(|i| c[i] as u8);
            digest.update(&bytes);
        }
        let actual = digest.finalize().to_hex().to_string();
        assert_eq!(actual, EXPECTED_FINGERPRINT);
    }

    #[cfg(not(feature = "challenge-weight-34"))]
    const EXPECTED_FINGERPRINT: &str =
        "0d38d7c02e497fbe84bb0f5e04829059c8232b39285c0060f47b97e22484636b";
    #[cfg(feature = "challenge-weight-34")]
    const EXPECTED_FINGERPRINT: &str =
        "2e555dcacc6409b80937c17c27d1411a6dd8b480b82432b12dc60d55adeb1aae";

    #[test]
    fn ring_output_encoding_is_consistent() {
        let mut hasher = HashWrapper::new();
        let mut elem = RingElement::new(Representation::Coefficients);
        let _ = sample_short_challenge_into(&mut hasher, &mut elem);
        assert_eq!(elem.representation, Representation::IncompleteNTT);
        elem.to_representation(Representation::Coefficients);
        let mut nonzero = 0;
        for &v in &elem.v {
            assert!(v == 0 || v == 1 || v == MOD_Q - 1);
            if v != 0 {
                nonzero += 1;
            }
        }
        assert_eq!(nonzero, TAU);
    }
}
