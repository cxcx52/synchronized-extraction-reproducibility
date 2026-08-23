use crate::{
    common::{
        arithmetic::precompute_structured_values_fast,
        config::{DEGREE, MOD_Q, NOF_BATCHES},
        hash::HashWrapper,
        matrix::{HorizontallyAlignedMatrix, VerticallyAlignedMatrix},
        norms::{assert_norm_bounded, l2_norm_coeffs},
        projection_matrix::ProjectionMatrix,
        ring_arithmetic::{Representation, RingElement},
        structured_row::{PreprocessedRow, StructuredRow},
    },
    hexl::bindings::{add_mod, eltwise_mult_mod, multiply_mod},
    protocol::{
        boundary::{BoundaryCapture, VerifierBoundary},
        commitment::{commit_basic_internal, BasicCommitment},
        config::{
            Config, IntermediateConfig, IntermediateRoundProof, NextRoundCommitment, RoundProof,
            SimpleConfig, SimpleRoundProof, SumcheckConfig, SumcheckRoundProof,
        },
        crs::VerifierCRS,
        intermediate_sumchecks::{
            context_verifier::IntermediateVerifierSumcheckContext,
            runner_verifier::intermediate_sumcheck_verifier,
        },
        open::{
            evaluation_point_to_structured_row, evaluation_point_to_structured_row_conjugate,
            open_at,
        },
        project_fine::{
            projection_block_count, verifier_sample_projection_challenges_collectively,
            BatchedProjectionChallengesSuccinct,
        },
        sumchecks::{
            context_verifier::{NextVerifierSumcheckContext, VerifierSumcheckContext},
            runner_verifier::sumcheck_verifier,
        },
    },
};

#[tracing::instrument(skip_all, name = "verifier_round")]
pub fn verifier_round(
    crs: &VerifierCRS,
    config: &SumcheckConfig,
    rc_commitment: &[RingElement],
    round_proof: &SumcheckRoundProof,
    evaluation_points_inner: &[StructuredRow],
    evaluation_points_outer: &[StructuredRow],
    claims: &[RingElement],
    sumcheck_context_verifier: &mut VerifierSumcheckContext,
    hash_wrapper_verifier: Option<HashWrapper>,
    boundary: Option<BoundaryCapture<'_, VerifierBoundary>>,
) {
    let mut hash_wrapper_verifier = hash_wrapper_verifier.unwrap_or_else(HashWrapper::new);

    let evaluation_points = sumcheck_verifier(
        &config,
        sumcheck_context_verifier,
        &rc_commitment,
        &round_proof,
        &evaluation_points_inner,
        &evaluation_points_outer,
        &claims,
        &mut hash_wrapper_verifier,
    );

    let at_cut = boundary.as_ref().is_some_and(BoundaryCapture::is_at_cut);

    if at_cut {
        let next_config = match config.next.as_deref() {
            Some(Config::Sumcheck(next_sumcheck_config)) => next_sumcheck_config.clone(),
            _ => panic!("round boundary cut requires the next round to be a Sumcheck round"),
        };
        let next_round_commitment = round_proof
            .next_round_commitment
            .as_ref()
            .expect("cut round must still carry the next-round commitment");
        let commitment_root = match next_round_commitment {
            NextRoundCommitment::Recursive(rc) => rc.clone(),
            NextRoundCommitment::Simple(_) => {
                panic!("round boundary cut requires a recursive next-round commitment")
            }
        };
        let capture = boundary.expect("round boundary cut requires a boundary slot");
        *capture.slot = Some(VerifierBoundary {
            config: next_config,
            commitment_root,
            claims: [
                round_proof.claim_over_witness.clone(),
                round_proof.claim_over_witness_conjugate.conjugate(),
            ],
            evaluation_points,
            transcript: hash_wrapper_verifier,
        });
        return;
    }

    match &round_proof.next {
        Some(next_round_proof) => {
            let next_round_commitment =
                round_proof
                    .next_round_commitment
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!(
                        "Next round commitment must be present when next round proof is present."
                    )
                    });

            match next_round_proof.as_ref() {
                RoundProof::Sumcheck(next_sumcheck_round_proof) => {
                    let next_sumcheck_config = match &config.next {
                        Some(next_config) => match next_config.as_ref() {
                            Config::Sumcheck(next_sumcheck_config) => next_sumcheck_config,
                            _ => panic!("Expected sumcheck config for next round."),
                        },
                        None => panic!("Next sumcheck config must be present."),
                    };

                    let (new_evaluation_points_outer, new_evaluation_points_inner) =
                        evaluation_points
                            .split_at(next_sumcheck_config.witness_width.ilog2() as usize);

                    let next_round_commiments_recursive = match &next_round_commitment {
                        NextRoundCommitment::Recursive(rc) => rc,
                        _ => panic!("Expected recursive commitment for next round."),
                    };

                    let inner_rows = [
                        evaluation_point_to_structured_row(new_evaluation_points_inner),
                        evaluation_point_to_structured_row_conjugate(new_evaluation_points_inner),
                    ];
                    let outer_rows = [
                        evaluation_point_to_structured_row(new_evaluation_points_outer),
                        evaluation_point_to_structured_row_conjugate(new_evaluation_points_outer),
                    ];
                    let new_claims = [
                        round_proof.claim_over_witness.clone(),
                        round_proof.claim_over_witness_conjugate.conjugate(),
                    ];

                    verifier_round(
                        crs,
                        &next_sumcheck_config,
                        next_round_commiments_recursive.as_slice(),
                        next_sumcheck_round_proof,
                        &inner_rows,
                        &outer_rows,
                        &new_claims,
                        match sumcheck_context_verifier.next.as_deref_mut() {
                            Some(NextVerifierSumcheckContext::Simple(ctx)) => ctx,
                            _ => panic!("Expected Simple context for next round."),
                        },
                        Some(hash_wrapper_verifier),
                        boundary.and_then(BoundaryCapture::advance),
                    );
                }

                RoundProof::Simple(next_simple_round_proof) => {
                    assert!(
                        boundary.is_none(),
                        "round boundary cut requires the next round to be a Sumcheck round"
                    );
                    let next_simple_config = match &config.next {
                        Some(next_config) => match next_config.as_ref() {
                            Config::Simple(next_simple_config) => next_simple_config,
                            _ => panic!("Expected simple config for next round."),
                        },
                        None => panic!("Next simple config must be present."),
                    };

                    let (new_evaluation_points_outer, new_evaluation_points_inner) =
                        evaluation_points
                            .split_at(next_simple_config.witness_width.ilog2() as usize);

                    let commitment = match &next_round_commitment {
                        NextRoundCommitment::Simple(basic_commitment) => basic_commitment,
                        _ => panic!("Expected simple commitment for next round."),
                    };

                    let inner_rows = [
                        evaluation_point_to_structured_row(new_evaluation_points_inner),
                        evaluation_point_to_structured_row_conjugate(new_evaluation_points_inner),
                    ];
                    let outer_rows = [
                        evaluation_point_to_structured_row(new_evaluation_points_outer),
                        evaluation_point_to_structured_row_conjugate(new_evaluation_points_outer),
                    ];
                    let new_claims = [
                        round_proof.claim_over_witness.clone(),
                        round_proof.claim_over_witness_conjugate.conjugate(),
                    ];

                    verifier_round_simple(
                        crs,
                        next_simple_config,
                        commitment,
                        next_simple_round_proof,
                        &inner_rows,
                        &outer_rows,
                        &new_claims,
                        Some(hash_wrapper_verifier),
                    );
                }
                RoundProof::Intermediate(next_intermediate_round_proof) => {
                    assert!(
                        boundary.is_none(),
                        "round boundary cut requires the next round to be a Sumcheck round"
                    );
                    let next_intermediate_config = match &config.next {
                        Some(next_config) => match next_config.as_ref() {
                            Config::Intermediate(next_intermediate_config) => {
                                next_intermediate_config
                            }
                            _ => panic!("Expected intermediate config for next round."),
                        },
                        None => panic!("Next intermediate config must be present."),
                    };

                    let (new_evaluation_points_outer, new_evaluation_points_inner) =
                        evaluation_points
                            .split_at(next_intermediate_config.witness_width.ilog2() as usize);

                    let commitment = match &next_round_commitment {
                        NextRoundCommitment::Simple(basic_commitment) => basic_commitment,
                        _ => panic!(
                            "Expected NextRoundCommitment::Simple for intermediate next round."
                        ),
                    };

                    let inner_rows = [
                        evaluation_point_to_structured_row(new_evaluation_points_inner),
                        evaluation_point_to_structured_row_conjugate(new_evaluation_points_inner),
                    ];
                    let outer_rows = [
                        evaluation_point_to_structured_row(new_evaluation_points_outer),
                        evaluation_point_to_structured_row_conjugate(new_evaluation_points_outer),
                    ];
                    let new_claims = [
                        round_proof.claim_over_witness.clone(),
                        round_proof.claim_over_witness_conjugate.conjugate(),
                    ];

                    verifier_round_intermediate(
                        crs,
                        next_intermediate_config,
                        commitment,
                        next_intermediate_round_proof,
                        &inner_rows,
                        &outer_rows,
                        &new_claims,
                        match sumcheck_context_verifier.next.as_deref_mut() {
                            Some(NextVerifierSumcheckContext::Intermediate(ctx)) => ctx,
                            _ => panic!("Expected Intermediate context for next round."),
                        },
                        Some(hash_wrapper_verifier),
                    );
                }
            }
        }
        None => {
            assert!(
                boundary.is_none(),
                "round boundary cut requested past the end of the round chain"
            );
        }
    }
}

pub(crate) fn fold_matrix_claims(
    matrix: &HorizontallyAlignedMatrix<RingElement>,
    folding_challenges: &[RingElement],
) -> Vec<RingElement> {
    debug_assert_eq!(
        folding_challenges.len(),
        matrix.width,
        "folding_challenges length must equal matrix width"
    );
    let mut folded_claims = vec![RingElement::zero(Representation::IncompleteNTT); matrix.height];
    let mut temp = RingElement::zero(Representation::IncompleteNTT);

    for row in 0..matrix.height {
        for col in 0..matrix.width {
            temp *= (&matrix[(row, col)], &folding_challenges[col]);
            folded_claims[row] += &temp;
        }
    }

    folded_claims
}

#[tracing::instrument(skip_all, name = "verifier_round_intermediate")]
pub fn verifier_round_intermediate(
    crs: &VerifierCRS,
    config: &IntermediateConfig,
    commitment: &BasicCommitment,
    round_proof: &IntermediateRoundProof,
    evaluation_points_inner: &[StructuredRow],
    evaluation_points_outer: &[StructuredRow],
    claims: &[RingElement],
    sumcheck_context_verifier: &mut IntermediateVerifierSumcheckContext,
    hash_wrapper: Option<HashWrapper>,
) {
    let mut hash_wrapper = hash_wrapper.unwrap_or_else(HashWrapper::new);
    hash_wrapper.update_with_ring_element_slice(&commitment.data);

    hash_wrapper.update_with_ring_element_slice(&round_proof.opening_rhs.data);

    let mut temp = RingElement::zero(Representation::IncompleteNTT);

    let mut last_col_opening_rhs =
        vec![RingElement::zero(Representation::IncompleteNTT); round_proof.opening_rhs.height];
    // instead of checking if claims are consistent with opening_rhs,
    // we assume they are and recompute the last column of opening_rhs to save on communication
    {
        let _s = tracing::info_span!("verifier_intermediate::recompute_last_col").entered();
        for i in 0..round_proof.opening_rhs.height {
            let preprocessed_row =
                PreprocessedRow::from_structured_row(&evaluation_points_outer[i]);

            last_col_opening_rhs[i].set_from(&claims[i]);
            for col in 0..round_proof.opening_rhs.width - 1 {
                temp *= (
                    &round_proof.opening_rhs[(i, col)],
                    &preprocessed_row.preprocessed_row[col],
                );
                last_col_opening_rhs[i] -= &temp;
            }
            temp.set_from(&preprocessed_row.preprocessed_row[round_proof.opening_rhs.width - 1]);
            temp.from_incomplete_ntt_to_homogenized_field_extensions();
            let mut inv_remaining_challenge = temp.inverse();
            inv_remaining_challenge.from_homogenized_field_extensions_to_incomplete_ntt();
            last_col_opening_rhs[i] *= &inv_remaining_challenge;
            temp.representation = Representation::IncompleteNTT;
        }
    }

    let mut projection_matrix =
        ProjectionMatrix::new(config.projection_ratio, config.projection_height);

    let challenges: [BatchedProjectionChallengesSuccinct; NOF_BATCHES] = {
        let _s = tracing::info_span!("verifier_intermediate::sample_projection").entered();
        projection_matrix.sample(&mut hash_wrapper);
        hash_wrapper.update_with_ring_element_slice(&round_proof.projection_image_ct.data);
        verifier_sample_projection_challenges_collectively(
            &projection_matrix,
            config,
            &mut hash_wrapper,
        )
    };

    let rows_per_chunk = config.projection_height / DEGREE;

    // constant term consistency
    {
        let _s = tracing::info_span!("verifier_intermediate::ct_consistency").entered();
        for i in 0..NOF_BATCHES {
            let c_0_values = precompute_structured_values_fast(&challenges[i].c_0_layers);
            let c_1_values = precompute_structured_values_fast(&challenges[i].c_1_layers);

            debug_assert_eq!(
                c_1_values.len(),
                config.projection_height,
                "c_1_values length mismatch."
            );

            for col in 0..config.witness_width {
                let mut expected_ct = 0u64;

                for row in 0..round_proof.projection_image_ct.height {
                    let chunk_idx = row / rows_per_chunk;
                    let c_0_coeff = c_0_values[chunk_idx];
                    let c_1_offset = (row % rows_per_chunk) * DEGREE;

                    unsafe {
                        eltwise_mult_mod(
                            temp.v.as_mut_ptr(),
                            c_1_values.as_ptr().add(c_1_offset),
                            round_proof.projection_image_ct[(row, col)].v.as_ptr(),
                            DEGREE as u64,
                            MOD_Q,
                        );
                    }

                    let mut row_sum = 0u64;
                    for l in 0..DEGREE {
                        unsafe {
                            row_sum = add_mod(row_sum, temp.v[l], MOD_Q);
                        }
                    }

                    unsafe {
                        let weighted = multiply_mod(row_sum, c_0_coeff, MOD_Q);
                        expected_ct = add_mod(expected_ct, weighted, MOD_Q);
                    }
                }

                let ct = round_proof.batched_projection_image[(i, col)]
                    .constant_term_from_incomplete_ntt();
                assert_eq!(ct, expected_ct);
            }
        }
    }

    hash_wrapper.update_with_ring_element_slice(&round_proof.batched_projection_image.data);

    let mut folding_challenges =
        vec![RingElement::zero(Representation::IncompleteNTT); config.witness_width];
    {
        let _s = tracing::info_span!("verifier_intermediate::sample_folding_challenges").entered();
        hash_wrapper.sample_low_op_norm_ring_vec_into(&mut folding_challenges);
    }

    let next_round_commitment =
        match round_proof
            .next_round_commitment
            .as_ref()
            .unwrap_or_else(|| {
                panic!("Next round commitment must be present for intermediate round proof.")
            }) {
            NextRoundCommitment::Simple(basic_commitment) => basic_commitment,
            _ => panic!("Expected simple commitment for intermediate round."),
        };
    hash_wrapper.update_with_ring_element_slice(&next_round_commitment.data);

    let mut folded_commitment =
        vec![RingElement::zero(Representation::IncompleteNTT); config.basic_commitment_rank];
    for row in 0..config.basic_commitment_rank {
        for col in 0..commitment.width {
            temp *= (&commitment[(row, col)], &folding_challenges[col]);
            folded_commitment[row] += &temp;
        }
    }

    let mut folded_opening_claims =
        vec![RingElement::zero(Representation::IncompleteNTT); round_proof.opening_rhs.height];
    for row in 0..round_proof.opening_rhs.height {
        for col in 0..round_proof.opening_rhs.width - 1 {
            temp *= (
                &round_proof.opening_rhs[(row, col)],
                &folding_challenges[col],
            );
            folded_opening_claims[row] += &temp;
        }
        temp *= (
            &last_col_opening_rhs[row],
            &folding_challenges[round_proof.opening_rhs.width - 1],
        );
        folded_opening_claims[row] += &temp;
    }

    let folded_batched_projection_claims =
        fold_matrix_claims(&round_proof.batched_projection_image, &folding_challenges);

    let l2_norm_proj = l2_norm_coeffs(&round_proof.projection_image_ct.data);
    assert_norm_bounded(
        "projection image in intermediate verifier",
        l2_norm_proj,
        config.projection_norm_bound,
    );

    let intermediate_evaluation_points = intermediate_sumcheck_verifier(
        config,
        sumcheck_context_verifier,
        &round_proof,
        &folded_commitment,
        &folded_opening_claims,
        &folded_batched_projection_claims,
        evaluation_points_inner,
        &challenges,
        &mut hash_wrapper,
    );

    let next_round_proof = round_proof.next.as_ref().unwrap_or_else(|| {
        panic!("Next round proof must be present for intermediate round proof.")
    });

    let next_round_config = config.next.as_ref().unwrap_or_else(|| {
        panic!("Next round config must be present for intermediate round proof.")
    });

    let next_witness_width = match next_round_config.as_ref() {
        Config::Simple(simple_config) => simple_config.witness_width,
        Config::Intermediate(intermediate_config) => intermediate_config.witness_width,
        Config::Sumcheck(_) => {
            unreachable!("Intermediate round must be followed by simple or intermediate round.")
        }
    };
    let (new_evaluation_points_outer, new_evaluation_points_inner) =
        intermediate_evaluation_points.split_at(next_witness_width.ilog2() as usize);
    let inner_rows = [
        evaluation_point_to_structured_row(new_evaluation_points_inner),
        evaluation_point_to_structured_row_conjugate(new_evaluation_points_inner),
    ];
    let outer_rows = [
        evaluation_point_to_structured_row(new_evaluation_points_outer),
        evaluation_point_to_structured_row_conjugate(new_evaluation_points_outer),
    ];
    let new_claims = [
        round_proof.claim_over_witness.clone(),
        round_proof.claim_over_witness_conjugate.conjugate(),
    ];

    match (next_round_proof.as_ref(), next_round_config.as_ref()) {
        (RoundProof::Simple(simple_round_proof), Config::Simple(simple_config)) => {
            verifier_round_simple(
                crs,
                simple_config,
                next_round_commitment,
                simple_round_proof,
                &inner_rows,
                &outer_rows,
                &new_claims,
                Some(hash_wrapper),
            );
        }
        (
            RoundProof::Intermediate(intermediate_round_proof),
            Config::Intermediate(intermediate_config),
        ) => {
            verifier_round_intermediate(
                crs,
                intermediate_config,
                next_round_commitment,
                intermediate_round_proof,
                &inner_rows,
                &outer_rows,
                &new_claims,
                sumcheck_context_verifier.next.as_deref_mut().unwrap(),
                Some(hash_wrapper),
            );
        }
        _ => panic!("Next round proof and config type mismatch."),
    }
}

fn evaluate_simple_batched_projection(
    folded_witness: &VerticallyAlignedMatrix<RingElement>,
    challenge: &BatchedProjectionChallengesSuccinct,
) -> RingElement {
    let c_0_values = precompute_structured_values_fast(&challenge.c_0_layers);
    let chunk_size = challenge.j_batched.len();
    assert_eq!(
        c_0_values.len() * chunk_size,
        folded_witness.height,
        "simple verifier projection batching dimensions do not cover the folded witness"
    );

    let mut result = RingElement::zero(Representation::IncompleteNTT);
    let mut temp = RingElement::zero(Representation::IncompleteNTT);
    for (chunk, &c_0) in c_0_values.iter().enumerate() {
        let mut chunk_result = RingElement::zero(Representation::IncompleteNTT);
        for (j, j_value) in challenge.j_batched.iter().enumerate() {
            temp *= (&folded_witness[(chunk * chunk_size + j, 0)], j_value);
            chunk_result += &temp;
        }
        for coefficient in &mut chunk_result.v {
            unsafe {
                *coefficient = multiply_mod(*coefficient, c_0, MOD_Q);
            }
        }
        result += &chunk_result;
    }
    result
}

fn expected_simple_projection_constant_term(
    projection_image: &VerticallyAlignedMatrix<RingElement>,
    column: usize,
    challenge: &BatchedProjectionChallengesSuccinct,
) -> u64 {
    let c_0_values = precompute_structured_values_fast(&challenge.c_0_layers);
    let c_1_values = precompute_structured_values_fast(&challenge.c_1_layers);
    let rows_per_chunk = c_1_values.len() / DEGREE;
    assert_eq!(
        c_0_values.len() * rows_per_chunk,
        projection_image.height,
        "c_0/c_1 values do not cover the simple projection image"
    );

    let mut expected = 0;
    let mut temp = RingElement::zero(Representation::IncompleteNTT);
    for (chunk, &c_0) in c_0_values.iter().enumerate() {
        for row in 0..rows_per_chunk {
            unsafe {
                eltwise_mult_mod(
                    temp.v.as_mut_ptr(),
                    c_1_values.as_ptr().add(DEGREE * row),
                    projection_image[(chunk * rows_per_chunk + row, column)]
                        .v
                        .as_ptr(),
                    DEGREE as u64,
                    MOD_Q,
                );
            }
            for &coefficient in &temp.v {
                unsafe {
                    let scaled = multiply_mod(coefficient, c_0, MOD_Q);
                    expected = add_mod(expected, scaled, MOD_Q);
                }
            }
        }
    }
    expected
}

fn validate_simple_round_shapes(
    config: &SimpleConfig,
    projection_matrix: &ProjectionMatrix,
    commitment: &BasicCommitment,
    round_proof: &SimpleRoundProof,
    evaluation_points_inner: &[StructuredRow],
    evaluation_points_outer: &[StructuredRow],
    claims: &[RingElement],
) {
    let padded_commitment_rank = if config.basic_commitment_rank == 0 {
        0
    } else {
        config.basic_commitment_rank.next_power_of_two()
    };
    assert_eq!(commitment.height, padded_commitment_rank);
    assert_eq!(commitment.width, config.witness_width);
    assert_eq!(commitment.data.len(), commitment.height * commitment.width);
    let zero = RingElement::zero(Representation::IncompleteNTT);
    for row in config.basic_commitment_rank..commitment.height {
        for column in 0..commitment.width {
            assert_eq!(
                commitment[(row, column)],
                zero,
                "unused padded basic-commitment rows must be canonical zero"
            );
        }
    }

    assert!(config.witness_height.is_power_of_two());
    assert!(config.witness_width.is_power_of_two());
    assert!(!claims.is_empty());
    assert!(claims.len().is_power_of_two());
    assert_eq!(evaluation_points_inner.len(), claims.len());
    assert_eq!(evaluation_points_outer.len(), claims.len());
    for point in evaluation_points_inner {
        assert_eq!(
            point.tensor_layers.len(),
            config.witness_height.ilog2() as usize
        );
    }
    for point in evaluation_points_outer {
        assert_eq!(
            point.tensor_layers.len(),
            config.witness_width.ilog2() as usize
        );
    }
    assert_eq!(round_proof.opening_rhs.height, claims.len());
    assert_eq!(round_proof.opening_rhs.width, config.witness_width);
    assert!(round_proof.opening_rhs.width > 0);
    assert_eq!(
        round_proof.opening_rhs.data.len(),
        round_proof.opening_rhs.height * round_proof.opening_rhs.width
    );
    for row in 0..round_proof.opening_rhs.height {
        assert_eq!(
            round_proof.opening_rhs[(row, round_proof.opening_rhs.width - 1)],
            zero,
            "the omitted final opening column must use its canonical zero encoding"
        );
    }

    assert_eq!(round_proof.folded_witness.height, config.witness_height);
    assert_eq!(round_proof.folded_witness.width, 1);
    assert_eq!(round_proof.folded_witness.used_cols, 1);
    assert_eq!(
        round_proof.folded_witness.data.len(),
        round_proof.folded_witness.height
    );

    let blocks = projection_block_count(projection_matrix, config.witness_height);
    let rows_per_projected_block = config.projection_height / DEGREE;
    assert_eq!(
        round_proof.projection_image_ct.height,
        blocks * rows_per_projected_block
    );
    assert_eq!(round_proof.projection_image_ct.width, config.witness_width);
    assert_eq!(
        round_proof.projection_image_ct.used_cols,
        config.witness_width
    );
    assert_eq!(
        round_proof.projection_image_ct.data.len(),
        round_proof.projection_image_ct.height * round_proof.projection_image_ct.width
    );

    assert_eq!(config.projection_nof_batches, NOF_BATCHES);
    assert_eq!(round_proof.batched_projection_image.height, NOF_BATCHES);
    assert_eq!(
        round_proof.batched_projection_image.width,
        config.witness_width
    );
    assert_eq!(
        round_proof.batched_projection_image.data.len(),
        round_proof.batched_projection_image.height * round_proof.batched_projection_image.width
    );
}

fn validate_simple_batching_challenges(
    config: &SimpleConfig,
    projection_matrix: &ProjectionMatrix,
    challenges: &[BatchedProjectionChallengesSuccinct; NOF_BATCHES],
) {
    let blocks = projection_block_count(projection_matrix, config.witness_height);
    let expected_c_0_layers = blocks.ilog2() as usize;
    let expected_c_1_layers = projection_matrix.projection_height.ilog2() as usize;
    let expected_j_batched =
        projection_matrix.projection_ratio * (projection_matrix.projection_height / DEGREE);
    for challenge in challenges {
        assert_eq!(
            challenge.c_0_layers.len(),
            expected_c_0_layers,
            "c_0 layer count must equal log2 of the exact projection-block count"
        );
        assert_eq!(challenge.c_1_layers.len(), expected_c_1_layers);
        assert_eq!(
            challenge.c_2_layers.len(),
            0,
            "Simple rounds do not batch witness columns through c_2"
        );
        assert_eq!(challenge.j_batched.len(), expected_j_batched);
    }
}

#[tracing::instrument(skip_all, name = "verifier_round_simple")]
pub fn verifier_round_simple(
    crs: &VerifierCRS,
    config: &SimpleConfig,
    commitment: &BasicCommitment,
    round_proof: &SimpleRoundProof,
    evaluation_points_inner: &[StructuredRow],
    evaluation_points_outer: &[StructuredRow],
    claims: &[RingElement],
    hash_wrapper: Option<HashWrapper>,
) {
    let mut hash_wrapper = hash_wrapper.unwrap_or_else(HashWrapper::new);
    let mut projection_matrix =
        ProjectionMatrix::new(config.projection_ratio, config.projection_height);
    validate_simple_round_shapes(
        config,
        &projection_matrix,
        commitment,
        round_proof,
        evaluation_points_inner,
        evaluation_points_outer,
        claims,
    );
    hash_wrapper.update_with_ring_element_slice(&commitment.data);
    hash_wrapper.update_with_ring_element_slice(&round_proof.opening_rhs.data);

    projection_matrix.sample(&mut hash_wrapper);

    hash_wrapper.update_with_ring_element_slice(&round_proof.projection_image_ct.data);

    let challenges: [BatchedProjectionChallengesSuccinct; NOF_BATCHES] =
        verifier_sample_projection_challenges_collectively(
            &projection_matrix,
            config,
            &mut hash_wrapper,
        );
    validate_simple_batching_challenges(config, &projection_matrix, &challenges);

    hash_wrapper.update_with_ring_element_slice(&round_proof.batched_projection_image.data);

    let mut folding_challenges =
        vec![RingElement::zero(Representation::IncompleteNTT); config.witness_width];

    hash_wrapper.sample_low_op_norm_ring_vec_into(&mut folding_challenges);

    // the folded witness is short, so preprocessing the key here is cheap
    let ck: Vec<PreprocessedRow> = crs
        .structured_ck_for_wit_dim(round_proof.folded_witness.height)
        .iter()
        .take(config.basic_commitment_rank)
        .map(PreprocessedRow::from_structured_row)
        .collect();
    let commitment_of_folded_witness = commit_basic_internal(
        &ck,
        &round_proof.folded_witness,
        config.basic_commitment_rank,
    );

    let mut folded_commitment = HorizontallyAlignedMatrix {
        data: vec![
            RingElement::zero(Representation::IncompleteNTT);
            config.basic_commitment_rank * 1
        ],
        width: 1,
        height: config.basic_commitment_rank,
    };

    let mut temp = RingElement::zero(Representation::IncompleteNTT);

    for i in 0..config.basic_commitment_rank {
        for col in 0..commitment.width {
            temp *= (&commitment[(i, col)], &folding_challenges[col]);
            folded_commitment[(i, 0)] += &temp;
        }
    }

    for i in 0..config.basic_commitment_rank {
        assert_eq!(
            commitment_of_folded_witness[(i, 0)],
            folded_commitment[(i, 0)],
            "Folded commitment at row {} does not match expected value.",
            i
        );
    }

    let opening_to_folded_witness = open_at(
        &round_proof.folded_witness,
        evaluation_points_inner,
        evaluation_points_outer,
        false,
    );

    let mut folded_opening = HorizontallyAlignedMatrix {
        data: vec![
            RingElement::zero(Representation::IncompleteNTT);
            round_proof.opening_rhs.height * 1
        ],
        width: 1,
        height: round_proof.opening_rhs.height,
    };

    let mut last_col_opening_rhs =
        vec![RingElement::zero(Representation::IncompleteNTT); round_proof.opening_rhs.height];
    // instead of checking if claims are consistent with opening_rhs,
    // we assume they are and recompute the last column of opening_rhs to save on communication
    for i in 0..round_proof.opening_rhs.height {
        let preprocessed_row = PreprocessedRow::from_structured_row(&evaluation_points_outer[i]);

        last_col_opening_rhs[i].set_from(&claims[i]);
        for col in 0..round_proof.opening_rhs.width - 1 {
            temp *= (
                &round_proof.opening_rhs[(i, col)],
                &preprocessed_row.preprocessed_row[col],
            );
            last_col_opening_rhs[i] -= &temp;
        }
        temp.set_from(&preprocessed_row.preprocessed_row[round_proof.opening_rhs.width - 1]);
        temp.from_incomplete_ntt_to_homogenized_field_extensions();
        let mut inv_remaining_challenge = temp.inverse();
        inv_remaining_challenge.from_homogenized_field_extensions_to_incomplete_ntt();
        last_col_opening_rhs[i] *= &inv_remaining_challenge;
        temp.representation = Representation::IncompleteNTT;
    }

    for i in 0..round_proof.opening_rhs.height {
        for col in 0..commitment.width - 1 {
            temp *= (&round_proof.opening_rhs[(i, col)], &folding_challenges[col]);
            folded_opening[(i, 0)] += &temp;
        }
        temp *= (
            &last_col_opening_rhs[i],
            &folding_challenges[commitment.width - 1],
        );
        folded_opening[(i, 0)] += &temp;
    }

    assert_eq!(opening_to_folded_witness.rhs, folded_opening);

    let mut batched_projection_of_folded_witness = VerticallyAlignedMatrix {
        data: vec![
            RingElement::zero(Representation::IncompleteNTT);
            round_proof.batched_projection_image.height * 1
        ],
        width: 1,
        height: round_proof.batched_projection_image.height,
        used_cols: 1,
    };

    for i in 0..round_proof.batched_projection_image.height {
        batched_projection_of_folded_witness[(i, 0)] =
            evaluate_simple_batched_projection(&round_proof.folded_witness, &challenges[i]);
    }

    let mut folded_batched_projection_image = VerticallyAlignedMatrix {
        data: vec![
            RingElement::zero(Representation::IncompleteNTT);
            round_proof.batched_projection_image.height * 1
        ],
        width: 1,
        height: round_proof.batched_projection_image.height,
        used_cols: 1,
    };

    for i in 0..round_proof.batched_projection_image.height {
        for j in 0..commitment.width {
            temp *= (
                &round_proof.batched_projection_image[(i, j)],
                &folding_challenges[j],
            );
            folded_batched_projection_image[(i, 0)] += &temp;
        }
    }

    assert_eq!(
        batched_projection_of_folded_witness,
        folded_batched_projection_image
    );

    // check constant terms
    for i in 0..NOF_BATCHES {
        for k in 0..config.witness_width {
            let expected_ct = expected_simple_projection_constant_term(
                &round_proof.projection_image_ct,
                k,
                &challenges[i],
            );

            let ct =
                round_proof.batched_projection_image[(i, k)].constant_term_from_incomplete_ntt();
            assert_eq!(ct, expected_ct);
        }
    }

    let mut witness_even_odd =
        vec![RingElement::zero(Representation::IncompleteNTT); round_proof.folded_witness.height];
    witness_even_odd.clone_from_slice(&round_proof.folded_witness.data);

    for w in witness_even_odd.iter_mut() {
        w.from_incomplete_ntt_to_even_odd_coefficients();
    }

    let l2_norm_witness = l2_norm_coeffs(&witness_even_odd);
    let l2_norm_proj = l2_norm_coeffs(&round_proof.projection_image_ct.data);

    assert_norm_bounded(
        "folded witness in simple verifier",
        l2_norm_witness,
        config.witness_norm_bound,
    );
    assert_norm_bounded(
        "projection image in simple verifier",
        l2_norm_proj,
        config.projection_norm_bound,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use crate::{
        common::init_common,
        protocol::{
            fold::fold,
            project_fine::{batch_projection_n_times, project_coefficients},
        },
    };

    fn succinct_challenge_with_c_0(
        template: &BatchedProjectionChallengesSuccinct,
        c_0_layers: Vec<u64>,
    ) -> BatchedProjectionChallengesSuccinct {
        BatchedProjectionChallengesSuccinct {
            c_0_layers,
            c_1_layers: template.c_1_layers.clone(),
            c_2_layers: template.c_2_layers.clone(),
            j_batched: template.j_batched.clone(),
        }
    }

    fn manual_c_0_tensor_weight(layers: &[u64], block: usize) -> u64 {
        let mut weight = 1_u64;
        for (layer_index, &challenge) in layers.iter().enumerate() {
            let bit = (block >> (layers.len() - 1 - layer_index)) & 1;
            let factor = if bit == 1 {
                challenge
            } else {
                (1 + MOD_Q - challenge) % MOD_Q
            };
            weight = ((weight as u128 * factor as u128) % MOD_Q as u128) as u64;
        }
        weight
    }

    fn manual_simple_projection_reference(
        witness: &VerticallyAlignedMatrix<RingElement>,
        column: usize,
        challenge: &BatchedProjectionChallengesSuccinct,
    ) -> RingElement {
        let block_size = challenge.j_batched.len();
        let block_count = 1_usize << challenge.c_0_layers.len();
        assert_eq!(witness.height, block_count * block_size);
        let mut result = RingElement::zero(Representation::IncompleteNTT);
        for block in 0..block_count {
            let block_weight = manual_c_0_tensor_weight(&challenge.c_0_layers, block);
            for row in 0..block_size {
                let mut product = RingElement::zero(Representation::IncompleteNTT);
                product *= (
                    &witness[(block * block_size + row, column)],
                    &challenge.j_batched[row],
                );
                for coefficient in &mut product.v {
                    *coefficient =
                        ((*coefficient as u128 * block_weight as u128) % MOD_Q as u128) as u64;
                }
                result += &product;
            }
        }
        result
    }

    fn simple_shape_fixture(
        opening_rows: usize,
    ) -> (
        SimpleConfig,
        ProjectionMatrix,
        BasicCommitment,
        SimpleRoundProof,
        Vec<StructuredRow>,
        Vec<StructuredRow>,
        Vec<RingElement>,
    ) {
        let config = SimpleConfig {
            witness_height: 8,
            witness_width: 1,
            projection_ratio: 1,
            projection_height: 256,
            projection_nof_batches: NOF_BATCHES,
            basic_commitment_rank: 3,
            witness_norm_bound: f64::INFINITY,
            projection_norm_bound: f64::INFINITY,
        };
        let projection_matrix =
            ProjectionMatrix::new(config.projection_ratio, config.projection_height);
        let zero = RingElement::zero(Representation::IncompleteNTT);
        let commitment = HorizontallyAlignedMatrix {
            data: vec![zero.clone(); 4],
            width: 1,
            height: 4,
        };
        let round_proof = SimpleRoundProof {
            folded_witness: VerticallyAlignedMatrix {
                data: vec![zero.clone(); config.witness_height],
                width: 1,
                height: config.witness_height,
                used_cols: 1,
            },
            projection_image_ct: VerticallyAlignedMatrix {
                data: vec![zero.clone(); 8],
                width: 1,
                height: 8,
                used_cols: 1,
            },
            batched_projection_image: HorizontallyAlignedMatrix {
                data: vec![zero.clone(); NOF_BATCHES],
                width: 1,
                height: NOF_BATCHES,
            },
            opening_rhs: HorizontallyAlignedMatrix {
                data: vec![zero.clone(); opening_rows],
                width: 1,
                height: opening_rows,
            },
        };
        let evaluation_points_inner = (0..opening_rows)
            .map(|_| StructuredRow {
                tensor_layers: vec![
                    RingElement::zero(Representation::IncompleteNTT);
                    config.witness_height.ilog2() as usize
                ],
            })
            .collect::<Vec<_>>();
        let evaluation_points_outer = (0..opening_rows)
            .map(|_| StructuredRow {
                tensor_layers: vec![
                    RingElement::zero(Representation::IncompleteNTT);
                    config.witness_width.ilog2() as usize
                ],
            })
            .collect::<Vec<_>>();
        let claims = vec![zero; opening_rows];
        (
            config,
            projection_matrix,
            commitment,
            round_proof,
            evaluation_points_inner,
            evaluation_points_outer,
            claims,
        )
    }

    fn check_simple_projection_geometry(
        projection_blocks: usize,
        rows_per_block: usize,
        witness_width: usize,
    ) {
        assert_eq!(rows_per_block % (256 / DEGREE), 0);
        let config = SimpleConfig {
            witness_height: projection_blocks * rows_per_block,
            witness_width,
            projection_ratio: rows_per_block / (256 / DEGREE),
            projection_height: 256,
            projection_nof_batches: NOF_BATCHES,
            basic_commitment_rank: 1,
            witness_norm_bound: f64::INFINITY,
            projection_norm_bound: f64::INFINITY,
        };
        let witness = VerticallyAlignedMatrix {
            data: vec![
                RingElement::random(Representation::IncompleteNTT);
                config.witness_height * config.witness_width
            ],
            width: config.witness_width,
            height: config.witness_height,
            used_cols: config.witness_width,
        };
        let mut projection_matrix =
            ProjectionMatrix::new(config.projection_ratio, config.projection_height);
        let mut transcript = HashWrapper::new();
        projection_matrix.sample(&mut transcript);
        let mut prover_transcript = transcript.clone();
        let mut verifier_transcript = transcript;

        let projection_image = project_coefficients(&witness, &projection_matrix);
        let (batched_projection, prover_challenges) = batch_projection_n_times(
            &witness,
            &projection_matrix,
            &mut prover_transcript,
            NOF_BATCHES,
            true,
        );
        let verifier_challenges = verifier_sample_projection_challenges_collectively(
            &projection_matrix,
            &config,
            &mut verifier_transcript,
        );

        for batch in 0..NOF_BATCHES {
            assert_eq!(
                verifier_challenges[batch].c_0_layers.len(),
                projection_blocks.ilog2() as usize
            );
            assert_eq!(
                precompute_structured_values_fast(&verifier_challenges[batch].c_0_layers),
                prover_challenges[batch].c_0_values
            );
            assert_eq!(
                verifier_challenges[batch].c_1_layers,
                prover_challenges[batch].c_1_layers
            );
            assert_eq!(
                verifier_challenges[batch].j_batched,
                prover_challenges[batch].j_batched
            );
            for column in 0..config.witness_width {
                let single_column = VerticallyAlignedMatrix {
                    data: witness.col(column).to_vec(),
                    width: 1,
                    height: witness.height,
                    used_cols: 1,
                };
                assert_eq!(
                    evaluate_simple_batched_projection(&single_column, &verifier_challenges[batch]),
                    batched_projection[(batch, column)]
                );
                assert_eq!(
                    manual_simple_projection_reference(
                        &witness,
                        column,
                        &verifier_challenges[batch]
                    ),
                    batched_projection[(batch, column)]
                );
                assert_eq!(
                    expected_simple_projection_constant_term(
                        &projection_image,
                        column,
                        &verifier_challenges[batch]
                    ),
                    batched_projection[(batch, column)].constant_term_from_incomplete_ntt()
                );
            }

            let folding_challenges = (0..config.witness_width)
                .map(|_| RingElement::random(Representation::IncompleteNTT))
                .collect::<Vec<_>>();
            let folded_witness = fold(&witness, &folding_challenges);
            let mut folded_batched_projection = RingElement::zero(Representation::IncompleteNTT);
            let mut temp = RingElement::zero(Representation::IncompleteNTT);
            for (column, folding_challenge) in folding_challenges.iter().enumerate() {
                temp *= (&batched_projection[(batch, column)], folding_challenge);
                folded_batched_projection += &temp;
            }
            assert_eq!(
                evaluate_simple_batched_projection(&folded_witness, &verifier_challenges[batch]),
                folded_batched_projection
            );
        }
    }

    #[test]
    fn simple_projection_verifier_matches_prover_across_block_geometries() {
        init_common();
        for projection_blocks in [1, 2, 4, 8] {
            for (rows_per_block, witness_width) in [(2, 1), (2, 2), (4, 1), (4, 2), (8, 2)] {
                check_simple_projection_geometry(projection_blocks, rows_per_block, witness_width);
            }
        }
    }

    #[test]
    fn simple_projection_batching_rejects_bad_layers_order_and_geometry() {
        init_common();
        let projection_blocks = 4;
        let rows_per_block = 2;
        let config = SimpleConfig {
            witness_height: projection_blocks * rows_per_block,
            witness_width: 1,
            projection_ratio: 1,
            projection_height: 256,
            projection_nof_batches: NOF_BATCHES,
            basic_commitment_rank: 1,
            witness_norm_bound: f64::INFINITY,
            projection_norm_bound: f64::INFINITY,
        };
        let witness = VerticallyAlignedMatrix {
            data: (1..=config.witness_height)
                .map(|value| RingElement::constant(value as u64, Representation::IncompleteNTT))
                .collect(),
            width: 1,
            height: config.witness_height,
            used_cols: 1,
        };
        let mut projection_matrix =
            ProjectionMatrix::new(config.projection_ratio, config.projection_height);
        let mut transcript = HashWrapper::new();
        projection_matrix.sample(&mut transcript);
        let challenges = verifier_sample_projection_challenges_collectively(
            &projection_matrix,
            &config,
            &mut transcript,
        );
        let valid = succinct_challenge_with_c_0(&challenges[0], vec![2, 3]);
        let expected = evaluate_simple_batched_projection(&witness, &valid);

        for malformed_layers in [vec![2], vec![2, 3, 5]] {
            let malformed = succinct_challenge_with_c_0(&valid, malformed_layers);
            assert!(catch_unwind(AssertUnwindSafe(|| {
                evaluate_simple_batched_projection(&witness, &malformed)
            }))
            .is_err());
        }
        for malformed_len in [1, 3] {
            let malformed_challenges = std::array::from_fn(|batch| {
                let layers = if batch == 0 {
                    vec![2; malformed_len]
                } else {
                    challenges[batch].c_0_layers.clone()
                };
                succinct_challenge_with_c_0(&challenges[batch], layers)
            });
            assert!(catch_unwind(AssertUnwindSafe(|| {
                validate_simple_batching_challenges(
                    &config,
                    &projection_matrix,
                    &malformed_challenges,
                )
            }))
            .is_err());
        }

        let modified_c_0 = succinct_challenge_with_c_0(&valid, vec![4, 3]);
        assert_ne!(
            evaluate_simple_batched_projection(&witness, &modified_c_0),
            expected
        );
        let exchanged_c_0_order = succinct_challenge_with_c_0(&valid, vec![3, 2]);
        assert_ne!(
            evaluate_simple_batched_projection(&witness, &exchanged_c_0_order),
            expected
        );

        let mut exchanged_blocks = witness.clone();
        for row in 0..rows_per_block {
            exchanged_blocks.data.swap(row, rows_per_block + row);
        }
        assert_ne!(
            evaluate_simple_batched_projection(&exchanged_blocks, &valid),
            expected
        );

        let mut shifted_tensor_index = witness.clone();
        shifted_tensor_index.data.rotate_left(1);
        assert_ne!(
            evaluate_simple_batched_projection(&shifted_tensor_index, &valid),
            expected
        );

        for unsupported_blocks in [3, 5, 7] {
            assert!(catch_unwind(AssertUnwindSafe(|| {
                projection_block_count(&projection_matrix, unsupported_blocks * rows_per_block)
            }))
            .is_err());
        }
        let non_power_of_two_height = ProjectionMatrix::new(1, 3 * DEGREE);
        assert!(catch_unwind(AssertUnwindSafe(|| {
            projection_block_count(&non_power_of_two_height, 3)
        }))
        .is_err());
    }

    #[test]
    fn simple_round_shapes_reject_noncanonical_and_inconsistent_metadata() {
        init_common();
        let (config, projection_matrix, commitment, round_proof, inner, outer, claims) =
            simple_shape_fixture(1);
        validate_simple_round_shapes(
            &config,
            &projection_matrix,
            &commitment,
            &round_proof,
            &inner,
            &outer,
            &claims,
        );

        let expect_shape_rejection = |commitment: &BasicCommitment, proof: &SimpleRoundProof| {
            assert!(catch_unwind(AssertUnwindSafe(|| {
                validate_simple_round_shapes(
                    &config,
                    &projection_matrix,
                    commitment,
                    proof,
                    &inner,
                    &outer,
                    &claims,
                )
            }))
            .is_err());
        };

        let mut noncanonical_commitment = commitment.clone();
        noncanonical_commitment[(3, 0)] = RingElement::constant(1, Representation::IncompleteNTT);
        expect_shape_rejection(&noncanonical_commitment, &round_proof);

        let mut noncanonical_opening = simple_shape_fixture(1).3;
        noncanonical_opening.opening_rhs[(0, 0)] =
            RingElement::constant(1, Representation::IncompleteNTT);
        expect_shape_rejection(&commitment, &noncanonical_opening);

        let mut bad_projection_used_cols = simple_shape_fixture(1).3;
        bad_projection_used_cols.projection_image_ct.used_cols = 0;
        expect_shape_rejection(&commitment, &bad_projection_used_cols);

        let mut bad_folded_storage = simple_shape_fixture(1).3;
        bad_folded_storage.folded_witness.data.pop();
        expect_shape_rejection(&commitment, &bad_folded_storage);

        let mut extra_folded_storage = simple_shape_fixture(1).3;
        extra_folded_storage
            .folded_witness
            .data
            .push(RingElement::zero(Representation::IncompleteNTT));
        expect_shape_rejection(&commitment, &extra_folded_storage);

        let mut bad_projection_height = simple_shape_fixture(1).3;
        bad_projection_height.projection_image_ct.height -= 1;
        bad_projection_height.projection_image_ct.data.pop();
        expect_shape_rejection(&commitment, &bad_projection_height);

        let mut bad_batch_count = simple_shape_fixture(1).3;
        bad_batch_count.batched_projection_image.height -= 1;
        bad_batch_count.batched_projection_image.data.pop();
        expect_shape_rejection(&commitment, &bad_batch_count);

        let mut bad_config = config.clone();
        bad_config.projection_nof_batches = NOF_BATCHES - 1;
        assert!(catch_unwind(AssertUnwindSafe(|| {
            validate_simple_round_shapes(
                &bad_config,
                &projection_matrix,
                &commitment,
                &round_proof,
                &inner,
                &outer,
                &claims,
            )
        }))
        .is_err());

        let mut bad_inner = inner.clone();
        bad_inner[0].tensor_layers.pop();
        assert!(catch_unwind(AssertUnwindSafe(|| {
            validate_simple_round_shapes(
                &config,
                &projection_matrix,
                &commitment,
                &round_proof,
                &bad_inner,
                &outer,
                &claims,
            )
        }))
        .is_err());
    }

    #[test]
    fn simple_projection_challenges_bind_transcript_order() {
        init_common();
        let config = SimpleConfig {
            witness_height: 8,
            witness_width: 1,
            projection_ratio: 1,
            projection_height: 256,
            projection_nof_batches: NOF_BATCHES,
            basic_commitment_rank: 1,
            witness_norm_bound: f64::INFINITY,
            projection_norm_bound: f64::INFINITY,
        };
        let first = RingElement::constant(11, Representation::IncompleteNTT);
        let second = RingElement::constant(29, Representation::IncompleteNTT);
        let mut transcript_left_right = HashWrapper::new();
        transcript_left_right.update_with_ring_element_slice(&[first.clone(), second.clone()]);
        let mut transcript_right_left = HashWrapper::new();
        transcript_right_left.update_with_ring_element_slice(&[second, first]);
        let mut matrix_left_right =
            ProjectionMatrix::new(config.projection_ratio, config.projection_height);
        let mut matrix_right_left =
            ProjectionMatrix::new(config.projection_ratio, config.projection_height);
        matrix_left_right.sample(&mut transcript_left_right);
        matrix_right_left.sample(&mut transcript_right_left);
        let left_right = verifier_sample_projection_challenges_collectively(
            &matrix_left_right,
            &config,
            &mut transcript_left_right,
        );
        let right_left = verifier_sample_projection_challenges_collectively(
            &matrix_right_left,
            &config,
            &mut transcript_right_left,
        );
        assert_ne!(left_right[0].c_0_layers, right_left[0].c_0_layers);
    }
}
