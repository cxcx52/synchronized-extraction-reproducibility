# Terminal `Simple` multi-block projection batching audit

Status: **proved locally** for the fixed-config `Simple` interface and every
block count supported by the implementation.

This audit is local and lightweight.  It did not start, stop, replace, or
modify the running exact-p29 full-chain calibration.

## Root cause

The prover has always batched every projection block with the tensor generated
by `c_0_layers`.  The terminal verifier reconstructed only the first block and
asserted that `c_0_layers` was empty.  This happened to agree with a one-block
terminal geometry, but exact-p29 reaches a `2048 x 2` terminal geometry with
two blocks and therefore one `c_0` layer.

The fix reconstructs the full `c_0 tensor j_batched` claim on the verifier
side, uses the same tensor in the constant-term check, and replaces the old
zero-length assertion with geometry-derived release assertions.

The audit also found two transcript values that were absorbed but not consumed
by equations: padding rows above the unpadded basic commitment rank, and the
omitted final opening column.  Both are now required to use canonical zero
encodings before transcript absorption, preventing them from acting as
Fiat-Shamir grinding nonces.

## Recovered invariant

Let

```text
rows_per_block = projection_ratio * (projection_height / DEGREE)
n_blocks       = witness_height / rows_per_block.
```

The implementation requires all of the following:

1. `projection_height` is a positive power of two and is divisible by
   `DEGREE`;
2. `witness_height` is an exact multiple of `rows_per_block`;
3. `n_blocks` is positive and a power of two;
4. `len(c_0_layers) = log2(n_blocks)`;
5. `len(c_1_layers) = log2(projection_height)`;
6. `c_2_layers` is empty in a `Simple` round;
7. `len(j_batched) = rows_per_block`.

There is no padding rule for non-power-of-two block counts.  Counts 3, 5, and
7 fail closed.  Consequently a one-block round reads no `c_0`, a two-block
round reads one layer, and 4/8 blocks read 2/3 layers.

`precompute_structured_values_fast` iterates layers in reverse while assigning
increasing index bits.  The last sampled layer therefore controls the least
significant bit and folds adjacent blocks first; the first sampled layer
controls the most significant bit.  For layers `(r0,r1)`, the four block
weights are

```text
[(1-r0)(1-r1), (1-r0)r1, r0(1-r1), r0 r1].
```

For block `b`, row `j`, and witness column `k`, the prover and verifier both
compute

```text
sum_b c0[b] * sum_j W[b * rows_per_block + j, k] * j_batched[j].
```

The outer index is the projection block and the inner index is the ring row.
`VerticallyAlignedMatrix` stores columns contiguously
(`data[column * height + row]`).  The projection image is ordered by
column, block, and packed projection row.  The batched projection image is a
row-major `batch x witness-column` matrix.

## Transcript chronology and binding

Prover and verifier use the same order:

```text
inherited state
  -> basic commitment data
  -> opening RHS data
  -> projection-matrix XOF sampling
  -> projection-image data
  -> batch 0: c0, c1, empty c2 layers
  -> batch 1: c0, c1, empty c2 layers
  -> batched-projection image data
  -> witness-column folding challenges.
```

All block contents and commitments are absorbed before their dependent
challenges.  Block count, dimensions, and layer counts are not free proof
messages: they are determined by the trusted verifier configuration and are
now checked exactly before absorption/use.  Hence one fixed verifier
configuration admits no alternative block decomposition for the same proof.
The protocol does not currently add a separate cross-configuration domain tag;
this audit therefore makes no cross-configuration transcript claim.

## Shape and canonical-encoding checks

The terminal verifier now checks:

- padded commitment height and storage, with every unused padding row zero;
- witness height/width tensor dimensions and every evaluation point's layer
  count;
- opening dimensions/storage and a zero omitted final column;
- folded-witness dimensions, used columns, and storage;
- projection-image dimensions, full `used_cols`, and storage;
- fixed batch count and batched-image dimensions/storage;
- exact `c_0`, `c_1`, `c_2`, and `j_batched` lengths.

Malformed inputs fail before transcript absorption or at the corresponding
claim equality.

## `2048 x 2` terminal instance

For exact-p29,

```text
DEGREE             = 128
projection_ratio   = 512
projection_height  = 256
rows_per_block      = 512 * (256/128) = 1024
witness_height      = 2048
n_blocks            = 2
len(c_0_layers)     = 1
len(j_batched)      = 1024
projection image    = 4 x 2
batched image       = 2 x 2
folded witness      = 2048 x 1.
```

Thus `2048 x 2` is an ordinary two-block instance of the general path; no
terminal-specific branch or hard-coded dimension is used.

## Test evidence

Rust component tests (`cargo +nightly test simple_ --lib --features p-29 --offline`):

- actual prover batching, independent manual tensor reference, and verifier
  reconstruction agree for block counts 1, 2, 4, 8 and geometries `2x1`,
  `2x2`, `4x1`, `4x2`, `8x2` (20 cases);
- folded-witness and constant-term equalities agree in every case;
- deleted/extra `c_0` layers, modified/reordered `c_0`, swapped blocks,
  one-position tensor shifts, unsupported counts 3/5/7, and non-power-of-two
  projection heights are rejected or change the checked claim;
- noncanonical commitment padding, noncanonical omitted opening values,
  wrong `used_cols`, shortened/extended storage, wrong image height, wrong
  batch count, and wrong evaluation-point shape fail closed;
- reversing two transcript inputs changes the derived challenges.

Independent Python model
(`proof_audit/simple_projection_batching_reference.py`):

- explicit tensor definition, rightmost-layer-first pair folding, and an
  independently constructed Kronecker dot product agree on the same 20
  parameterized cases;
- 14 deterministic negative/edge tests pass, including zero blocks and the
  no-challenge one-block survivor;
- 800 deterministic differential-fuzz cases pass exactly.

Machine-readable results are in
`proof_audit/generated/simple_projection_batching_audit.json`.

## Remaining boundary

No multi-block correctness blocker was found.  A future protocol that permits
the same transcript to be verified under dynamically selected configurations
would need an explicit configuration/domain binding.  The current result is
for the repository's fixed `SimpleConfig` verification interface.
