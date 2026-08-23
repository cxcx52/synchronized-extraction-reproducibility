use crate::common::{config::MOD_Q, ring_arithmetic::RingElement};

pub fn inf_norm(vec: &[RingElement]) -> u64 {
    vec.iter()
        .map(|el| {
            let mut el_cloned = el.clone();
            el_cloned.from_incomplete_ntt_to_even_odd_coefficients();
            el_cloned
                .v
                .map(|x| x)
                .iter()
                .map(|&x| {
                    if x > MOD_Q / 2 {
                        MOD_Q - x as u64
                    } else {
                        x as u64
                    }
                })
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0)
}

pub fn l2_norm(vec: &[RingElement]) -> f64 {
    let mut sum = 0u128;
    for el in vec {
        let mut el_cloned = el.clone();
        el_cloned.from_incomplete_ntt_to_even_odd_coefficients();
        for &x in el_cloned.v.map(|x| x).iter() {
            let centered = if x < MOD_Q / 2 { x } else { MOD_Q - x };
            sum += centered as u128 * centered as u128;
        }
    }
    (sum as f64).sqrt() as f64
}

pub fn l2_norm_coeffs(vec: &[RingElement]) -> f64 {
    let mut sum = 0u128;
    for el in vec {
        for &x in el.v.map(|x| x).iter() {
            let centered = if x < MOD_Q / 2 { x } else { MOD_Q - x };
            sum += centered as u128 * centered as u128;
        }
    }
    (sum as f64).sqrt() as f64
}

#[cfg(test)]
mod tests {
    use super::l2_norm_coeffs;
    use crate::common::{
        config::{DEGREE, MOD_Q},
        ring_arithmetic::{Representation, RingElement},
    };

    #[test]
    fn l2_accumulator_handles_full_width_centered_coefficients() {
        let coefficient = MOD_Q / 2;
        let values = [RingElement::all(
            coefficient,
            Representation::EvenOddCoefficients,
        )];
        let expected = coefficient as f64 * (DEGREE as f64).sqrt();
        let actual = l2_norm_coeffs(&values);
        assert!((actual - expected).abs() <= expected * 1e-12);
    }
}

pub fn assert_norm_bounded(label: &str, value: f64, bound: f64) {
    tracing::debug!("L2 norm of {label}: {value} (bound {bound})");
    #[cfg(test)]
    if std::env::var_os("ROKOKO_CALIBRATE_NORMS").is_some() {
        eprintln!("norm calibration: label={label:?} value={value} old_bound={bound}");
        return;
    }
    assert!(
        value <= bound,
        "L2 norm of {label} = {value} exceeds the registered bound {bound}"
    );
}
