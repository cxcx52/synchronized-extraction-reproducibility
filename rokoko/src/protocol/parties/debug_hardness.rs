//! Per-round norm tracking and RSIS hardness estimation (`debug-hardness`).
//!
//! Security certification is deliberately based on verifier-enforced norm
//! bounds from the parameter set.  Honest-run norms are printed only as
//! completeness diagnostics and never enter a malicious-prover SIS estimate.
//! The extracted witness norm is the worse of the rewinding bound and the JL
//! projection bound, as in the paper's extraction analysis.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    common::{
        config::MOD_Q,
        decomposition::compose_from_decomposed,
        estimator::{estimate_rsis_security, EstimatorResult, RSISParameters},
        norms,
        ring_arithmetic::RingElement,
        short_challenge::SPECTRAL_OP_NORM_SAFE_BOUND,
    },
    protocol::{
        commitment::{RecursionConfig, RecursiveCommitmentWithAux},
        config::{IntermediateConfig, Projection, SimpleConfig, SumcheckConfig},
    },
};

/// Paper: alpha_rp = sqrt(30), the lower JL bound (Lemma "JL", kappa = 2^-128).
const JL_ALPHA_RP: f64 = 5.477225575051661;

/// Rewinding slack: factor 4 for the difference quotient in extraction,
/// factor 2 for ISIS-to-SIS.
const EXTRACTION_SLACK: f64 = 8.0;
const TARGET_SECURITY_BITS: f64 = 128.0;

static ROUND_ID: AtomicUsize = AtomicUsize::new(0);
static DEBUG_HARDNESS_FROM_ROUND: usize = 0;

fn exhaustive_audit_enabled() -> bool {
    #[cfg(test)]
    {
        return std::env::var_os("ROKOKO_AUDIT_HARDNESS").is_some();
    }
    #[cfg(not(test))]
    false
}

fn minimum_rank_for_bound(
    witness_height: usize,
    length_bound: f64,
    target_bits: f64,
    max_rank: usize,
) -> Option<(usize, f64)> {
    if !length_bound.is_finite() || length_bound.ceil() >= ((MOD_Q - 1) / 2) as f64 {
        return None;
    }

    (1..=max_rank).find_map(|rank| {
        let result = estimate_rsis_security(&RSISParameters {
            m: witness_height as u64,
            n: rank as u64,
            length_bound: length_bound.ceil() as u64,
        })
        .ok()?;
        (result.secpar >= target_bits).then_some((rank, result.secpar))
    })
}

fn enforce_estimated_security(
    scope: &str,
    witness_height: usize,
    rank: usize,
    length_bound: f64,
    result: &Result<EstimatorResult, std::io::Error>,
) {
    if exhaustive_audit_enabled() {
        eprintln!(
            "HARDNESS_AUDIT security scope={scope:?} m={witness_height} rank={rank} \
             length_bound={length_bound} target_bits={TARGET_SECURITY_BITS} result={result:?} \
             minimum_rank_ge_128={:?}",
            minimum_rank_for_bound(witness_height, length_bound, TARGET_SECURITY_BITS, 64)
        );
        return;
    }

    match result {
        Ok(estimate) => assert!(
            estimate.secpar >= TARGET_SECURITY_BITS,
            "SIS estimate below target for {scope}: rank={rank}, \
             length_bound={length_bound}, estimated_bits={}, target_bits={TARGET_SECURITY_BITS}",
            estimate.secpar
        ),
        Err(error) => panic!(
            "SIS estimate invalid for {scope}: rank={rank}, \
             length_bound={length_bound}, target_bits={TARGET_SECURITY_BITS}, error={error}"
        ),
    }
}

/// Euclidean operator norm of balanced radix recomposition
///
///     (d_0, ..., d_{r-1}) -> sum_i 2^{base_log i} d_i.
///
/// The decomposition parameter stores the *logarithm* of the radix, not the
/// radix itself.  Consequently the norm multiplier is the Euclidean norm of
/// `(1, 2^base_log, ..., 2^{base_log (chunks-1)})`; using
/// `base_log^(chunks-1)` would not be a valid recomposition bound.
fn recomposition_l2_operator_norm(base_log: usize, chunks: usize) -> f64 {
    assert!(chunks > 0, "decomposition must contain at least one chunk");

    let exponent = i32::try_from(base_log).expect("decomposition base_log must fit in i32");
    let radix = 2f64.powi(exponent);
    let mut squared_norm = 0.0;
    let mut weight = 1.0;
    for _ in 0..chunks {
        squared_norm += weight * weight;
        weight *= radix;
    }

    let operator_norm = squared_norm.sqrt();
    assert!(
        operator_norm.is_finite(),
        "decomposition recomposition norm overflowed f64"
    );
    operator_norm
}

fn certified_recomposition_bound(
    verifier_enforced_decomposed_bound: f64,
    base_log: usize,
    chunks: usize,
) -> f64 {
    assert!(
        verifier_enforced_decomposed_bound.is_finite(),
        "security certification requires a finite verifier-enforced norm bound"
    );
    verifier_enforced_decomposed_bound * recomposition_l2_operator_norm(base_log, chunks)
}

fn check_recursive_commitment(
    rc: &RecursiveCommitmentWithAux,
    config: &RecursionConfig,
    name: &str,
    extracted_norm: f64,
    extracted_norm_most_inner: f64,
    depth: usize,
) {
    let ell_inf_norm = norms::inf_norm(&rc.committed_data);
    let ell_2_norm = norms::l2_norm(&rc.committed_data);

    let current_extracted_norm = match config.next {
        Some(_) => extracted_norm,
        None => extracted_norm_most_inner,
    };

    let hardness = estimate_rsis_security(&RSISParameters {
        m: rc.committed_data.len() as u64,
        n: config.rank as u64,
        length_bound: current_extracted_norm.ceil() as u64,
    });
    let indent = "  ".repeat(depth);
    tracing::debug!(
        "{}Recursive Commitment '{}' norms: L_2 = {}, bit_len = {}, MOD_Q = {} => estimated security for extraction: {:?}",
        indent,
        name,
        ell_2_norm,
        ell_inf_norm.ilog2(),
        MOD_Q,
        hardness,
    );
    enforce_estimated_security(
        &format!("recursive commitment {name} depth {depth}"),
        rc.committed_data.len(),
        config.rank,
        current_extracted_norm,
        &hardness,
    );

    if let (Some(next_rc), Some(next_config)) = (&rc.next, &config.next) {
        check_recursive_commitment(
            next_rc,
            next_config,
            name,
            extracted_norm,
            extracted_norm_most_inner,
            depth + 1,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn check_sumcheck_round(
    config: &SumcheckConfig,
    next_round_data: &[RingElement],
    rc_commitment: &RecursiveCommitmentWithAux,
    rc_opening: &RecursiveCommitmentWithAux,
    rc_coarse_projection: Option<&RecursiveCommitmentWithAux>,
    rc_fine_projection: Option<(&RecursiveCommitmentWithAux, &RecursiveCommitmentWithAux)>,
    next_level_width: usize,
) {
    if ROUND_ID.fetch_add(1, Ordering::Relaxed) < DEBUG_HARDNESS_FROM_ROUND {
        return;
    }

    tracing::debug!("=== Debug Hardness Check ===");

    let recommited_ell_inf_norm = norms::inf_norm(next_round_data);
    let recommited_ell_2_norm = norms::l2_norm(next_round_data);

    let most_inner_commitment_data_ell_2 = {
        let commitment_data = &rc_commitment
            .most_inner_commitment_with_aux()
            .committed_data;
        let norm_commitment_data_ell_2_sq = norms::l2_norm(commitment_data).powf(2.0) as u64;

        let opening_data = &rc_opening.most_inner_commitment_with_aux().committed_data;
        let norm_opening_data_ell_2_sq = norms::l2_norm(opening_data).powf(2.0) as u64;

        let norm_projection_data_ell_2_sq = match (rc_coarse_projection, rc_fine_projection) {
            (Some(rc_proj), _) => {
                let proj_data = &rc_proj.most_inner_commitment_with_aux().committed_data;
                norms::l2_norm(proj_data).powf(2.0) as u64
            }
            (_, Some((rc_ct, rc_batched))) => {
                let proj_ct_data = &rc_ct.most_inner_commitment_with_aux().committed_data;
                let proj_batched_data = &rc_batched.most_inner_commitment_with_aux().committed_data;
                norms::l2_norm(proj_ct_data).powf(2.0) as u64
                    + norms::l2_norm(proj_batched_data).powf(2.0) as u64
            }
            _ => 0,
        };
        ((norm_commitment_data_ell_2_sq
            + norm_opening_data_ell_2_sq
            + norm_projection_data_ell_2_sq) as f64)
            .sqrt()
    };
    tracing::debug!(
        "Most inner commitment data L_2 norm: {}",
        most_inner_commitment_data_ell_2
    );

    let folded_prefix = &config.folded_witness_prefix;
    let folded_region_capacity =
        1usize << (next_round_data.len().ilog2() as usize - folded_prefix.length);
    let folded_region_start =
        folded_prefix.prefix << (next_round_data.len().ilog2() as usize - folded_prefix.length);
    let folded_region_length = config.witness_height * config.witness_decomposition_chunks;
    assert!(folded_region_length <= folded_region_capacity);
    let decomposed_folded_witness_l2 = norms::l2_norm(
        &next_round_data[folded_region_start..folded_region_start + folded_region_length],
    );
    check_recursive_commitment(
        rc_commitment,
        &config.commitment_recursion,
        "Commitment",
        config.norm_bound,
        config.most_inner_norm_bound,
        0,
    );

    check_recursive_commitment(
        rc_opening,
        &config.opening_recursion,
        "Opening",
        config.norm_bound,
        config.most_inner_norm_bound,
        0,
    );

    if let (Some(rc_projection), Projection::Coarse(projection_config)) =
        (rc_coarse_projection, &config.projection_recursion)
    {
        check_recursive_commitment(
            rc_projection,
            projection_config,
            "Projection Image",
            config.norm_bound,
            config.most_inner_norm_bound,
            0,
        );
    }

    if let (Some((rc_ct, rc_batched)), Projection::Fine(projection_config)) =
        (rc_fine_projection, &config.projection_recursion)
    {
        check_recursive_commitment(
            rc_ct,
            &projection_config.recursion_constant_term,
            "Fine Projection Constant Term",
            config.norm_bound,
            config.most_inner_norm_bound,
            0,
        );
        check_recursive_commitment(
            rc_batched,
            &projection_config.recursion_batched_projection,
            "Fine Projection Batched",
            config.norm_bound,
            config.most_inner_norm_bound,
            0,
        );
    }
    tracing::debug!(
        "Next round data norms: L_inf = {}, bit_len = {}, L_2 = {}, MOD_Q = {}",
        recommited_ell_inf_norm,
        recommited_ell_inf_norm.ilog2(),
        recommited_ell_2_norm,
        MOD_Q
    );

    let observed_recomposed_folded_witness_l2 = norms::l2_norm(&compose_from_decomposed(
        &next_round_data[folded_region_start..folded_region_start + folded_region_length].to_vec(),
        config.witness_decomposition_base_log as u64,
        config.witness_decomposition_chunks,
    ));
    let recomposed_witness_bound = config.folded_recomposed_norm_bound;

    let extracted_witness_bound =
        recomposed_witness_bound * SPECTRAL_OP_NORM_SAFE_BOUND * EXTRACTION_SLACK;

    let (
        recomposed_projection_bound,
        observed_recomposed_projection_l2,
        decomposed_projection_l2,
        projection_base_log,
        projection_chunks,
    ) = match &config.projection_recursion {
        Projection::Coarse(proj_config) => {
            let projection_commitment =
                rc_coarse_projection.expect("coarse projection must have a recursive commitment");
            let decomposed_projection_l2 = norms::l2_norm(&projection_commitment.committed_data);
            let observed_recomposed_projection_l2 = norms::l2_norm(&compose_from_decomposed(
                &projection_commitment.committed_data,
                proj_config.decomposition_base_log as u64,
                proj_config.decomposition_chunks,
            ));
            (
                config.projection_recomposed_norm_bound,
                observed_recomposed_projection_l2,
                decomposed_projection_l2,
                proj_config.decomposition_base_log,
                proj_config.decomposition_chunks,
            )
        }
        Projection::Fine(proj_config) => {
            let constant_term = &proj_config.recursion_constant_term;
            let projection_commitment = &rc_fine_projection
                .expect("fine projection must have recursive commitments")
                .0;
            let decomposed_projection_l2 = norms::l2_norm(&projection_commitment.committed_data);
            let observed_recomposed_projection_l2 = norms::l2_norm(&compose_from_decomposed(
                &projection_commitment.committed_data,
                constant_term.decomposition_base_log as u64,
                constant_term.decomposition_chunks,
            ));
            (
                config.projection_recomposed_norm_bound,
                observed_recomposed_projection_l2,
                decomposed_projection_l2,
                constant_term.decomposition_base_log,
                constant_term.decomposition_chunks,
            )
        }
        Projection::Skip => (0.0, 0.0, 0.0, 0, 0), // not used
    };

    let argued_witness_bound = recomposed_projection_bound / JL_ALPHA_RP;

    let worse_bound = if extracted_witness_bound > argued_witness_bound {
        tracing::debug!(
            "Using extracted witness bound {} for security estimation.",
            extracted_witness_bound
        );
        extracted_witness_bound
    } else {
        tracing::debug!(
            "Using projection-argued witness bound {} for security estimation.",
            argued_witness_bound
        );
        argued_witness_bound
    };

    match &config.projection_recursion {
        Projection::Skip => {
            // no projection: inner-product norm extraction is not available anyway
        }
        _ => {
            let uniqueness_lhs =
                next_level_width as f64 * argued_witness_bound * argued_witness_bound;
            let uniqueness_rhs = MOD_Q as f64 / 2f64;
            let uniqueness_holds = uniqueness_lhs < uniqueness_rhs;
            if exhaustive_audit_enabled() {
                eprintln!(
                    "HARDNESS_AUDIT sumcheck uniqueness_holds={uniqueness_holds} \
                     provenance=verifier_enforced width={next_level_width} next_data_l2={recommited_ell_2_norm} \
                     decomposed_folded_witness_l2={decomposed_folded_witness_l2} \
                     observed_recomposed_folded_witness_l2={observed_recomposed_folded_witness_l2} \
                     certified_combined_bound={} \
                     certified_folded_recomposed_bound={} \
                     certified_projection_recomposed_bound={} \
                     decomposed_projection_l2={decomposed_projection_l2} \
                     observed_recomposed_projection_l2={observed_recomposed_projection_l2} \
                     projection_base_log={projection_base_log} projection_chunks={projection_chunks} \
                     recomposed_projection_bound={recomposed_projection_bound} \
                     argued_witness_bound={argued_witness_bound} \
                     width_times_bound_squared={uniqueness_lhs} q_over_two={uniqueness_rhs}",
                    config.norm_bound,
                    config.folded_recomposed_norm_bound,
                    config.projection_recomposed_norm_bound
                );
            } else {
                assert!(
                    uniqueness_holds,
                    "Witness bound too large for inner-product norm extraction: \
                     width={next_level_width}, next_data_l2={recommited_ell_2_norm}, \
                     decomposed_projection_l2={decomposed_projection_l2}, \
                     observed_recomposed_projection_l2={observed_recomposed_projection_l2}, \
                     projection_base_log={projection_base_log}, projection_chunks={projection_chunks}, \
                     recomposed_projection_bound={recomposed_projection_bound}, \
                     argued_witness_bound={argued_witness_bound}, \
                     width_times_bound_squared={uniqueness_lhs}, q_over_two={uniqueness_rhs}"
                );
            }
        }
    }

    let basic_commitment_security = estimate_rsis_security(&RSISParameters {
        m: config.witness_height as u64,
        n: config.basic_commitment_rank as u64,
        length_bound: worse_bound.ceil() as u64,
    });
    tracing::debug!(
        "Basic commitment estimated security for extraction: {:?} with rank {}",
        basic_commitment_security,
        config.basic_commitment_rank
    );
    enforce_estimated_security(
        "sumcheck basic commitment",
        config.witness_height,
        config.basic_commitment_rank,
        worse_bound,
        &basic_commitment_security,
    );
    if exhaustive_audit_enabled() {
        eprintln!(
                    "HARDNESS_AUDIT sumcheck provenance=verifier_enforced extracted_bound={extracted_witness_bound} \
             decomposed_folded_witness_l2={decomposed_folded_witness_l2} \
             observed_recomposed_folded_witness_l2={observed_recomposed_folded_witness_l2} \
             certified_combined_bound={} \
             certified_folded_recomposed_bound={} \
             certified_projection_recomposed_bound={} \
             projection_argued_bound={argued_witness_bound} selected_bound={worse_bound} \
             basic_rank={} estimated_security={basic_commitment_security:?}",
            config.norm_bound,
            config.folded_recomposed_norm_bound,
            config.projection_recomposed_norm_bound,
            config.basic_commitment_rank
        );
    }
}

pub fn check_intermediate_round(
    config: &IntermediateConfig,
    next_round_witness_data: &[RingElement],
    folded_witness_data: &[RingElement],
    projection_image_ct_data: &[RingElement],
) {
    tracing::debug!("=== Debug Hardness Check for Intermediate Round ===");

    let recommited_ell_2_norm = norms::l2_norm(next_round_witness_data);
    let recommited_ell_inf_norm = norms::inf_norm(next_round_witness_data);
    tracing::debug!(
        "Next round witness norms: L_2 = {}, L_inf = {}, bit_len = {}, MOD_Q = {}",
        recommited_ell_2_norm,
        recommited_ell_inf_norm,
        recommited_ell_inf_norm.ilog2(),
        MOD_Q
    );

    let folded_witness_ell_2_norm = norms::l2_norm(folded_witness_data);
    let folded_witness_inf_norm = norms::inf_norm(folded_witness_data);
    tracing::debug!(
        "Folded witness norms: L_2 = {}, L_inf = {}, bit_len = {}, MOD_Q = {}",
        folded_witness_ell_2_norm,
        folded_witness_inf_norm,
        folded_witness_inf_norm.ilog2(),
        MOD_Q
    );

    let recomposed_witness_bound = certified_recomposition_bound(
        config.norm_bound,
        config.witness_decomposition_base_log,
        config.witness_decomposition_chunks,
    );

    tracing::debug!("Folded witness norm: {}", recomposed_witness_bound);

    let projection_l2_norm = norms::l2_norm_coeffs(projection_image_ct_data);

    let extracted_witness_bound =
        recomposed_witness_bound * SPECTRAL_OP_NORM_SAFE_BOUND * EXTRACTION_SLACK;

    let argued_witness_bound = config.projection_norm_bound / JL_ALPHA_RP;

    let uniqueness_lhs = argued_witness_bound * argued_witness_bound;
    let uniqueness_rhs = MOD_Q as f64 / 2f64;
    let uniqueness_holds = uniqueness_lhs < uniqueness_rhs;
    if exhaustive_audit_enabled() {
        eprintln!(
            "HARDNESS_AUDIT intermediate provenance=verifier_enforced uniqueness_holds={uniqueness_holds} \
             observed_combined_l2={recommited_ell_2_norm} observed_folded_l2={folded_witness_ell_2_norm} \
             observed_projection_l2={projection_l2_norm} certified_combined_bound={} \
             certified_projection_bound={} \
             projection_argued_bound={argued_witness_bound} \
             bound_squared={uniqueness_lhs} q_over_two={uniqueness_rhs}",
            config.norm_bound,
            config.projection_norm_bound
        );
    } else {
        assert!(
            uniqueness_holds,
            "Projection-argued witness bound too large for inner-product norm extraction: \
             argued_witness_bound={argued_witness_bound}, \
             bound_squared={uniqueness_lhs}, q_over_two={uniqueness_rhs}"
        );
    }

    let worse_bound = if extracted_witness_bound > argued_witness_bound {
        tracing::debug!(
            "Using extracted witness bound {} for security estimation.",
            extracted_witness_bound
        );
        extracted_witness_bound
    } else {
        tracing::debug!(
            "Using projection-argued witness bound {} for security estimation.",
            argued_witness_bound
        );
        argued_witness_bound
    };

    let basic_commitment_security = estimate_rsis_security(&RSISParameters {
        m: config.witness_height as u64,
        n: config.basic_commitment_rank as u64,
        length_bound: worse_bound.ceil() as u64,
    });
    tracing::debug!(
        "Basic commitment estimated security for extraction: {:?} with rank {}",
        basic_commitment_security,
        config.basic_commitment_rank
    );
    enforce_estimated_security(
        "intermediate basic commitment",
        config.witness_height,
        config.basic_commitment_rank,
        worse_bound,
        &basic_commitment_security,
    );
    if exhaustive_audit_enabled() {
        eprintln!(
            "HARDNESS_AUDIT intermediate provenance=verifier_enforced extracted_bound={extracted_witness_bound} \
             certified_combined_bound={} certified_projection_bound={} \
             projection_argued_bound={argued_witness_bound} selected_bound={worse_bound} \
             basic_rank={} estimated_security={basic_commitment_security:?}",
            config.norm_bound,
            config.projection_norm_bound,
            config.basic_commitment_rank
        );
    }
}

pub fn check_simple_round(
    config: &SimpleConfig,
    folded_witness_data: &[RingElement],
    projection_image_ct_data: &[RingElement],
) {
    tracing::debug!("=== Debug Hardness Check for Simple Round ===");

    let folded_witness_l2_norm = norms::l2_norm(folded_witness_data);
    tracing::debug!("Folded witness norm: {}", folded_witness_l2_norm);

    let projection_l2_norm = norms::l2_norm_coeffs(projection_image_ct_data);

    let extracted_witness_bound =
        config.witness_norm_bound * SPECTRAL_OP_NORM_SAFE_BOUND * EXTRACTION_SLACK;

    let argued_witness_bound = config.projection_norm_bound / JL_ALPHA_RP;
    let uniqueness_lhs = config.witness_width as f64 * argued_witness_bound.powi(2);
    let uniqueness_rhs = MOD_Q as f64 / 2.0;
    let uniqueness_holds = uniqueness_lhs < uniqueness_rhs;
    if exhaustive_audit_enabled() {
        eprintln!(
            "HARDNESS_AUDIT simple provenance=verifier_enforced uniqueness_holds={uniqueness_holds} \
             width={} observed_witness_l2={folded_witness_l2_norm} \
             observed_projection_l2={projection_l2_norm} certified_witness_bound={} \
             certified_projection_bound={} argued_witness_bound={argued_witness_bound} \
             width_times_bound_squared={uniqueness_lhs} q_over_two={uniqueness_rhs}",
            config.witness_width,
            config.witness_norm_bound,
            config.projection_norm_bound
        );
    } else {
        assert!(
            uniqueness_holds,
            "Simple projection bound exceeds centered uniqueness gate: width={}, \
             certified_projection_bound={}, argued_witness_bound={}, \
             width_times_bound_squared={}, q_over_two={}",
            config.witness_width,
            config.projection_norm_bound,
            argued_witness_bound,
            uniqueness_lhs,
            uniqueness_rhs
        );
    }
    let worse_bound = if extracted_witness_bound > argued_witness_bound {
        tracing::debug!(
            "Using extracted witness bound {} for security estimation.",
            extracted_witness_bound
        );
        extracted_witness_bound
    } else {
        tracing::debug!(
            "Using projection-argued witness bound {} for security estimation.",
            argued_witness_bound
        );
        argued_witness_bound
    };

    let basic_commitment_security = estimate_rsis_security(&RSISParameters {
        m: config.witness_height as u64,
        n: config.basic_commitment_rank as u64,
        length_bound: worse_bound.ceil() as u64,
    });
    tracing::debug!(
        "Basic commitment estimated security for extraction: {:?} with rank {}",
        basic_commitment_security,
        config.basic_commitment_rank
    );
    enforce_estimated_security(
        "simple basic commitment",
        config.witness_height,
        config.basic_commitment_rank,
        worse_bound,
        &basic_commitment_security,
    );
    if exhaustive_audit_enabled() {
        eprintln!(
            "HARDNESS_AUDIT simple provenance=verifier_enforced extracted_bound={extracted_witness_bound} \
             certified_witness_bound={} certified_projection_bound={} \
             projection_argued_bound={argued_witness_bound} selected_bound={worse_bound} \
             basic_rank={} estimated_security={basic_commitment_security:?}",
            config.witness_norm_bound,
            config.projection_norm_bound,
            config.basic_commitment_rank
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        certified_recomposition_bound, enforce_estimated_security, exhaustive_audit_enabled,
        minimum_rank_for_bound, recomposition_l2_operator_norm, EXTRACTION_SLACK, JL_ALPHA_RP,
        SPECTRAL_OP_NORM_SAFE_BOUND,
    };
    use crate::{
        common::{
            config::MOD_Q,
            estimator::{estimate_rsis_security, EstimatorResult, RSISParameters},
        },
        protocol::{
            commitment::RecursionConfig,
            config::{Config, Projection},
            params::{P_EN_MEDIUM, P_EN_NARROW_LARGE, P_EN_SMALL, P_LARGE, P_MEDIUM, P_SMALL},
        },
    };

    fn certify_sis(scope: &str, m: usize, rank: usize, bound: f64) {
        let result = estimate_rsis_security(&RSISParameters {
            m: m as u64,
            n: rank as u64,
            length_bound: bound.ceil() as u64,
        });
        enforce_estimated_security(scope, m, rank, bound, &result);
        eprintln!(
            "STATIC_CERT security scope={scope:?} m={m} rank={rank} bound={bound} result={result:?}"
        );
    }

    fn certify_recursion(
        scope: &str,
        config: &RecursionConfig,
        base_size: usize,
        outer_bound: f64,
        inner_bound: f64,
        depth: usize,
    ) {
        let m = base_size * config.decomposition_chunks;
        let bound = if config.next.is_some() {
            outer_bound
        } else {
            inner_bound
        };
        certify_sis(
            &format!("{scope} recursive depth {depth}"),
            m,
            config.rank,
            bound,
        );
        if let Some(next) = config.next.as_deref() {
            certify_recursion(
                scope,
                next,
                config.rank,
                outer_bound,
                inner_bound,
                depth + 1,
            );
        }
    }

    fn next_width(config: &Config) -> usize {
        match config {
            Config::Sumcheck(next) => next.witness_width,
            Config::Intermediate(next) => next.witness_width,
            Config::Simple(next) => next.witness_width,
        }
    }

    fn certify_registered_config(name: &str, config: &Config) {
        let mut round = 0;
        let mut current = config;
        loop {
            match current {
                Config::Sumcheck(sumcheck) => {
                    certify_recursion(
                        &format!("{name}/r{round}/commitment"),
                        &sumcheck.commitment_recursion,
                        sumcheck.basic_commitment_rank.next_power_of_two() * sumcheck.witness_width,
                        sumcheck.norm_bound,
                        sumcheck.most_inner_norm_bound,
                        0,
                    );
                    certify_recursion(
                        &format!("{name}/r{round}/opening"),
                        &sumcheck.opening_recursion,
                        sumcheck.nof_openings * sumcheck.witness_width,
                        sumcheck.norm_bound,
                        sumcheck.most_inner_norm_bound,
                        0,
                    );
                    match &sumcheck.projection_recursion {
                        Projection::Coarse(recursion) => certify_recursion(
                            &format!("{name}/r{round}/projection"),
                            recursion,
                            sumcheck.witness_height * sumcheck.witness_width
                                / sumcheck.projection_ratio,
                            sumcheck.norm_bound,
                            sumcheck.most_inner_norm_bound,
                            0,
                        ),
                        Projection::Fine(fine) => {
                            certify_recursion(
                                &format!("{name}/r{round}/projection-constant"),
                                &fine.recursion_constant_term,
                                sumcheck.witness_height * sumcheck.witness_width
                                    / sumcheck.projection_ratio,
                                sumcheck.norm_bound,
                                sumcheck.most_inner_norm_bound,
                                0,
                            );
                            certify_recursion(
                                &format!("{name}/r{round}/projection-batched"),
                                &fine.recursion_batched_projection,
                                sumcheck.witness_width * fine.nof_batches,
                                sumcheck.norm_bound,
                                sumcheck.most_inner_norm_bound,
                                0,
                            );
                        }
                        Projection::Skip => {}
                    }

                    let extracted = sumcheck.folded_recomposed_norm_bound
                        * SPECTRAL_OP_NORM_SAFE_BOUND
                        * EXTRACTION_SLACK;
                    let argued = sumcheck.projection_recomposed_norm_bound / JL_ALPHA_RP;
                    if !matches!(sumcheck.projection_recursion, Projection::Skip) {
                        let width = next_width(
                            sumcheck
                                .next
                                .as_deref()
                                .expect("a projected sumcheck round must have a successor"),
                        );
                        let lhs = width as f64 * argued.powi(2);
                        let rhs = MOD_Q as f64 / 2.0;
                        eprintln!(
                            "STATIC_CERT gate name={name:?} round={round} width={width} lhs={lhs} rhs={rhs} holds={}",
                            lhs < rhs
                        );
                        if !exhaustive_audit_enabled() {
                            assert!(lhs < rhs, "{name} round {round} fails centered uniqueness");
                        }
                    }
                    certify_sis(
                        &format!("{name}/r{round}/basic"),
                        sumcheck.witness_height,
                        sumcheck.basic_commitment_rank,
                        extracted.max(argued),
                    );
                    let Some(next) = sumcheck.next.as_deref() else {
                        break;
                    };
                    current = next;
                }
                Config::Intermediate(intermediate) => {
                    let extracted = certified_recomposition_bound(
                        intermediate.norm_bound,
                        intermediate.witness_decomposition_base_log,
                        intermediate.witness_decomposition_chunks,
                    ) * SPECTRAL_OP_NORM_SAFE_BOUND
                        * EXTRACTION_SLACK;
                    let argued = intermediate.projection_norm_bound / JL_ALPHA_RP;
                    let lhs = argued.powi(2);
                    let rhs = MOD_Q as f64 / 2.0;
                    eprintln!(
                        "STATIC_CERT gate name={name:?} round={round} width=1 lhs={lhs} rhs={rhs} holds={}",
                        lhs < rhs
                    );
                    if !exhaustive_audit_enabled() {
                        assert!(
                            lhs < rhs,
                            "{name} intermediate round fails centered uniqueness"
                        );
                    }
                    certify_sis(
                        &format!("{name}/r{round}/intermediate-basic"),
                        intermediate.witness_height,
                        intermediate.basic_commitment_rank,
                        extracted.max(argued),
                    );
                    let Some(next) = intermediate.next.as_deref() else {
                        break;
                    };
                    current = next;
                }
                Config::Simple(simple) => {
                    let extracted =
                        simple.witness_norm_bound * SPECTRAL_OP_NORM_SAFE_BOUND * EXTRACTION_SLACK;
                    let argued = simple.projection_norm_bound / JL_ALPHA_RP;
                    let lhs = simple.witness_width as f64 * argued.powi(2);
                    let rhs = MOD_Q as f64 / 2.0;
                    eprintln!(
                        "STATIC_CERT gate name={name:?} round={round} width={} lhs={lhs} rhs={rhs} holds={}",
                        simple.witness_width,
                        lhs < rhs
                    );
                    if !exhaustive_audit_enabled() {
                        assert!(lhs < rhs, "{name} simple round fails centered uniqueness");
                    }
                    certify_sis(
                        &format!("{name}/r{round}/simple-basic"),
                        simple.witness_height,
                        simple.basic_commitment_rank,
                        extracted.max(argued),
                    );
                    break;
                }
            }
            round += 1;
        }
    }

    #[test]
    fn recomposition_norm_uses_the_radix_not_its_logarithm() {
        let actual = recomposition_l2_operator_norm(3, 4);
        let expected = (1f64 + 8f64.powi(2) + 64f64.powi(2) + 512f64.powi(2)).sqrt();
        assert!((actual - expected).abs() <= f64::EPSILON * expected);
    }

    #[test]
    fn one_chunk_recomposition_is_the_identity() {
        assert_eq!(recomposition_l2_operator_norm(17, 1), 1.0);
    }

    #[test]
    fn target_security_is_accepted() {
        enforce_estimated_security(
            "unit test",
            1,
            1,
            1.0,
            &Ok(EstimatorResult { secpar: 128.0 }),
        );
    }

    #[test]
    fn registered_p28_p30_and_exact_bounds_are_128_bit_certified() {
        certify_registered_config("p28", &P_MEDIUM);
        certify_registered_config("p30", &P_LARGE);
        certify_registered_config("exact-p26", &P_EN_SMALL);
        certify_registered_config("exact-p28", &P_EN_MEDIUM);
        certify_registered_config("exact-p29", &P_EN_NARROW_LARGE);
    }

    #[test]
    fn registered_p26_bounds_are_128_bit_certified() {
        certify_registered_config("p26", &P_SMALL);
    }

    #[test]
    #[should_panic(expected = "SIS estimate below target")]
    fn sub_target_security_is_rejected() {
        enforce_estimated_security(
            "unit test",
            1,
            1,
            1.0,
            &Ok(EstimatorResult { secpar: 127.0 }),
        );
    }

    /// Estimator-only follow-up to the long-running full-chain audit.  The
    /// inputs are the deterministic medium-chain observed bounds printed by
    /// `calibrate_full_chain_norms` with `ROKOKO_AUDIT_HARDNESS=1` on
    /// 2026-08-23.  They are benchmark diagnostics, not theorem-level bounds.
    #[test]
    #[ignore = "prints the rank impact of the recorded full-chain audit"]
    fn audit_medium_observed_minimum_ranks() {
        let rows = [
            ("root", 1usize << 14, 662_371_544_518_619.5),
            ("p1", 1usize << 13, 78_126_653_221.73138),
            ("p2", 1usize << 10, 88_595_099_476.23438),
            ("p3", 1usize << 8, 53_980_621_254.51988),
            ("p4", 1usize << 9, 18_946_047_940.875683),
            ("p5", 1usize << 8, 13_454_245_035.26238),
            ("simple", 1usize << 8, 310_021_936.4924724),
        ];

        for (label, witness_height, bound) in rows {
            eprintln!(
                "HARDNESS_AUDIT_MIN_RANK label={label} observed_bound={bound} \
                 minimum_rank_ge_128={:?}",
                minimum_rank_for_bound(witness_height, bound, 128.0, 64)
            );
        }
    }
}
