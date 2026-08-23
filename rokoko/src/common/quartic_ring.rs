//! Isolated arithmetic backend for the 50-bit quartic candidate modulus.
//!
//! This module intentionally does not replace the repository's default
//! quadratic backend.  It proves the arithmetic migration path independently:
//! four coefficient streams, four size-32 negacyclic NTTs, explicit slot
//! isomorphisms into one common quartic field, and componentwise multiplication.

use incomplete_rexl::{
    add_mod, inv_mod, multiply_mod, ntt_forward_in_place, ntt_inverse_in_place, power_mod,
};
use num::bigint::BigUint;
use num::{One, Zero};

pub const QUARTIC_Q: u64 = 926_510_094_425_921;
pub const RING_DEGREE: usize = 128;
pub const EXTENSION_DEGREE: usize = 4;
pub const SLOT_COUNT: usize = RING_DEGREE / EXTENSION_DEGREE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuarticSlot {
    pub coeffs: [u64; EXTENSION_DEGREE],
}

impl QuarticSlot {
    pub const fn zero() -> Self {
        Self {
            coeffs: [0; EXTENSION_DEGREE],
        }
    }

    pub const fn one() -> Self {
        Self {
            coeffs: [1, 0, 0, 0],
        }
    }

    pub fn mul(&self, rhs: &Self, beta: u64) -> Self {
        let mut convolution = [0u64; 2 * EXTENSION_DEGREE - 1];
        for left_degree in 0..EXTENSION_DEGREE {
            for right_degree in 0..EXTENSION_DEGREE {
                let index = left_degree + right_degree;
                let product = multiply_mod(
                    self.coeffs[left_degree],
                    rhs.coeffs[right_degree],
                    QUARTIC_Q,
                );
                convolution[index] = add_mod(convolution[index], product, QUARTIC_Q);
            }
        }
        for index in (EXTENSION_DEGREE..convolution.len()).rev() {
            let reduced = multiply_mod(convolution[index], beta, QUARTIC_Q);
            convolution[index - EXTENSION_DEGREE] = add_mod(
                convolution[index - EXTENSION_DEGREE],
                reduced,
                QUARTIC_Q,
            );
        }
        Self {
            coeffs: convolution[..EXTENSION_DEGREE].try_into().unwrap(),
        }
    }

    pub fn pow_big(&self, exponent: &BigUint, beta: u64) -> Self {
        let mut result = Self::one();
        let mut base = *self;
        let mut remaining = exponent.clone();
        let one = BigUint::one();
        while !remaining.is_zero() {
            if (&remaining & &one) == one {
                result = result.mul(&base, beta);
            }
            remaining >>= 1usize;
            if !remaining.is_zero() {
                base = base.mul(&base, beta);
            }
        }
        result
    }

    pub fn inverse(&self, beta: u64) -> Self {
        assert_ne!(*self, Self::zero(), "zero quartic slot is not invertible");
        let exponent = BigUint::from(QUARTIC_Q).pow(EXTENSION_DEGREE as u32) - 2u8;
        self.pow_big(&exponent, beta)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuarticTransform {
    /// The raw slot constants `zeta_i` satisfying `X^4=zeta_i`.
    pub raw_shift_factors: [u64; SLOT_COUNT],
    /// Common field presentation `F_q[T]/(T^4-beta)`.
    pub beta: u64,
    /// Odd exponents such that `zeta_i=beta^slot_exponents[i]`.
    pub slot_exponents: [usize; SLOT_COUNT],
}

impl QuarticTransform {
    pub fn new() -> Self {
        let mut shifts = [0u64; SLOT_COUNT];
        shifts[1] = 1;
        ntt_forward_in_place(&mut shifts, SLOT_COUNT, QUARTIC_Q);
        let beta = shifts[0];
        assert_eq!(power_mod(beta, 32, QUARTIC_Q), QUARTIC_Q - 1);
        assert_eq!(power_mod(beta, 64, QUARTIC_Q), 1);

        let mut exponents = [0usize; SLOT_COUNT];
        for (slot, shift) in shifts.iter().copied().enumerate() {
            exponents[slot] = (1..64)
                .step_by(2)
                .find(|exponent| power_mod(beta, *exponent as u64, QUARTIC_Q) == shift)
                .expect("each size-32 negacyclic NTT slot must be an odd beta power");
        }
        let mut sorted = exponents;
        sorted.sort_unstable();
        assert_eq!(sorted, std::array::from_fn(|index| 2 * index + 1));

        Self {
            raw_shift_factors: shifts,
            beta,
            slot_exponents: exponents,
        }
    }

    pub fn coefficients_to_homogeneous_slots(
        &self,
        coefficients: &[u64; RING_DEGREE],
    ) -> [QuarticSlot; SLOT_COUNT] {
        let mut blocks = [[0u64; SLOT_COUNT]; EXTENSION_DEGREE];
        for residue in 0..EXTENSION_DEGREE {
            for digit in 0..SLOT_COUNT {
                blocks[residue][digit] = coefficients[EXTENSION_DEGREE * digit + residue];
            }
            ntt_forward_in_place(&mut blocks[residue], SLOT_COUNT, QUARTIC_Q);
        }

        std::array::from_fn(|slot| {
            let exponent = self.slot_exponents[slot];
            let mut result = QuarticSlot::zero();
            for raw_degree in 0..EXTENSION_DEGREE {
                let power = exponent * raw_degree;
                let homogeneous_degree = power % EXTENSION_DEGREE;
                let scale = power_mod(self.beta, (power / EXTENSION_DEGREE) as u64, QUARTIC_Q);
                result.coeffs[homogeneous_degree] = multiply_mod(
                    blocks[raw_degree][slot],
                    scale,
                    QUARTIC_Q,
                );
            }
            result
        })
    }

    pub fn homogeneous_slots_to_coefficients(
        &self,
        slots: &[QuarticSlot; SLOT_COUNT],
    ) -> [u64; RING_DEGREE] {
        let mut blocks = [[0u64; SLOT_COUNT]; EXTENSION_DEGREE];
        for slot in 0..SLOT_COUNT {
            let exponent = self.slot_exponents[slot];
            for raw_degree in 0..EXTENSION_DEGREE {
                let power = exponent * raw_degree;
                let homogeneous_degree = power % EXTENSION_DEGREE;
                let scale = power_mod(self.beta, (power / EXTENSION_DEGREE) as u64, QUARTIC_Q);
                blocks[raw_degree][slot] = multiply_mod(
                    slots[slot].coeffs[homogeneous_degree],
                    inv_mod(scale, QUARTIC_Q),
                    QUARTIC_Q,
                );
            }
        }

        for block in &mut blocks {
            ntt_inverse_in_place(block, SLOT_COUNT, QUARTIC_Q);
        }
        let mut coefficients = [0u64; RING_DEGREE];
        for residue in 0..EXTENSION_DEGREE {
            for digit in 0..SLOT_COUNT {
                coefficients[EXTENSION_DEGREE * digit + residue] = blocks[residue][digit];
            }
        }
        coefficients
    }

    pub fn multiply(
        &self,
        lhs: &[u64; RING_DEGREE],
        rhs: &[u64; RING_DEGREE],
    ) -> [u64; RING_DEGREE] {
        let lhs_slots = self.coefficients_to_homogeneous_slots(lhs);
        let rhs_slots = self.coefficients_to_homogeneous_slots(rhs);
        let product_slots = std::array::from_fn(|slot| {
            lhs_slots[slot].mul(&rhs_slots[slot], self.beta)
        });
        self.homogeneous_slots_to_coefficients(&product_slots)
    }

    pub fn inverse(
        &self,
        value: &[u64; RING_DEGREE],
    ) -> Option<[u64; RING_DEGREE]> {
        let slots = self.coefficients_to_homogeneous_slots(value);
        if slots.iter().any(|slot| *slot == QuarticSlot::zero()) {
            return None;
        }
        let inverses = std::array::from_fn(|slot| slots[slot].inverse(self.beta));
        Some(self.homogeneous_slots_to_coefficients(&inverses))
    }
}

pub fn naive_negacyclic_multiply(
    lhs: &[u64; RING_DEGREE],
    rhs: &[u64; RING_DEGREE],
) -> [u64; RING_DEGREE] {
    let mut result = [0u64; RING_DEGREE];
    for (left_index, left) in lhs.iter().copied().enumerate() {
        for (right_index, right) in rhs.iter().copied().enumerate() {
            let product = multiply_mod(left, right, QUARTIC_Q);
            let degree = left_index + right_index;
            let index = degree % RING_DEGREE;
            let signed = if degree < RING_DEGREE {
                product
            } else if product == 0 {
                0
            } else {
                QUARTIC_Q - product
            };
            result[index] = add_mod(result[index], signed, QUARTIC_Q);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, SeedableRng};

    fn random_coefficients(rng: &mut rand::rngs::StdRng) -> [u64; RING_DEGREE] {
        std::array::from_fn(|_| rng.random_range(0..QUARTIC_Q))
    }

    #[test]
    fn quartic_transform_round_trip() {
        let transform = QuarticTransform::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(0x5141_5254_4943);
        for _ in 0..256 {
            let input = random_coefficients(&mut rng);
            let slots = transform.coefficients_to_homogeneous_slots(&input);
            assert_eq!(transform.homogeneous_slots_to_coefficients(&slots), input);
        }
    }

    #[test]
    fn quartic_multiplication_matches_naive_negacyclic_reference() {
        let transform = QuarticTransform::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(0x4d55_4c54_4950_4c59);
        for _ in 0..256 {
            let lhs = random_coefficients(&mut rng);
            let rhs = random_coefficients(&mut rng);
            assert_eq!(
                transform.multiply(&lhs, &rhs),
                naive_negacyclic_multiply(&lhs, &rhs)
            );
        }
    }

    #[test]
    fn quartic_slot_and_ring_inverses_are_correct() {
        let transform = QuarticTransform::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(0x494e_5645_5253_4553);
        let one = {
            let mut value = [0u64; RING_DEGREE];
            value[0] = 1;
            value
        };
        for _ in 0..16 {
            let value = random_coefficients(&mut rng);
            let inverse = transform.inverse(&value).expect("random ring element is a unit");
            assert_eq!(transform.multiply(&value, &inverse), one);
        }
    }

    #[test]
    fn quartic_slot_constants_match_certified_factorization() {
        let transform = QuarticTransform::new();
        for (shift, exponent) in transform
            .raw_shift_factors
            .iter()
            .zip(transform.slot_exponents.iter())
        {
            assert_eq!(*shift, power_mod(transform.beta, *exponent as u64, QUARTIC_Q));
            assert_eq!(power_mod(*shift, 32, QUARTIC_Q), QUARTIC_Q - 1);
        }
    }
}
