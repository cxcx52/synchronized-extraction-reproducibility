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
            convolution[index - EXTENSION_DEGREE] =
                add_mod(convolution[index - EXTENSION_DEGREE], reduced, QUARTIC_Q);
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
    raw_to_homogeneous_index: [usize; RING_DEGREE],
    raw_to_homogeneous_scale: [u64; RING_DEGREE],
    homogeneous_to_raw_scale: [u64; RING_DEGREE],
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

        let raw_to_homogeneous_index = std::array::from_fn(|raw_index| {
            let raw_degree = raw_index / SLOT_COUNT;
            let slot = raw_index % SLOT_COUNT;
            let power = exponents[slot] * raw_degree;
            (power % EXTENSION_DEGREE) * SLOT_COUNT + slot
        });
        let raw_to_homogeneous_scale = std::array::from_fn(|raw_index| {
            let raw_degree = raw_index / SLOT_COUNT;
            let slot = raw_index % SLOT_COUNT;
            let power = exponents[slot] * raw_degree;
            power_mod(beta, (power / EXTENSION_DEGREE) as u64, QUARTIC_Q)
        });
        let homogeneous_to_raw_scale =
            raw_to_homogeneous_scale.map(|scale| inv_mod(scale, QUARTIC_Q));

        Self {
            raw_shift_factors: shifts,
            beta,
            slot_exponents: exponents,
            raw_to_homogeneous_index,
            raw_to_homogeneous_scale,
            homogeneous_to_raw_scale,
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
                result.coeffs[homogeneous_degree] =
                    multiply_mod(blocks[raw_degree][slot], scale, QUARTIC_Q);
            }
            result
        })
    }

    /// Convert the block-major output of four independent size-32 NTTs into
    /// the common presentation `F_q[T]/(T^4-beta)` used by every CRT slot.
    /// Both layouts store coefficient `degree` of slot `slot` at
    /// `degree * SLOT_COUNT + slot`.
    pub fn raw_ntt_layout_to_homogeneous(&self, raw: &[u64; RING_DEGREE]) -> [u64; RING_DEGREE] {
        let mut homogeneous = [0u64; RING_DEGREE];
        for raw_index in 0..RING_DEGREE {
            homogeneous[self.raw_to_homogeneous_index[raw_index]] = multiply_mod(
                raw[raw_index],
                self.raw_to_homogeneous_scale[raw_index],
                QUARTIC_Q,
            );
        }
        homogeneous
    }

    /// Inverse of [`Self::raw_ntt_layout_to_homogeneous`].
    pub fn homogeneous_to_raw_ntt_layout(
        &self,
        homogeneous: &[u64; RING_DEGREE],
    ) -> [u64; RING_DEGREE] {
        let mut raw = [0u64; RING_DEGREE];
        for raw_index in 0..RING_DEGREE {
            raw[raw_index] = multiply_mod(
                homogeneous[self.raw_to_homogeneous_index[raw_index]],
                self.homogeneous_to_raw_scale[raw_index],
                QUARTIC_Q,
            );
        }
        raw
    }

    pub fn multiply_homogeneous_layout(
        &self,
        lhs: &[u64; RING_DEGREE],
        rhs: &[u64; RING_DEGREE],
    ) -> [u64; RING_DEGREE] {
        let mut result = [0u64; RING_DEGREE];
        for slot in 0..SLOT_COUNT {
            let lhs_slot = QuarticSlot {
                coeffs: std::array::from_fn(|degree| lhs[degree * SLOT_COUNT + slot]),
            };
            let rhs_slot = QuarticSlot {
                coeffs: std::array::from_fn(|degree| rhs[degree * SLOT_COUNT + slot]),
            };
            let product = lhs_slot.mul(&rhs_slot, self.beta);
            for degree in 0..EXTENSION_DEGREE {
                result[degree * SLOT_COUNT + slot] = product.coeffs[degree];
            }
        }
        result
    }

    pub fn multiply_raw_ntt_layout(
        &self,
        lhs: &[u64; RING_DEGREE],
        rhs: &[u64; RING_DEGREE],
    ) -> [u64; RING_DEGREE] {
        let lhs_homogeneous = self.raw_ntt_layout_to_homogeneous(lhs);
        let rhs_homogeneous = self.raw_ntt_layout_to_homogeneous(rhs);
        let product = self.multiply_homogeneous_layout(&lhs_homogeneous, &rhs_homogeneous);
        self.homogeneous_to_raw_ntt_layout(&product)
    }

    pub fn inverse_homogeneous_layout(
        &self,
        value: &[u64; RING_DEGREE],
    ) -> Option<[u64; RING_DEGREE]> {
        let mut result = [0u64; RING_DEGREE];
        for slot in 0..SLOT_COUNT {
            let value_slot = QuarticSlot {
                coeffs: std::array::from_fn(|degree| value[degree * SLOT_COUNT + slot]),
            };
            if value_slot == QuarticSlot::zero() {
                return None;
            }
            let inverse = value_slot.inverse(self.beta);
            for degree in 0..EXTENSION_DEGREE {
                result[degree * SLOT_COUNT + slot] = inverse.coeffs[degree];
            }
        }
        Some(result)
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
        let product_slots =
            std::array::from_fn(|slot| lhs_slots[slot].mul(&rhs_slots[slot], self.beta));
        self.homogeneous_slots_to_coefficients(&product_slots)
    }

    pub fn inverse(&self, value: &[u64; RING_DEGREE]) -> Option<[u64; RING_DEGREE]> {
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
    #[cfg(feature = "quartic-q")]
    use crate::common::{hash::HashWrapper, ring_arithmetic::QuadraticExtension};
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

            let mut raw = [0u64; RING_DEGREE];
            for residue in 0..EXTENSION_DEGREE {
                for digit in 0..SLOT_COUNT {
                    raw[residue * SLOT_COUNT + digit] = input[EXTENSION_DEGREE * digit + residue];
                }
                ntt_forward_in_place(
                    &mut raw[residue * SLOT_COUNT..(residue + 1) * SLOT_COUNT],
                    SLOT_COUNT,
                    QUARTIC_Q,
                );
            }
            let homogeneous = transform.raw_ntt_layout_to_homogeneous(&raw);
            assert_eq!(transform.homogeneous_to_raw_ntt_layout(&homogeneous), raw);
            for slot in 0..SLOT_COUNT {
                for degree in 0..EXTENSION_DEGREE {
                    assert_eq!(
                        homogeneous[degree * SLOT_COUNT + slot],
                        slots[slot].coeffs[degree]
                    );
                }
            }
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
            let inverse = transform
                .inverse(&value)
                .expect("random ring element is a unit");
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
            assert_eq!(
                *shift,
                power_mod(transform.beta, *exponent as u64, QUARTIC_Q)
            );
            assert_eq!(power_mod(*shift, 32, QUARTIC_Q), QUARTIC_Q - 1);
        }
    }

    #[cfg(feature = "quartic-q")]
    #[test]
    fn quartic_extension_serialization_and_transcript_bind_all_limbs_in_order() {
        let first = QuadraticExtension {
            coeffs: [11, 22, 33, 44],
        };
        let second = QuadraticExtension {
            coeffs: [55, 66, 77, 88],
        };
        assert_eq!(
            QuadraticExtension::from_le_bytes(&first.to_le_bytes()),
            Some(first)
        );

        let mut typed = HashWrapper::new();
        typed.update_with_quadratic_extension_element(&first);
        let mut canonical_bytes = HashWrapper::new();
        canonical_bytes.update_with_bytes(&first.to_le_bytes());
        assert_eq!(typed.sample_bytes(32), canonical_bytes.sample_bytes(32));

        let mut ordered = HashWrapper::new();
        ordered.update_with_quadratic_extension_element(&first);
        ordered.update_with_quadratic_extension_element(&second);
        let mut swapped = HashWrapper::new();
        swapped.update_with_quadratic_extension_element(&second);
        swapped.update_with_quadratic_extension_element(&first);
        assert_ne!(ordered.sample_bytes(32), swapped.sample_bytes(32));

        for degree in 0..EXTENSION_DEGREE {
            let mut changed = first;
            changed.coeffs[degree] += 1;
            let mut original_transcript = HashWrapper::new();
            original_transcript.update_with_quadratic_extension_element(&first);
            let mut changed_transcript = HashWrapper::new();
            changed_transcript.update_with_quadratic_extension_element(&changed);
            assert_ne!(
                original_transcript.sample_bytes(32),
                changed_transcript.sample_bytes(32),
                "extension coefficient {degree} was not transcript-bound"
            );
        }
    }
}
