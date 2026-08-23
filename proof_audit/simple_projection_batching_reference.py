#!/usr/bin/env python3
"""Independent scalar model for terminal Simple c_0 projection batching.

This model deliberately does not import or reimplement the Rust verifier
helper.  It compares three formulations: an explicit tensor definition, the
prover's rightmost-layer-first pair folding, and a verifier dot product using
independently constructed Kronecker weights.  It also runs negative tests and
deterministic differential fuzz over small geometries.
"""

from __future__ import annotations

import hashlib
import json
import random
from pathlib import Path


Q = 1_125_899_906_839_937
GEOMETRIES = ((2, 1), (2, 2), (4, 1), (4, 2), (8, 2))
SUPPORTED_BLOCK_COUNTS = (1, 2, 4, 8)
REJECTED_BLOCK_COUNTS = (3, 5, 7)
FUZZ_CASES = 800


def require_power_of_two(value: int) -> int:
    if value <= 0 or value & (value - 1):
        raise ValueError("projection block count must be a positive power of two")
    return value.bit_length() - 1


def validate_blocks(blocks: list[list[int]], rows: int, columns: int) -> int:
    layers = require_power_of_two(len(blocks))
    expected = rows * columns
    if rows <= 0 or columns <= 0:
        raise ValueError("block geometry must be nonzero")
    if any(len(block) != expected for block in blocks):
        raise ValueError("shape metadata does not match projection block storage")
    return layers


def explicit_tensor_weights(challenges: list[int], modulus: int) -> list[int]:
    count = 1 << len(challenges)
    result = []
    for index in range(count):
        weight = 1
        for layer_index, challenge in enumerate(challenges):
            bit = (index >> (len(challenges) - 1 - layer_index)) & 1
            factor = challenge if bit else 1 - challenge
            weight = weight * factor % modulus
        result.append(weight)
    return result


def reference_tensor_batch(
    blocks: list[list[int]], challenges: list[int], rows: int, columns: int
) -> list[int]:
    expected_layers = validate_blocks(blocks, rows, columns)
    if len(challenges) != expected_layers:
        raise ValueError("c_0 layer count does not match projection block count")
    weights = explicit_tensor_weights(challenges, Q)
    return [
        sum(weight * block[position] for weight, block in zip(weights, blocks)) % Q
        for position in range(rows * columns)
    ]


def prover_pair_fold(
    blocks: list[list[int]], challenges: list[int], rows: int, columns: int
) -> list[int]:
    expected_layers = validate_blocks(blocks, rows, columns)
    if len(challenges) != expected_layers:
        raise ValueError("prover c_0 layer count mismatch")
    survivors = [block[:] for block in blocks]
    # The last layer is the least-significant tensor bit and folds adjacent
    # blocks first.  No challenge is read after one survivor remains.
    for challenge in reversed(challenges):
        if len(survivors) % 2:
            raise ValueError("unpaired projection block")
        next_survivors = []
        for left_index in range(0, len(survivors), 2):
            left = survivors[left_index]
            right = survivors[left_index + 1]
            next_survivors.append(
                [
                    ((1 - challenge) * a + challenge * b) % Q
                    for a, b in zip(left, right)
                ]
            )
        survivors = next_survivors
    if len(survivors) != 1:
        raise ValueError("folding did not end with exactly one survivor")
    return survivors[0]


def verifier_kronecker_dot(
    blocks: list[list[int]], challenges: list[int], rows: int, columns: int
) -> list[int]:
    expected_layers = validate_blocks(blocks, rows, columns)
    if len(challenges) != expected_layers:
        raise ValueError("verifier c_0 layer count mismatch")
    weights = [1]
    for challenge in challenges:
        expanded = []
        for weight in weights:
            expanded.append(weight * (1 - challenge) % Q)
            expanded.append(weight * challenge % Q)
        weights = expanded
    if len(weights) != len(blocks):
        raise ValueError("expanded c_0 tensor does not cover all projection blocks")
    return [
        sum(weights[index] * blocks[index][position] for index in range(len(blocks)))
        % Q
        for position in range(rows * columns)
    ]


def transcript_layers(blocks: list[list[int]], count: int) -> list[int]:
    shake = hashlib.shake_256()
    for block in blocks:
        for value in block:
            shake.update(value.to_bytes(8, "little"))
    raw = shake.digest(8 * count)
    return [int.from_bytes(raw[8 * i : 8 * i + 8], "little") % Q for i in range(count)]


def assert_rejected(action, label: str, results: dict[str, bool]) -> None:
    try:
        action()
    except (ValueError, IndexError):
        results[label] = True
        return
    raise AssertionError(f"negative test did not reject: {label}")


def deterministic_negative_tests() -> dict[str, bool]:
    rows, columns = 2, 2
    blocks = [
        [11, 12, 13, 14],
        [21, 22, 23, 24],
        [31, 32, 33, 34],
        [41, 42, 43, 44],
    ]
    challenges = [7, 19]
    baseline = reference_tensor_batch(blocks, challenges, rows, columns)
    results: dict[str, bool] = {}

    modified = [block[:] for block in blocks]
    modified[2][1] += 1
    assert reference_tensor_batch(modified, challenges, rows, columns) != baseline
    results["modified_projection_block_changes_claim"] = True

    modified_challenge = challenges[:]
    modified_challenge[0] += 1
    assert reference_tensor_batch(blocks, modified_challenge, rows, columns) != baseline
    results["modified_c0_changes_claim"] = True

    swapped = [blocks[1], blocks[0], blocks[2], blocks[3]]
    assert reference_tensor_batch(swapped, challenges, rows, columns) != baseline
    results["swapped_block_order_changes_claim"] = True

    rotated = blocks[1:] + blocks[:1]
    assert reference_tensor_batch(rotated, challenges, rows, columns) != baseline
    results["tensor_index_offset_changes_claim"] = True

    assert_rejected(
        lambda: verifier_kronecker_dot(blocks, challenges[:-1], rows, columns),
        "deleted_c0_layer_rejected",
        results,
    )
    assert_rejected(
        lambda: verifier_kronecker_dot(blocks, challenges + [23], rows, columns),
        "extra_c0_layer_rejected",
        results,
    )
    assert_rejected(
        lambda: verifier_kronecker_dot(blocks[:-1], challenges, rows, columns),
        "prover_verifier_block_count_mismatch_rejected",
        results,
    )
    malformed = [block[:] for block in blocks]
    malformed[0].pop()
    assert_rejected(
        lambda: verifier_kronecker_dot(malformed, challenges, rows, columns),
        "shape_metadata_mismatch_rejected",
        results,
    )
    for count in REJECTED_BLOCK_COUNTS:
        non_power_blocks = [[i] for i in range(count)]
        assert_rejected(
            lambda b=non_power_blocks: verifier_kronecker_dot(b, [2], 1, 1),
            f"non_power_of_two_{count}_rejected",
            results,
        )

    transcript_a = transcript_layers(blocks, len(challenges))
    transcript_b = transcript_layers(swapped, len(challenges))
    assert transcript_a != transcript_b
    results["transcript_block_order_changes_challenges"] = True

    zero_blocks = [blocks[0], [0, 0, 0, 0], blocks[2], blocks[3]]
    assert (
        reference_tensor_batch(zero_blocks, challenges, rows, columns)
        == prover_pair_fold(zero_blocks, challenges, rows, columns)
        == verifier_kronecker_dot(zero_blocks, challenges, rows, columns)
    )
    results["zero_block_has_only_weighted_zero_contribution"] = True

    single = [[5, 6, 7, 8]]
    assert reference_tensor_batch(single, [], rows, columns) == single[0]
    assert prover_pair_fold(single, [], rows, columns) == single[0]
    assert verifier_kronecker_dot(single, [], rows, columns) == single[0]
    results["single_block_reads_no_c0_layer"] = True
    return results


def differential_fuzz() -> int:
    rng = random.Random(0xC0B10C)
    cases = 0
    for _ in range(FUZZ_CASES):
        block_count = rng.choice(SUPPORTED_BLOCK_COUNTS)
        rows, columns = rng.choice(GEOMETRIES)
        blocks = [
            [rng.randrange(Q) for _ in range(rows * columns)]
            for _ in range(block_count)
        ]
        layers = [rng.randrange(Q) for _ in range(require_power_of_two(block_count))]
        reference = reference_tensor_batch(blocks, layers, rows, columns)
        assert prover_pair_fold(blocks, layers, rows, columns) == reference
        assert verifier_kronecker_dot(blocks, layers, rows, columns) == reference
        cases += 1
    return cases


def main() -> None:
    parameterized = []
    for block_count in SUPPORTED_BLOCK_COUNTS:
        layers = [7 + 12 * i for i in range(require_power_of_two(block_count))]
        for rows, columns in GEOMETRIES:
            blocks = [
                [1000 * block + 10 * row + column for row in range(rows) for column in range(columns)]
                for block in range(block_count)
            ]
            reference = reference_tensor_batch(blocks, layers, rows, columns)
            assert prover_pair_fold(blocks, layers, rows, columns) == reference
            assert verifier_kronecker_dot(blocks, layers, rows, columns) == reference
            parameterized.append(
                {"blocks": block_count, "rows": rows, "columns": columns, "passed": True}
            )

    negative = deterministic_negative_tests()
    fuzz_cases = differential_fuzz()
    result = {
        "status": "proved locally",
        "modulus": Q,
        "invariant": {
            "supported_block_counts": list(SUPPORTED_BLOCK_COUNTS),
            "non_power_of_two_policy": "reject; no padding is defined",
            "c0_layer_count": "log2(n_blocks)",
            "tensor_order": "layer 0 is the most-significant block-index bit; the last layer folds adjacent blocks first",
        },
        "parameterized_cases": parameterized,
        "negative_tests": negative,
        "differential_fuzz_cases": fuzz_cases,
    }
    output = Path(__file__).resolve().parent / "generated" / "simple_projection_batching_audit.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output}")
    print(
        f"parameterized={len(parameterized)} negative={len(negative)} "
        f"differential_fuzz={fuzz_cases} status={result['status']}"
    )


if __name__ == "__main__":
    main()
