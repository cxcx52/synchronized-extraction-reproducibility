use crate::common::config::*;
use crate::hexl::bindings::*;
use crate::protocol::config::SizeableProof;
use rand::{Rng, SeedableRng};
use std::cell::RefCell;
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};
use std::sync::LazyLock;

#[cfg(feature = "quartic-q")]
use crate::common::quartic_ring::QuarticTransform;

#[cfg(feature = "quartic-q")]
pub static QUARTIC_TRANSFORM: LazyLock<QuarticTransform> = LazyLock::new(QuarticTransform::new);

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Representation {
    Coefficients, // This should not be used almost ever. Use only for printing or debugging.
    EvenOddCoefficients, // Coefficients grouped by their residue modulo EXTENSION_DEGREE. Historical name retained for compatibility.
    IncompleteNTT,       // Each residue block is transformed independently.
    HomogenizedFieldExtensions, // All CRT slots use one common degree-EXTENSION_DEGREE field presentation.
}

// DO NOT derive Copy here, as RingElement is large.
// align(64) ensures `v` starts on a cache-line boundary so the AVX-512
// kernel can use aligned 512-bit loads/stores without crossing cache lines.
#[derive(PartialEq, Clone, Debug)]
#[repr(C, align(64))]
pub struct RingElement {
    pub v: [u64; DEGREE],
    pub representation: Representation,
}

thread_local! {
    static RNG: RefCell<rand::rngs::StdRng> =
        RefCell::new(rand::rngs::StdRng::from_os_rng());
}

pub fn seed_rng(seed: &str) {
    let seed_bytes: [u8; 32] = *blake3::hash(seed.as_bytes()).as_bytes();
    RNG.with(|cell| {
        *cell.borrow_mut() = rand::rngs::StdRng::from_seed(seed_bytes);
    });
}

impl RingElement {
    pub const fn new(representation: Representation) -> Self {
        Self {
            v: [0; DEGREE],
            representation,
        }
    }

    pub fn random(representation: Representation) -> Self {
        let mut element = Self {
            v: [0; DEGREE],
            representation,
        };

        RNG.with(|cell| {
            let mut rng = cell.borrow_mut();
            for i in 0..DEGREE {
                element.v[i] = rng.random_range(0..MOD_Q);
            }
        });

        element
    }

    pub fn one(representation: Representation) -> Self {
        let mut element = Self {
            v: [0; DEGREE],
            representation: Representation::EvenOddCoefficients,
        };
        element.v[0] = 1;

        element.to_representation(representation);

        element
    }

    pub fn all(value: u64, representation: Representation) -> Self {
        let mut element = Self {
            v: [0; DEGREE],
            representation: Representation::EvenOddCoefficients,
        };
        for i in 0..DEGREE {
            element.v[i] = value;
        }

        element.to_representation(representation);

        element
    }

    pub fn zero(representation: Representation) -> Self {
        let mut element = Self {
            v: [0; DEGREE],
            representation,
        };
        element.v[0] = 0;

        element
    }

    pub fn constant(value: u64, representation: Representation) -> Self {
        let mut element = Self {
            v: [0; DEGREE],
            representation: Representation::EvenOddCoefficients,
        };

        element.v[0] = value;

        element.to_representation(representation);

        element
    }

    pub fn random_bounded(representation: Representation, bound: u64) -> Self {
        let mut element = Self {
            v: [0; DEGREE],
            representation: Representation::Coefficients,
        };

        RNG.with(|cell| {
            let mut rng = cell.borrow_mut();
            for i in 0..DEGREE {
                let val = rng.random_range(0..bound);
                // Use a single random u64 bit to decide sign
                element.v[i] = if (rng.random::<u8>() & 1) == 0 {
                    val
                } else {
                    MOD_Q - val
                };
            }
        });
        unsafe {
            eltwise_reduce_mod(
                element.v.as_mut_ptr(),
                element.v.as_mut_ptr(),
                element.v.len() as u64,
                MOD_Q,
            );
        }

        element.to_representation(representation);

        element
    }

    pub fn random_bounded_unsigned(representation: Representation, bound: u64) -> Self {
        let mut element = Self {
            v: [0; DEGREE],
            representation: Representation::Coefficients,
        };

        RNG.with(|cell| {
            let mut rng = cell.borrow_mut();
            for i in 0..DEGREE {
                element.v[i] = rng.random_range(0..bound);
            }
        });

        element.to_representation(representation);

        element
    }

    pub fn from_even_odd_coefficients_to_incomplete_ntt_representation(&mut self) {
        debug_assert!(
            self.representation == Representation::EvenOddCoefficients,
            "Already in Incomplete NTT representation"
        );

        unsafe {
            for residue in 0..EXTENSION_DEGREE {
                ntt_forward_in_place(
                    self.v.as_mut_ptr().add(residue * HALF_DEGREE),
                    HALF_DEGREE,
                    MOD_Q,
                );
            }
        }

        self.representation = Representation::IncompleteNTT;
    }

    pub fn from_incomplete_ntt_to_even_odd_coefficients(&mut self) {
        debug_assert!(
            self.representation == Representation::IncompleteNTT,
            "Not in Incomplete NTT representation"
        );

        unsafe {
            for residue in 0..EXTENSION_DEGREE {
                ntt_inverse_in_place(
                    self.v.as_mut_ptr().add(residue * HALF_DEGREE),
                    HALF_DEGREE,
                    MOD_Q,
                );
            }
        }

        self.representation = Representation::EvenOddCoefficients;
    }

    pub fn from_coefficients_to_even_odd_coefficients(&mut self) {
        debug_assert!(
            self.representation == Representation::Coefficients,
            "Not in Coefficients representation"
        );

        let mut temp = [0u64; DEGREE];

        for residue in 0..EXTENSION_DEGREE {
            for digit in 0..HALF_DEGREE {
                temp[residue * HALF_DEGREE + digit] = self.v[EXTENSION_DEGREE * digit + residue];
            }
        }

        self.v = temp;
        self.representation = Representation::EvenOddCoefficients;
    }

    pub fn from_even_odd_coefficients_to_coefficients(&mut self) {
        debug_assert!(
            self.representation == Representation::EvenOddCoefficients,
            "Not in Even-Odd Coefficients representation"
        );

        let mut temp = [0u64; DEGREE];

        for residue in 0..EXTENSION_DEGREE {
            for digit in 0..HALF_DEGREE {
                temp[EXTENSION_DEGREE * digit + residue] = self.v[residue * HALF_DEGREE + digit];
            }
        }

        self.v = temp;
        self.representation = Representation::Coefficients;
    }

    pub fn from_incomplete_ntt_to_homogenized_field_extensions(&mut self) {
        debug_assert!(
            self.representation == Representation::IncompleteNTT,
            "Not in Incomplete NTT representation"
        );

        #[cfg(feature = "quartic-q")]
        {
            self.v = QUARTIC_TRANSFORM.raw_ntt_layout_to_homogeneous(&self.v);
        }
        #[cfg(not(feature = "quartic-q"))]
        unsafe {
            eltwise_mult_mod(
                self.v.as_mut_ptr().add(HALF_DEGREE),
                self.v.as_ptr().add(HALF_DEGREE),
                NORMALIZE_INCOMPLETE_NTT_FACTORS.as_ptr(),
                (HALF_DEGREE) as u64,
                MOD_Q,
            );
        }
        self.representation = Representation::HomogenizedFieldExtensions;
    }

    pub fn from_homogenized_field_extensions_to_incomplete_ntt(&mut self) {
        debug_assert!(
            self.representation == Representation::HomogenizedFieldExtensions,
            "Not in Homogenized Field Extensions representation"
        );

        #[cfg(feature = "quartic-q")]
        {
            self.v = QUARTIC_TRANSFORM.homogeneous_to_raw_ntt_layout(&self.v);
        }
        #[cfg(not(feature = "quartic-q"))]
        unsafe {
            eltwise_mult_mod(
                self.v.as_mut_ptr().add(HALF_DEGREE),
                self.v.as_ptr().add(HALF_DEGREE),
                NORMALIZE_INCOMPLETE_NTT_FACTORS_INVERSE.as_ptr(),
                (HALF_DEGREE) as u64,
                MOD_Q,
            );
        }
        self.representation = Representation::IncompleteNTT;
    }

    pub fn to_representation(&mut self, representation: Representation) {
        match (self.representation, representation) {
            (Representation::Coefficients, Representation::EvenOddCoefficients) => {
                self.from_coefficients_to_even_odd_coefficients()
            }
            (Representation::Coefficients, Representation::IncompleteNTT) => {
                self.from_coefficients_to_even_odd_coefficients();
                self.from_even_odd_coefficients_to_incomplete_ntt_representation();
            }
            (Representation::Coefficients, Representation::HomogenizedFieldExtensions) => {
                self.from_coefficients_to_even_odd_coefficients();
                self.from_even_odd_coefficients_to_incomplete_ntt_representation();
                self.from_incomplete_ntt_to_homogenized_field_extensions();
            }
            (Representation::EvenOddCoefficients, Representation::IncompleteNTT) => {
                self.from_even_odd_coefficients_to_incomplete_ntt_representation()
            }
            (Representation::EvenOddCoefficients, Representation::HomogenizedFieldExtensions) => {
                self.from_even_odd_coefficients_to_incomplete_ntt_representation();
                self.from_incomplete_ntt_to_homogenized_field_extensions();
            }
            (Representation::IncompleteNTT, Representation::HomogenizedFieldExtensions) => {
                self.from_incomplete_ntt_to_homogenized_field_extensions()
            }
            (Representation::HomogenizedFieldExtensions, Representation::IncompleteNTT) => {
                self.from_homogenized_field_extensions_to_incomplete_ntt()
            }
            (Representation::HomogenizedFieldExtensions, Representation::EvenOddCoefficients) => {
                self.from_homogenized_field_extensions_to_incomplete_ntt();
                self.from_incomplete_ntt_to_even_odd_coefficients();
            }
            (Representation::HomogenizedFieldExtensions, Representation::Coefficients) => {
                self.from_homogenized_field_extensions_to_incomplete_ntt();
                self.from_incomplete_ntt_to_even_odd_coefficients();
                self.from_even_odd_coefficients_to_coefficients();
            }
            (Representation::IncompleteNTT, Representation::EvenOddCoefficients) => {
                self.from_incomplete_ntt_to_even_odd_coefficients();
            }
            (Representation::IncompleteNTT, Representation::Coefficients) => {
                self.from_incomplete_ntt_to_even_odd_coefficients();
                self.from_even_odd_coefficients_to_coefficients();
            }
            (Representation::EvenOddCoefficients, Representation::Coefficients) => {
                self.from_even_odd_coefficients_to_coefficients();
            }
            _ => {
                // nothing to do
            }
        }
    }

    // Probably should never be used
    pub fn split_into_quadratic_extensions(&self) -> [QuadraticExtension; HALF_DEGREE] {
        debug_assert!(
            self.representation == Representation::HomogenizedFieldExtensions,
            "RingElement not in Homogenized Field Extensions representation"
        );

        let mut result = [QuadraticExtension::zero(); HALF_DEGREE];

        for i in 0..HALF_DEGREE {
            for coefficient in 0..EXTENSION_DEGREE {
                result[i].coeffs[coefficient] = self.v[i + coefficient * HALF_DEGREE];
            }
        }

        result
    }

    pub fn combine_from_quadratic_extensions(
        &mut self,
        extensions: &[QuadraticExtension; HALF_DEGREE],
    ) {
        debug_assert!(
            self.representation == Representation::HomogenizedFieldExtensions,
            "RingElement not in Homogenized Field Extensions representation"
        );

        for i in 0..HALF_DEGREE {
            for coefficient in 0..EXTENSION_DEGREE {
                self.v[i + coefficient * HALF_DEGREE] = extensions[i].coeffs[coefficient];
            }
        }
    }

    #[inline]
    pub fn set_zero(&mut self) {
        self.v.fill(0);
    }

    fn conjugate_in_place_ref(&mut self) {
        // True Galois conjugation: X -> X^{-1} = -X^{n-1} in Z_q[X]/(X^n + 1)
        // In coefficient form: [c_0, c_1, ..., c_{n-1}] -> [c_0, -c_{n-1}, -c_{n-2}, ..., -c_1]
        // Reference implementation used for deriving NTT-domain transformations
        debug_assert_eq!(self.representation, Representation::IncompleteNTT);
        self.from_incomplete_ntt_to_even_odd_coefficients();
        self.from_even_odd_coefficients_to_coefficients();

        // Reverse and negate coefficients 1 to n-1
        for i in 1..(DEGREE / 2 + 1) {
            let temp = self.v[i];
            self.v[i] = if self.v[DEGREE - i] == 0 {
                0
            } else {
                MOD_Q - self.v[DEGREE - i]
            };
            self.v[DEGREE - i] = if temp == 0 { 0 } else { MOD_Q - temp };
        }

        self.from_coefficients_to_even_odd_coefficients();
        self.from_even_odd_coefficients_to_incomplete_ntt_representation();
    }

    #[inline]
    pub fn set_from(&mut self, other: &RingElement) {
        self.v.copy_from_slice(&other.v);
        self.representation = other.representation;
    }

    pub fn conjugate_in_place(&mut self) {
        // True Galois conjugation: X -> X^{-1} = -X^{n-1} in Z_q[X]/(X^n + 1)
        // Pure NTT-domain implementation using empirically derived transformations
        //
        // PERFORMANCE: O(n) vs O(n log n)
        // This implementation: 0 NTT transforms, pure element-wise permutation and multiplication
        // Reference (coefficient space): 4 NTT transforms required
        //
        // MATHEMATICAL FOUNDATION:
        // =======================
        // Conjugation in coefficient space: [c_0, c_1, ..., c_{n-1}] -> [c_0, -c_{n-1}, ..., -c_1]
        // This reverses and negates all non-constant coefficients.
        //
        // In IncompleteNTT representation: f(X) = f_even(X^2) + X·f_odd(X^2)
        // Conjugation: f(X^{-1}) = f_even(X^{-2}) - X^{n-1}·f_odd(X^{-2})
        //
        // NTT-DOMAIN TRANSFORMATION:
        // =========================
        // Rather than analytically deriving how conjugation acts on NTT coefficients
        // (which requires deep knowledge of HEXL's root ordering and evaluation points),
        // we use PRECOMPUTED PERMUTATIONS AND FACTORS derived empirically.
        //
        // The empirical approach:
        // 1. For each basis vector e_i in IncompleteNTT space
        // 2. Apply conjugation via coefficient space (ground truth)
        // 3. Observe where e_i maps to and what scaling factor is applied
        // 4. Build lookup tables: CONJUGATION_NTT_TRANSFORM
        //
        // This gives us:
        // - even_permutation[i]: where even[i] goes after conjugation
        // - odd_permutation[i]: where odd[i] goes after conjugation
        // - odd_factors[i]: scaling factor for odd[i] (accounts for X -> -X^{n-1})
        //
        // IMPLEMENTATION:
        // ==============
        // Apply the precomputed transformation directly in NTT space:
        // - even_new[even_permutation[i]] = even_old[i]
        // - odd_new[odd_permutation[i]] = odd_old[i] * odd_factors[i]
        //
        // Benefits:
        // - No NTT transforms needed (pure O(n) operation)
        // - Provably correct (matches reference implementation by construction)
        // - Robust to HEXL implementation details

        debug_assert_eq!(self.representation, Representation::IncompleteNTT);

        #[cfg(feature = "quartic-q")]
        {
            let transform = &*QUARTIC_CONJUGATION_NTT_TRANSFORM;
            let mut result = [0u64; DEGREE];
            for source in 0..DEGREE {
                result[transform.permutation[source]] =
                    unsafe { multiply_mod(self.v[source], transform.factors[source], MOD_Q) };
            }
            self.v = result;
            return;
        }

        #[cfg(not(feature = "quartic-q"))]
        {
            let transform = &*CONJUGATION_NTT_TRANSFORM;
            let mut temp = [0u64; DEGREE];

            // Apply even part permutation
            for i in 0..HALF_DEGREE {
                temp[transform.even_permutation[i]] = self.v[i];
            }
            self.v[..HALF_DEGREE].copy_from_slice(&temp[..HALF_DEGREE]);

            // Apply odd part: multiply by factors, then permute
            unsafe {
                eltwise_mult_mod(
                    temp.as_mut_ptr(),
                    self.v.as_ptr().add(HALF_DEGREE),
                    transform.odd_factors.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
            }
            for i in 0..HALF_DEGREE {
                self.v[HALF_DEGREE + transform.odd_permutation[i]] = temp[i];
            }
        }
    }

    #[inline]
    pub fn conjugate_into(&self, result: &mut RingElement) {
        debug_assert_eq!(self.representation, Representation::IncompleteNTT);
        result.representation = self.representation;

        #[cfg(feature = "quartic-q")]
        {
            let transform = &*QUARTIC_CONJUGATION_NTT_TRANSFORM;
            for source in 0..DEGREE {
                result.v[transform.permutation[source]] =
                    unsafe { multiply_mod(self.v[source], transform.factors[source], MOD_Q) };
            }
            return;
        }

        #[cfg(not(feature = "quartic-q"))]
        {
            let transform = &*CONJUGATION_NTT_TRANSFORM;
            let mut temp = [0u64; DEGREE];
            for i in 0..HALF_DEGREE {
                temp[transform.even_permutation[i]] = self.v[i];
            }
            result.v[..HALF_DEGREE].copy_from_slice(&temp[..HALF_DEGREE]);
            unsafe {
                eltwise_mult_mod(
                    temp.as_mut_ptr(),
                    self.v.as_ptr().add(HALF_DEGREE),
                    transform.odd_factors.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
            }
            for i in 0..HALF_DEGREE {
                result.v[HALF_DEGREE + transform.odd_permutation[i]] = temp[i];
            }
        }
    }

    pub fn conjugate(&self) -> RingElement {
        let mut result = RingElement::new(self.representation);
        self.conjugate_into(&mut result);
        result
    }

    pub fn negate(&self) -> RingElement {
        let zero = RingElement::zero(self.representation);
        &zero - self
    }

    // 1 us
    pub fn inverse(&self) -> RingElement {
        assert_eq!(
            self.representation,
            Representation::HomogenizedFieldExtensions
        );

        #[cfg(feature = "quartic-q")]
        {
            return RingElement {
                v: QUARTIC_TRANSFORM
                    .inverse_homogeneous_layout(&self.v)
                    .expect("cannot invert a ring element with a zero CRT slot"),
                representation: Representation::HomogenizedFieldExtensions,
            };
        }

        #[cfg(not(feature = "quartic-q"))]
        {
            // Each default slot is Z_q[X]/(X^2 - beta).  Montgomery batch
            // inversion computes all slot-norm inverses with one base-field
            // inversion.
            let beta = *FIELD_SHIFT_FACTOR;
            let mut result = RingElement::new(Representation::HomogenizedFieldExtensions);
            let mut norms = [0u64; HALF_DEGREE];
            let mut temp = [0u64; HALF_DEGREE];

            unsafe {
                eltwise_mult_mod(
                    norms.as_mut_ptr(),
                    self.v.as_ptr(),
                    self.v.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                eltwise_mult_mod(
                    temp.as_mut_ptr(),
                    self.v.as_ptr().add(HALF_DEGREE),
                    self.v.as_ptr().add(HALF_DEGREE),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                eltwise_fma_mod(
                    norms.as_mut_ptr(),
                    temp.as_ptr(),
                    MOD_Q - beta,
                    norms.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
            }

            let mut prefix_products = [0u64; HALF_DEGREE];
            prefix_products[0] = norms[0];
            for i in 1..HALF_DEGREE {
                prefix_products[i] =
                    unsafe { multiply_mod(prefix_products[i - 1], norms[i], MOD_Q) };
            }
            let mut inv = unsafe { inv_mod(prefix_products[HALF_DEGREE - 1], MOD_Q) };
            let mut norm_inverses = [0u64; HALF_DEGREE];
            for i in (1..HALF_DEGREE).rev() {
                norm_inverses[i] = unsafe { multiply_mod(inv, prefix_products[i - 1], MOD_Q) };
                inv = unsafe { multiply_mod(inv, norms[i], MOD_Q) };
            }
            norm_inverses[0] = inv;

            unsafe {
                eltwise_mult_mod(
                    result.v.as_mut_ptr(),
                    self.v.as_ptr(),
                    norm_inverses.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                eltwise_mult_mod(
                    result.v.as_mut_ptr().add(HALF_DEGREE),
                    self.v.as_ptr().add(HALF_DEGREE),
                    norm_inverses.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                temp.fill(0);
                eltwise_sub_mod(
                    result.v.as_mut_ptr().add(HALF_DEGREE),
                    temp.as_ptr(),
                    result.v.as_ptr().add(HALF_DEGREE),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
            }
            result
        }
    }

    pub fn constant_term_from_incomplete_ntt(&self) -> u64 {
        debug_assert_eq!(self.representation, Representation::IncompleteNTT);
        #[cfg(feature = "quartic-q")]
        {
            let mut coefficients = self.clone();
            coefficients.from_incomplete_ntt_to_even_odd_coefficients();
            coefficients.from_even_odd_coefficients_to_coefficients();
            return coefficients.v[0];
        }

        #[cfg(not(feature = "quartic-q"))]
        {
            let mut buf = [0u64; DEGREE];
            buf.copy_from_slice(&self.v);
            unsafe {
                eltwise_mult_mod(
                    buf.as_mut_ptr(),
                    self.v.as_ptr(),
                    CONSTANT_TERM_FACTORS.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
            }
            let mut sum = 0u64;
            for i in 0..HALF_DEGREE {
                sum += buf[i];
            }

            // we call it once so it's probably fine
            sum % MOD_Q
        }
    }
}

pub static CONSTANT_TERM_FACTORS: LazyLock<[u64; HALF_DEGREE]> = LazyLock::new(|| {
    let scale = unsafe { inv_mod(HALF_DEGREE as u64, MOD_Q) };
    let mut factors = RingElement::one(Representation::IncompleteNTT);
    unsafe {
        for i in 0..HALF_DEGREE {
            factors.v[i] = multiply_mod(scale, inv_mod(factors.v[i], MOD_Q), MOD_Q);
        }
    }
    factors.v[..HALF_DEGREE].try_into().unwrap()
});

pub static SHIFT_FACTORS: LazyLock<[u64; HALF_DEGREE]> = LazyLock::new(|| {
    let mut factors = [0u64; HALF_DEGREE];
    factors[1] = 1;
    unsafe { ntt_forward_in_place(factors.as_mut_ptr(), factors.len(), MOD_Q) };
    factors
});

pub static FIELD_SHIFT_FACTOR: LazyLock<u64> = LazyLock::new(|| SHIFT_FACTORS[0]);

pub static INV_HALF_DEGREE: LazyLock<u64> =
    LazyLock::new(|| unsafe { power_mod(HALF_DEGREE as u64, MOD_Q - 2, MOD_Q) });

pub static TWO_INV_HALF_DEGREE: LazyLock<u64> =
    LazyLock::new(|| unsafe { multiply_mod(2, *INV_HALF_DEGREE, MOD_Q) });

/// Precomputed permutation and factors for NTT-domain conjugation
/// Generated by analyzing how conjugation transforms NTT coefficients empirically
pub static CONJUGATION_NTT_TRANSFORM: LazyLock<ConjugationTransform> =
    LazyLock::new(|| derive_conjugation_transform());

#[cfg(feature = "quartic-q")]
pub static QUARTIC_CONJUGATION_NTT_TRANSFORM: LazyLock<QuarticConjugationTransform> =
    LazyLock::new(derive_quartic_conjugation_transform);

#[derive(Clone, Debug)]
pub struct ConjugationTransform {
    pub even_permutation: [usize; HALF_DEGREE],
    pub odd_permutation: [usize; HALF_DEGREE],
    pub odd_factors: [u64; HALF_DEGREE],
}

#[cfg(feature = "quartic-q")]
#[derive(Clone, Debug)]
pub struct QuarticConjugationTransform {
    pub permutation: [usize; DEGREE],
    pub factors: [u64; DEGREE],
}

#[cfg(feature = "quartic-q")]
fn derive_quartic_conjugation_transform() -> QuarticConjugationTransform {
    let mut permutation = [0usize; DEGREE];
    let mut factors = [0u64; DEGREE];
    let mut seen_destinations = [false; DEGREE];

    for source in 0..DEGREE {
        let mut basis = RingElement::new(Representation::IncompleteNTT);
        basis.v[source] = 1;
        basis.conjugate_in_place_ref();

        let mut nonzero = basis
            .v
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, value)| *value != 0);
        let (destination, factor) = nonzero
            .next()
            .expect("quartic NTT conjugation must map a basis vector nontrivially");
        assert!(
            nonzero.next().is_none(),
            "quartic NTT conjugation must be monomial"
        );
        assert!(!seen_destinations[destination]);
        seen_destinations[destination] = true;
        permutation[source] = destination;
        factors[source] = factor;
    }
    assert!(seen_destinations.into_iter().all(|seen| seen));
    QuarticConjugationTransform {
        permutation,
        factors,
    }
}

/// Empirically derive the conjugation transformation in NTT domain
///
/// METHODOLOGY:
/// ============
/// We cannot analytically derive the NTT-domain conjugation without deep knowledge
/// of HEXL's internal NTT implementation details. Instead, we use an empirical approach:
///
/// 1. Generate test basis vectors in IncompleteNTT (one-hot encoded)
/// 2. For each basis vector:
///    a. Convert to coefficient space
///    b. Apply conjugation (reverse and negate non-constant coefficients)
///    c. Convert back to IncompleteNTT
///    d. Observe where the value moved and what factor was applied
/// 3. Build permutation tables and factor arrays from observations
///
/// This approach is robust because:
/// - It directly measures HEXL's actual behavior
/// - No assumptions about root ordering or evaluation points
/// - Automatically handles any HEXL implementation details
/// - Will continue working even if HEXL internals change (as long as we regenerate)
fn derive_conjugation_transform() -> ConjugationTransform {
    let mut even_permutation = [0usize; HALF_DEGREE];
    let mut odd_permutation = [0usize; HALF_DEGREE];
    let mut odd_factors = [0u64; HALF_DEGREE];

    // Derive even part permutation
    // Test each position in the even part
    for i in 0..HALF_DEGREE {
        let mut test_vec = RingElement::new(Representation::IncompleteNTT);
        test_vec.v[i] = 1; // One-hot encode position i in even part

        // Apply reference conjugation (via coefficient space)
        let mut conjugated = test_vec.clone();
        conjugated.conjugate_in_place_ref();

        // Find where the 1 moved to in the even part
        for j in 0..HALF_DEGREE {
            if conjugated.v[j] != 0 {
                even_permutation[i] = j;
                break;
            }
        }
    }

    // Derive odd part permutation and factors
    // Test each position in the odd part
    for i in 0..HALF_DEGREE {
        let mut test_vec = RingElement::new(Representation::IncompleteNTT);
        test_vec.v[HALF_DEGREE + i] = 1; // One-hot encode position i in odd part

        // Apply reference conjugation
        let mut conjugated = test_vec.clone();
        conjugated.conjugate_in_place_ref();

        // Find where the value moved to and what factor was applied
        for j in 0..HALF_DEGREE {
            if conjugated.v[HALF_DEGREE + j] != 0 {
                odd_permutation[i] = j;
                // The factor is the value at the new position
                // (since we started with 1)
                odd_factors[i] = conjugated.v[HALF_DEGREE + j];
                break;
            }
        }
    }

    ConjugationTransform {
        even_permutation,
        odd_permutation,
        odd_factors,
    }
}

///// Helpers

pub fn addition(result: &mut RingElement, operand1: &RingElement, operand2: &RingElement) {
    debug_assert!(
        operand1.representation == operand2.representation,
        "Operands have different representations"
    );
    debug_assert!(
        result.representation == operand1.representation,
        "Result has different representation than operands"
    );

    unsafe {
        eltwise_add_mod(
            result.v.as_mut_ptr(),
            operand1.v.as_ptr(),
            operand2.v.as_ptr(),
            DEGREE as u64,
            MOD_Q,
        );
    }
}
pub fn addition_in_place(result_op1: &mut RingElement, operand2: &RingElement) {
    debug_assert!(
        result_op1.representation == operand2.representation,
        "Operands have different representations"
    );

    unsafe {
        eltwise_add_mod(
            result_op1.v.as_mut_ptr(),
            result_op1.v.as_ptr(),
            operand2.v.as_ptr(),
            DEGREE as u64,
            MOD_Q,
        );
    }
}

pub fn subtraction(result: &mut RingElement, operand1: &RingElement, operand2: &RingElement) {
    debug_assert!(
        operand1.representation == operand2.representation,
        "Operands have different representations"
    );
    debug_assert!(
        result.representation == operand1.representation,
        "Result has different representation than operands"
    );

    unsafe {
        eltwise_sub_mod(
            result.v.as_mut_ptr(),
            operand1.v.as_ptr(),
            operand2.v.as_ptr(),
            DEGREE as u64,
            MOD_Q,
        );
    }
}

pub fn subtraction_in_place(result_op1: &mut RingElement, operand2: &RingElement) {
    debug_assert!(
        result_op1.representation == operand2.representation,
        "Operands have different representations"
    );

    unsafe {
        eltwise_sub_mod(
            result_op1.v.as_mut_ptr(),
            result_op1.v.as_ptr(),
            operand2.v.as_ptr(),
            DEGREE as u64,
            MOD_Q,
        );
    }
}

#[inline(always)]
pub fn incomplete_ntt_multiplication(
    result: &mut RingElement,
    operand1: &RingElement,
    operand2: &RingElement,
) {
    debug_assert!(
        operand1.representation == Representation::IncompleteNTT,
        "Operand1 not in Incomplete NTT representation"
    );
    debug_assert!(
        operand2.representation == Representation::IncompleteNTT,
        "Operand2 not in Incomplete NTT representation"
    );
    debug_assert!(
        result.representation == Representation::IncompleteNTT,
        "Result not in Incomplete NTT representation"
    );

    incomplete_ntt_multiplication_inner(result, operand1, operand2, false);
}

#[inline(always)]
pub fn incomplete_ntt_multiplication_in_place(result: &mut RingElement, operand: &RingElement) {
    debug_assert!(
        operand.representation == Representation::IncompleteNTT,
        "Operand not in Incomplete NTT representation"
    );
    debug_assert!(
        result.representation == Representation::IncompleteNTT,
        "Result not in Incomplete NTT representation"
    );

    #[cfg(feature = "quartic-q")]
    {
        let lhs = result.clone();
        incomplete_ntt_multiplication_inner(result, &lhs, operand, false);
        return;
    }

    #[cfg(not(feature = "quartic-q"))]
    {
        // The fused AVX512 kernel loads all inputs into registers before any
        // store within each 8-element iteration, so result can alias operand1.
        unsafe {
            fused_incomplete_ntt_mult(
                result.v.as_mut_ptr(),
                result.v.as_ptr(),
                operand.v.as_ptr(),
                SHIFT_FACTORS.as_ptr(),
                HALF_DEGREE,
                MOD_Q,
            );
        }
    }
}

pub fn incomplete_ntt_multiplication_homogenized(
    result: &mut RingElement,
    operand1: &RingElement,
    operand2: &RingElement,
) {
    debug_assert!(
        operand1.representation == Representation::HomogenizedFieldExtensions,
        "Operand1 not in Homogenized Field Extensions representation"
    );
    debug_assert!(
        operand2.representation == Representation::HomogenizedFieldExtensions,
        "Operand2 not in Homogenized Field Extensions representation"
    );
    debug_assert!(
        result.representation == Representation::HomogenizedFieldExtensions,
        "Result not in Homogenized Field Extensions representation"
    );
    incomplete_ntt_multiplication_inner(result, operand1, operand2, true);
}

#[inline(always)]
pub fn incomplete_ntt_multiplication_inner(
    result: &mut RingElement,
    operand1: &RingElement,
    operand2: &RingElement,
    homogenized: bool,
) {
    #[cfg(feature = "quartic-q")]
    {
        result.v = if homogenized {
            QUARTIC_TRANSFORM.multiply_homogeneous_layout(&operand1.v, &operand2.v)
        } else {
            QUARTIC_TRANSFORM.multiply_raw_ntt_layout(&operand1.v, &operand2.v)
        };
        return;
    }

    #[cfg(not(feature = "quartic-q"))]
    {
        let op1_data = &operand1.v;
        let op2_data = &operand2.v;

        if !homogenized {
            // Fused path: all 5 mults + 2 adds in a single AVX512 pass.
            // Eliminates per-call dispatch overhead, redundant int↔float
            // conversions, and intermediate memory traffic.
            unsafe {
                fused_incomplete_ntt_mult(
                    result.v.as_mut_ptr(),
                    op1_data.as_ptr(),
                    op2_data.as_ptr(),
                    SHIFT_FACTORS.as_ptr(),
                    HALF_DEGREE,
                    MOD_Q,
                );
            }
            return;
        }

        // Homogenized path: keep original separate-call implementation
        let mut temp = [0u64; DEGREE];

        unsafe {
            // result_even = op1_even * op2_even
            eltwise_mult_mod(
                result.v.as_mut_ptr(),
                op1_data.as_ptr(),
                op2_data.as_ptr(),
                HALF_DEGREE as u64,
                MOD_Q,
            );

            // result_odd = op1_odd * op2_even
            eltwise_mult_mod(
                result.v.as_mut_ptr().add(HALF_DEGREE),
                op1_data.as_ptr().add(HALF_DEGREE),
                op2_data.as_ptr(),
                HALF_DEGREE as u64,
                MOD_Q,
            );

            // temp = op1_odd * op2_odd
            eltwise_mult_mod(
                temp.as_mut_ptr(),
                op1_data.as_ptr().add(HALF_DEGREE),
                op2_data.as_ptr().add(HALF_DEGREE),
                HALF_DEGREE as u64,
                MOD_Q,
            );

            // result_even += temp * SHIFT_FACTORS[0]
            eltwise_fma_mod(
                result.v.as_mut_ptr(),
                temp.as_ptr(),
                SHIFT_FACTORS[0],
                result.v.as_ptr(),
                HALF_DEGREE as u64,
                MOD_Q,
            );

            // Reuse temp for op1_even * op2_odd
            eltwise_mult_mod(
                temp.as_mut_ptr(),
                op1_data.as_ptr(),
                op2_data.as_ptr().add(HALF_DEGREE),
                HALF_DEGREE as u64,
                MOD_Q,
            );

            // result_odd += temp
            eltwise_add_mod(
                result.v.as_mut_ptr().add(HALF_DEGREE),
                result.v.as_ptr().add(HALF_DEGREE),
                temp.as_ptr(),
                HALF_DEGREE as u64,
                MOD_Q,
            );
        }
    }
}

#[inline(always)]
pub fn incomplete_ntt_multiplication_in_place_inner(
    result: &mut RingElement,
    operand1: &RingElement,
    homogenized: bool,
) {
    #[cfg(feature = "quartic-q")]
    {
        let rhs = result.clone();
        incomplete_ntt_multiplication_inner(result, operand1, &rhs, homogenized);
        return;
    }

    #[cfg(not(feature = "quartic-q"))]
    {
        let mut temp = [0u64; DEGREE];

        let op1_data = &operand1.v;

        unsafe {
            // result_even = op1_even * op2_even
            eltwise_mult_mod(
                result.v.as_mut_ptr(),
                op1_data.as_ptr(),
                result.v.as_ptr(),
                HALF_DEGREE as u64,
                MOD_Q,
            );

            // result_odd = op1_odd * op2_even
            eltwise_mult_mod(
                result.v.as_mut_ptr().add(HALF_DEGREE),
                op1_data.as_ptr().add(HALF_DEGREE),
                result.v.as_ptr(),
                HALF_DEGREE as u64,
                MOD_Q,
            );

            // temp = op1_odd * op2_odd
            eltwise_mult_mod(
                temp.as_mut_ptr(),
                op1_data.as_ptr().add(HALF_DEGREE),
                result.v.as_ptr().add(HALF_DEGREE),
                HALF_DEGREE as u64,
                MOD_Q,
            );

            if homogenized {
                // result_even += temp * SHIFT_FACTORS[0]
                eltwise_fma_mod(
                    result.v.as_mut_ptr(),
                    temp.as_ptr(),
                    SHIFT_FACTORS[0],
                    result.v.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
            } else {
                // Apply shift factors
                eltwise_mult_mod(
                    temp.as_mut_ptr(),
                    temp.as_ptr(),
                    SHIFT_FACTORS.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );

                // result_even += temp
                eltwise_add_mod(
                    result.v.as_mut_ptr(),
                    result.v.as_ptr(),
                    temp.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
            }

            // Reuse temp for op1_even * op2_odd
            eltwise_mult_mod(
                temp.as_mut_ptr(),
                op1_data.as_ptr(),
                result.v.as_ptr().add(HALF_DEGREE),
                HALF_DEGREE as u64,
                MOD_Q,
            );

            // result_odd += temp
            eltwise_add_mod(
                result.v.as_mut_ptr().add(HALF_DEGREE),
                result.v.as_ptr().add(HALF_DEGREE),
                temp.as_ptr(),
                HALF_DEGREE as u64,
                MOD_Q,
            );
        }
    }
}

pub fn naive_polynomial_multiplication(
    result: &mut RingElement,
    operand1: &RingElement,
    operand2: &RingElement,
) {
    debug_assert!(
        operand1.representation == Representation::Coefficients,
        "Operand1 not in Coefficients representation"
    );
    debug_assert!(
        operand2.representation == Representation::Coefficients,
        "Operand2 not in Coefficients representation"
    );
    debug_assert!(
        result.representation == Representation::Coefficients,
        "Result not in Coefficients representation"
    );

    for i in 0..DEGREE {
        result.v[i] = 0;
    }

    for i in 0..DEGREE {
        for j in 0..DEGREE {
            let index = (i + j) % DEGREE;
            let prod = (operand1.v[i] as u128 * operand2.v[j] as u128) % (MOD_Q as u128);
            if i + j >= DEGREE {
                // Negation in modular arithmetic: MOD_Q - value
                let neg_prod = (MOD_Q as u128 - prod) % (MOD_Q as u128);
                result.v[index] = (result.v[index] + neg_prod as u64) % MOD_Q;
            } else {
                result.v[index] = (result.v[index] + prod as u64) % MOD_Q;
            }
        }
    }
}

pub static NORMALIZE_INCOMPLETE_NTT_FACTORS: LazyLock<[u64; HALF_DEGREE]> =
    LazyLock::new(|| get_roots_of_unity_trans().0);

pub static NORMALIZE_INCOMPLETE_NTT_FACTORS_INVERSE: LazyLock<[u64; HALF_DEGREE]> =
    LazyLock::new(|| get_roots_of_unity_trans().1);

pub fn get_roots_of_unity_trans() -> ([u64; HALF_DEGREE], [u64; HALF_DEGREE]) {
    let mut roots_translations = [0u64; HALF_DEGREE];
    for i in 0..HALF_DEGREE {
        let mut t = 0;
        while (|| {
            let mut ex = RingElement::new(Representation::IncompleteNTT);
            ex.v[HALF_DEGREE + i] = 1;
            let mut ex_0 = RingElement::new(Representation::IncompleteNTT);
            incomplete_ntt_multiplication_inner(&mut ex_0, &ex, &ex, false);
            let mut ex_1 = RingElement::new(Representation::HomogenizedFieldExtensions);

            ex.v[HALF_DEGREE + i] = unsafe { power_mod(SHIFT_FACTORS[i], t, MOD_Q) };
            incomplete_ntt_multiplication_inner(&mut ex_1, &ex, &ex, true);
            ex_0.v != ex_1.v
        })() {
            t = t + 1;
        }
        roots_translations[i] = unsafe { power_mod(SHIFT_FACTORS[i], t, MOD_Q) };
    }

    let mut roots_translations_inv = [0u64; HALF_DEGREE];

    for i in 0..HALF_DEGREE {
        roots_translations_inv[i] = unsafe { inv_mod(roots_translations[i], MOD_Q) };
    }

    (roots_translations, roots_translations_inv)
}

impl Add for &RingElement {
    type Output = RingElement;

    fn add(self, other: Self) -> Self::Output {
        let mut result = RingElement::new(self.representation);
        addition(&mut result, &self, &other);
        result
    }
}

impl AddAssign<&RingElement> for RingElement {
    fn add_assign(&mut self, other: &Self) {
        addition_in_place(self, other);
    }
}

impl Mul for &RingElement {
    type Output = RingElement;

    fn mul(self, other: Self) -> Self::Output {
        let mut result = RingElement::new(self.representation);
        incomplete_ntt_multiplication(&mut result, self, other);
        result
    }
}

impl MulAssign<&RingElement> for RingElement {
    fn mul_assign(&mut self, other: &Self) {
        incomplete_ntt_multiplication_in_place(self, other);
    }
}

impl Sub for &RingElement {
    type Output = RingElement;

    fn sub(self, other: Self) -> Self::Output {
        let mut result = RingElement::new(self.representation);
        subtraction(&mut result, self, other);
        result
    }
}

impl SubAssign<&RingElement> for RingElement {
    fn sub_assign(&mut self, other: &Self) {
        subtraction_in_place(self, other);
    }
}

// Methods below are a bit unorthodox, but they allow to avoid cloning when using
// addition with references.
// In this case, a += (&b, &c) means a = b + c, but without cloning b and c.

impl AddAssign<(&RingElement, &RingElement)> for RingElement {
    fn add_assign(&mut self, other: (&RingElement, &RingElement)) {
        let (op1, op2) = other;
        addition(self, op1, op2);
    }
}

impl SubAssign<(&RingElement, &RingElement)> for RingElement {
    fn sub_assign(&mut self, other: (&RingElement, &RingElement)) {
        let (op1, op2) = other;
        subtraction(self, op1, op2);
    }
}

impl MulAssign<(&RingElement, &RingElement)> for RingElement {
    fn mul_assign(&mut self, other: (&RingElement, &RingElement)) {
        let (op1, op2) = other;
        incomplete_ntt_multiplication(self, op1, op2);
    }
}

// They are small so we can store them on stack.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct QuadraticExtension {
    pub coeffs: [u64; EXTENSION_DEGREE],
}

impl QuadraticExtension {
    pub const fn zero() -> Self {
        Self {
            coeffs: [0; EXTENSION_DEGREE],
        }
    }

    pub const fn one() -> Self {
        let mut coeffs = [0; EXTENSION_DEGREE];
        coeffs[0] = 1;
        Self { coeffs }
    }

    pub const fn from_base(value: u64) -> Self {
        let mut coeffs = [0; EXTENSION_DEGREE];
        coeffs[0] = value;
        Self { coeffs }
    }

    pub const fn from_pair(constant: u64, linear: u64) -> Self {
        let mut coeffs = [0; EXTENSION_DEGREE];
        coeffs[0] = constant;
        coeffs[1] = linear;
        Self { coeffs }
    }

    /// Canonical little-endian encoding of all extension coefficients in
    /// increasing basis degree.  The historical type name is retained, but
    /// the encoded length follows `EXTENSION_DEGREE`.
    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(EXTENSION_DEGREE * size_of::<u64>());
        for coefficient in self.coeffs {
            bytes.extend_from_slice(&coefficient.to_le_bytes());
        }
        bytes
    }

    /// Decode the canonical extension representation, rejecting wrong
    /// lengths and non-canonical base-field representatives.
    pub fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != EXTENSION_DEGREE * size_of::<u64>() {
            return None;
        }
        let mut result = Self::zero();
        for (index, chunk) in bytes.chunks_exact(size_of::<u64>()).enumerate() {
            let coefficient = u64::from_le_bytes(chunk.try_into().ok()?);
            if coefficient >= MOD_Q {
                return None;
            }
            result.coeffs[index] = coefficient;
        }
        Some(result)
    }

    fn multiply(&self, other: &Self) -> Self {
        let mut convolution = [0u64; 2 * EXTENSION_DEGREE - 1];
        for left_degree in 0..EXTENSION_DEGREE {
            for right_degree in 0..EXTENSION_DEGREE {
                let index = left_degree + right_degree;
                let product = unsafe {
                    multiply_mod(self.coeffs[left_degree], other.coeffs[right_degree], MOD_Q)
                };
                convolution[index] = unsafe { add_mod(convolution[index], product, MOD_Q) };
            }
        }
        for index in (EXTENSION_DEGREE..convolution.len()).rev() {
            let reduced = unsafe { multiply_mod(convolution[index], *FIELD_SHIFT_FACTOR, MOD_Q) };
            convolution[index - EXTENSION_DEGREE] =
                unsafe { add_mod(convolution[index - EXTENSION_DEGREE], reduced, MOD_Q) };
        }
        Self {
            coeffs: convolution[..EXTENSION_DEGREE].try_into().unwrap(),
        }
    }
}

impl Add for QuadraticExtension {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let coeffs = std::array::from_fn(|index| unsafe {
            add_mod(self.coeffs[index], other.coeffs[index], MOD_Q)
        });
        Self { coeffs }
    }
}

impl Mul for QuadraticExtension {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        self.multiply(&other)
    }
}

impl<'a> AddAssign<&'a QuadraticExtension> for QuadraticExtension {
    fn add_assign(&mut self, other: &'a QuadraticExtension) {
        for index in 0..EXTENSION_DEGREE {
            self.coeffs[index] = unsafe { add_mod(self.coeffs[index], other.coeffs[index], MOD_Q) };
        }
    }
}

impl<'a> AddAssign<(&'a QuadraticExtension, &'a QuadraticExtension)> for QuadraticExtension {
    fn add_assign(&mut self, other: (&'a QuadraticExtension, &'a QuadraticExtension)) {
        let (op1, op2) = other;
        for index in 0..EXTENSION_DEGREE {
            self.coeffs[index] = unsafe { add_mod(op1.coeffs[index], op2.coeffs[index], MOD_Q) };
        }
    }
}
impl<'a> SubAssign<&'a QuadraticExtension> for QuadraticExtension {
    fn sub_assign(&mut self, other: &'a QuadraticExtension) {
        for index in 0..EXTENSION_DEGREE {
            self.coeffs[index] = unsafe { sub_mod(self.coeffs[index], other.coeffs[index], MOD_Q) };
        }
    }
}

impl<'a> MulAssign<&'a QuadraticExtension> for QuadraticExtension {
    fn mul_assign(&mut self, other: &'a QuadraticExtension) {
        *self = self.multiply(other);
    }
}

impl<'a> MulAssign<(&'a QuadraticExtension, &'a QuadraticExtension)> for QuadraticExtension {
    fn mul_assign(&mut self, other: (&'a QuadraticExtension, &'a QuadraticExtension)) {
        let (lhs, rhs) = other;
        *self = *lhs * *rhs;
    }
}

impl RingElement {
    pub fn compact_size_in_bits(&self) -> usize {
        let resident = self.size_in_bits();
        let mut other = self.clone();
        match self.representation {
            Representation::IncompleteNTT => {
                other.from_incomplete_ntt_to_even_odd_coefficients();
                other.from_even_odd_coefficients_to_coefficients();
            }
            Representation::Coefficients => {
                other.from_coefficients_to_even_odd_coefficients();
                other.from_even_odd_coefficients_to_incomplete_ntt_representation();
            }
            _ => return resident,
        }
        resident.min(other.size_in_bits())
    }
}

impl SizeableProof for RingElement {
    fn size_in_bits(&self) -> usize {
        let mut size = 0;
        for v in &self.v {
            if *v > MOD_Q {
                panic!("Value exceeds modulus in size_in_bits calculation");
            }
            if *v % MOD_Q == 0 {
                continue; // zero contributes 0 bits
            }
            let size_0 = v.ilog2() as usize + 1;
            let centered = if *v > MOD_Q / 2 { MOD_Q - *v } else { *v };
            let size_1 = centered.ilog2() as usize + 2; // +1 for the sign bit
            size += size_0.min(size_1);
        }
        size
    }
}

impl SizeableProof for QuadraticExtension {
    fn size_in_bits(&self) -> usize {
        let mut size = 0;
        for v in &self.coeffs {
            let centered = if *v > MOD_Q / 2 { MOD_Q - *v } else { *v };
            if centered == 0 {
                continue; // zero contributes 0 bits
            }
            size += centered.ilog2() as usize + 1; // +1 for the sign bit
        }
        size
    }
}

#[cfg(test)]
mod tests {
    use crate::common::init_common;

    use super::*;

    #[test]
    fn test_ntt_multiplication_matches_naive() {
        init_common();
        let mut a = RingElement::random(Representation::Coefficients);
        let mut b = RingElement::random(Representation::Coefficients);
        let mut c = RingElement::new(Representation::Coefficients);

        naive_polynomial_multiplication(&mut c, &a, &b);

        a.from_coefficients_to_even_odd_coefficients();
        b.from_coefficients_to_even_odd_coefficients();
        a.from_even_odd_coefficients_to_incomplete_ntt_representation();
        b.from_even_odd_coefficients_to_incomplete_ntt_representation();

        let mut d = RingElement::new(Representation::IncompleteNTT);
        incomplete_ntt_multiplication(&mut d, &a, &b);
        d.from_incomplete_ntt_to_even_odd_coefficients();
        d.from_even_odd_coefficients_to_coefficients();

        debug_assert_eq!(c.v, d.v);
    }

    #[test]
    fn test_homogenized_field_extension_conversion_roundtrip() {
        init_common();
        let mut b = RingElement::random(Representation::Coefficients);
        b.from_coefficients_to_even_odd_coefficients();
        b.from_even_odd_coefficients_to_incomplete_ntt_representation();

        let mut b_c = b.clone();
        b_c.from_incomplete_ntt_to_homogenized_field_extensions();
        b_c.from_homogenized_field_extensions_to_incomplete_ntt();

        debug_assert_eq!(b.v, b_c.v);
    }

    #[test]
    fn test_quadratic_extension_split_combine_roundtrip() {
        init_common();
        let mut b = RingElement::random(Representation::Coefficients);
        b.from_coefficients_to_even_odd_coefficients();
        b.from_even_odd_coefficients_to_incomplete_ntt_representation();
        b.from_incomplete_ntt_to_homogenized_field_extensions();

        let ext_b: [QuadraticExtension; HALF_DEGREE] = b.split_into_quadratic_extensions();
        let mut b_reconstructed = RingElement::new(Representation::HomogenizedFieldExtensions);
        b_reconstructed.combine_from_quadratic_extensions(&ext_b);

        debug_assert_eq!(b.v, b_reconstructed.v);
    }

    #[test]
    fn test_hadamard_multiplication_in_quadratic_extensions() {
        init_common();
        let mut a = RingElement::random(Representation::Coefficients);
        let mut b = RingElement::random(Representation::Coefficients);
        let mut c = RingElement::new(Representation::Coefficients);

        naive_polynomial_multiplication(&mut c, &a, &b);

        a.from_coefficients_to_even_odd_coefficients();
        b.from_coefficients_to_even_odd_coefficients();
        a.from_even_odd_coefficients_to_incomplete_ntt_representation();
        b.from_even_odd_coefficients_to_incomplete_ntt_representation();
        a.from_incomplete_ntt_to_homogenized_field_extensions();
        b.from_incomplete_ntt_to_homogenized_field_extensions();

        let ext_a: [QuadraticExtension; HALF_DEGREE] = a.split_into_quadratic_extensions();
        let ext_b: [QuadraticExtension; HALF_DEGREE] = b.split_into_quadratic_extensions();

        let quadratic_fields_hadamard: [QuadraticExtension; HALF_DEGREE] = ext_a
            .iter()
            .zip(ext_b.iter())
            .map(|(x, y)| *x * *y)
            .collect::<Vec<QuadraticExtension>>()
            .try_into()
            .unwrap();

        let mut c_c = RingElement::new(Representation::HomogenizedFieldExtensions);
        c_c.combine_from_quadratic_extensions(&quadratic_fields_hadamard);
        c_c.from_homogenized_field_extensions_to_incomplete_ntt();
        c_c.from_incomplete_ntt_to_even_odd_coefficients();
        c_c.from_even_odd_coefficients_to_coefficients();

        debug_assert_eq!(c.v, c_c.v);
    }

    #[test]
    fn test_homogenized_multiplication_matches_naive() {
        init_common();
        let mut a = RingElement::random(Representation::Coefficients);
        let mut b = RingElement::random(Representation::Coefficients);
        let mut c = RingElement::new(Representation::Coefficients);

        naive_polynomial_multiplication(&mut c, &a, &b);

        a.from_coefficients_to_even_odd_coefficients();
        b.from_coefficients_to_even_odd_coefficients();
        a.from_even_odd_coefficients_to_incomplete_ntt_representation();
        b.from_even_odd_coefficients_to_incomplete_ntt_representation();
        a.from_incomplete_ntt_to_homogenized_field_extensions();
        b.from_incomplete_ntt_to_homogenized_field_extensions();

        let mut e = RingElement::new(Representation::HomogenizedFieldExtensions);
        incomplete_ntt_multiplication_homogenized(&mut e, &a, &b);
        e.from_homogenized_field_extensions_to_incomplete_ntt();
        e.from_incomplete_ntt_to_even_odd_coefficients();
        e.from_even_odd_coefficients_to_coefficients();

        debug_assert_eq!(c.v, e.v);
    }

    #[test]
    fn test_even_odd_coefficients_conversion_roundtrip() {
        init_common();
        let original = RingElement::random(Representation::Coefficients);
        let mut a = original.clone();

        a.from_coefficients_to_even_odd_coefficients();
        a.from_even_odd_coefficients_to_coefficients();

        debug_assert_eq!(original.v, a.v);
    }

    #[test]
    fn test_quadratic_extension_multiplication() {
        let qe1 = QuadraticExtension::from_pair(2, 3);
        let qe2 = QuadraticExtension::from_pair(4, 5);
        let result = qe1 * qe2;

        #[cfg(not(feature = "quartic-q"))]
        {
            // (2 + 3X)(4 + 5X) = 8 + 10X + 12X + 15X^2 = 8 + 22X + 15*shift
            let expected_c0 = unsafe {
                add_mod(
                    multiply_mod(2, 4, MOD_Q),
                    multiply_mod(*FIELD_SHIFT_FACTOR, multiply_mod(3, 5, MOD_Q), MOD_Q),
                    MOD_Q,
                )
            };
            let expected_c1 =
                unsafe { add_mod(multiply_mod(2, 5, MOD_Q), multiply_mod(3, 4, MOD_Q), MOD_Q) };

            debug_assert_eq!(result.coeffs[0], expected_c0);
            debug_assert_eq!(result.coeffs[1], expected_c1);
        }

        #[cfg(feature = "quartic-q")]
        assert_eq!(result.coeffs, [8, 22, 15, 0]);
    }

    #[test]
    fn test_extension_canonical_encoding_roundtrip_and_rejection() {
        let value = QuadraticExtension {
            coeffs: std::array::from_fn(|index| 17 + 31 * index as u64),
        };
        let encoded = value.to_le_bytes();
        assert_eq!(encoded.len(), EXTENSION_DEGREE * size_of::<u64>());
        assert_eq!(QuadraticExtension::from_le_bytes(&encoded), Some(value));
        assert_eq!(
            QuadraticExtension::from_le_bytes(&encoded[..encoded.len() - 1]),
            None
        );

        let mut noncanonical = encoded;
        noncanonical[..8].copy_from_slice(&MOD_Q.to_le_bytes());
        assert_eq!(QuadraticExtension::from_le_bytes(&noncanonical), None);
    }

    #[test]
    fn test_conjugation_ref() {
        init_common();
        let a = RingElement::random(Representation::IncompleteNTT);
        let mut a_conj = a.clone();
        let b = RingElement::new(Representation::IncompleteNTT);
        let mut b_conj = b.clone();

        a_conj.conjugate_in_place_ref();
        // a.conjugate_in_place_ref();

        b_conj.conjugate_in_place_ref();
        // debug_assert_eq!(a.v, a_conj.v);

        let a_plus_b = &a + &b;
        let a_times_b = &a * &b;

        let mut a_plus_b_conj = a_plus_b.clone();
        a_plus_b_conj.conjugate_in_place_ref();

        let mut a_times_b_conj = a_times_b.clone();
        a_times_b_conj.conjugate_in_place_ref();

        debug_assert_eq!(&a_conj + &b_conj, a_plus_b_conj);
        debug_assert_eq!(&a_conj * &b_conj, a_times_b_conj);
    }

    #[test]
    fn test_conjugation() {
        init_common();
        let a = RingElement::random(Representation::IncompleteNTT);
        let mut a_conj = a.clone();
        let mut a_conj_ref = a.clone();
        a_conj.conjugate_in_place();
        a_conj_ref.conjugate_in_place_ref();
        debug_assert_eq!(a_conj.v, a_conj_ref.v);
    }

    #[test]
    fn test_norm_squared_via_conjugation() {
        init_common();
        let mut vector: Vec<RingElement> = vec![
            RingElement::random_bounded(Representation::Coefficients, 10),
            RingElement::random_bounded(Representation::Coefficients, 10),
            RingElement::random_bounded(Representation::Coefficients, 10),
            RingElement::random_bounded(Representation::Coefficients, 10),
        ];

        let mut two_norm_squared = 0u64;
        for e in vector.iter_mut() {
            for coeff in e.v.iter() {
                let centered = if *coeff > MOD_Q / 2 {
                    MOD_Q - *coeff // Interpret as negative
                } else {
                    *coeff
                };
                two_norm_squared = unsafe {
                    add_mod(
                        two_norm_squared,
                        multiply_mod(centered, centered, MOD_Q),
                        MOD_Q,
                    )
                };
            }
            e.to_representation(Representation::IncompleteNTT);
        }

        let mut vector_conj = vector.clone();
        for e in vector_conj.iter_mut() {
            e.conjugate_in_place();
        }

        let mut inner_product = RingElement::new(Representation::IncompleteNTT);
        for (e1, e2) in vector.iter().zip(vector_conj.iter()) {
            let mut prod = RingElement::new(Representation::IncompleteNTT);
            prod *= (e1, e2);
            inner_product += &prod;
        }
        inner_product.from_incomplete_ntt_to_even_odd_coefficients();
        inner_product.from_even_odd_coefficients_to_coefficients();
        let ct = inner_product.v[0];

        debug_assert_eq!(ct, two_norm_squared);
    }

    #[test]
    fn test_conjugate_into_matches_in_place() {
        init_common();

        let mut a = RingElement::random(Representation::Coefficients);
        a.from_coefficients_to_even_odd_coefficients();
        a.from_even_odd_coefficients_to_incomplete_ntt_representation();

        let original = a.clone();

        let mut result = RingElement::new(Representation::IncompleteNTT);
        a.conjugate_into(&mut result);

        let mut expected = a.clone();
        expected.conjugate_in_place();

        debug_assert_eq!(result, expected);
        debug_assert_eq!(a, original);
    }

    #[test]
    fn test_constant_term_from_incomplete_ntt() {
        init_common();

        let mut a = RingElement::random(Representation::IncompleteNTT);
        let computed_constant_term = a.constant_term_from_incomplete_ntt();
        a.from_incomplete_ntt_to_even_odd_coefficients();
        a.from_even_odd_coefficients_to_coefficients();
        let expected_constant_term = a.v[0];

        debug_assert_eq!(expected_constant_term, computed_constant_term % MOD_Q);
    }

    /// Verifies that the fused incomplete-NTT multiplication kernel produces
    /// bit-identical results to the original separate-call implementation.
    #[cfg(not(feature = "quartic-q"))]
    #[test]
    fn test_fused_incomplete_ntt_mult_matches_separate() {
        init_common();

        for _ in 0..20 {
            let op1 = RingElement::random(Representation::IncompleteNTT);
            let op2 = RingElement::random(Representation::IncompleteNTT);

            // --- Reference: separate eltwise calls (the original algorithm) ---
            let mut ref_result = RingElement::new(Representation::IncompleteNTT);
            let temp = &mut [0u64; DEGREE];
            unsafe {
                // ref_even = op1_even * op2_even
                eltwise_mult_mod(
                    ref_result.v.as_mut_ptr(),
                    op1.v.as_ptr(),
                    op2.v.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                // ref_odd = op1_odd * op2_even
                eltwise_mult_mod(
                    ref_result.v.as_mut_ptr().add(HALF_DEGREE),
                    op1.v.as_ptr().add(HALF_DEGREE),
                    op2.v.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                // temp = op1_odd * op2_odd
                eltwise_mult_mod(
                    temp.as_mut_ptr(),
                    op1.v.as_ptr().add(HALF_DEGREE),
                    op2.v.as_ptr().add(HALF_DEGREE),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                // temp *= shift_factors
                eltwise_mult_mod(
                    temp.as_mut_ptr(),
                    temp.as_ptr(),
                    SHIFT_FACTORS.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                // ref_even += temp
                eltwise_add_mod(
                    ref_result.v.as_mut_ptr(),
                    ref_result.v.as_ptr(),
                    temp.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                // temp2 = op1_even * op2_odd
                eltwise_mult_mod(
                    temp.as_mut_ptr(),
                    op1.v.as_ptr(),
                    op2.v.as_ptr().add(HALF_DEGREE),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
                // ref_odd += temp2
                eltwise_add_mod(
                    ref_result.v.as_mut_ptr().add(HALF_DEGREE),
                    ref_result.v.as_ptr().add(HALF_DEGREE),
                    temp.as_ptr(),
                    HALF_DEGREE as u64,
                    MOD_Q,
                );
            }

            // --- Fused path ---
            let mut fused_result = RingElement::new(Representation::IncompleteNTT);
            unsafe {
                fused_incomplete_ntt_mult(
                    fused_result.v.as_mut_ptr(),
                    op1.v.as_ptr(),
                    op2.v.as_ptr(),
                    SHIFT_FACTORS.as_ptr(),
                    HALF_DEGREE,
                    MOD_Q,
                );
            }

            assert_eq!(
                ref_result.v, fused_result.v,
                "Fused ring mult diverged from reference"
            );
        }
    }

    #[test]
    fn test_inverse_times_self_is_one() {
        init_common();

        for _ in 0..10 {
            let a = RingElement::random(Representation::HomogenizedFieldExtensions);
            let a_inv = a.inverse();

            let mut product = RingElement::new(Representation::HomogenizedFieldExtensions);
            incomplete_ntt_multiplication_homogenized(&mut product, &a, &a_inv);

            let one = RingElement::one(Representation::HomogenizedFieldExtensions);
            assert_eq!(product.v, one.v, "a * a^{{-1}} should equal 1");
        }
    }

    #[test]
    fn test_inverse_slot_by_slot() {
        init_common();

        let a = RingElement::random(Representation::HomogenizedFieldExtensions);
        let a_inv = a.inverse();

        let slots = a.split_into_quadratic_extensions();
        let inv_slots = a_inv.split_into_quadratic_extensions();

        let one = QuadraticExtension::one();
        for i in 0..HALF_DEGREE {
            let product = slots[i] * inv_slots[i];
            assert_eq!(
                product, one,
                "Slot {i} inverse incorrect: {:?} * {:?} = {:?}",
                slots[i], inv_slots[i], product
            );
        }
    }

    #[test]
    fn test_inverse_of_one_is_one() {
        init_common();

        let one = RingElement::one(Representation::HomogenizedFieldExtensions);
        let one_inv = one.inverse();
        assert_eq!(one.v, one_inv.v, "1^{{-1}} should equal 1");
    }
}
