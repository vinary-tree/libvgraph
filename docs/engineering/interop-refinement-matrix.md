# Snapshot refinement matrix

This matrix connects each externally observable contract to its formal definition, executable
oracle, required-red property, complexity envelope, and production implementation surface.

## Matrix

| Contract | Formal evidence | Executable evidence | Required-red property | Production surface |
|---|---|---|---|---|
| Exact round trip | Rocq `decode_encode_round_trip` | All 1,593 exhaustive encodings | `contract_round_trip` | `encode_snapshot`, `decode_snapshot` |
| Unique canonical bytes | Rocq `canonical_encoding_unique` | Encoding uniqueness map and two golden vectors | `contract_unique_encoding` | Encoder |
| Insertion/duplicate invariance | Rocq `canonical_enumeration_extensional` | Reversed and duplicated enumerations | `contract_insertion_order_and_duplicate_invariance` | Core builder plus encoder |
| Renaming equivariance | Rocq `canonical_rename_round_trip` | 9,321 dense bijections | `contract_lawful_renaming_equivariance` | Domain adapter plus codec |
| Exact schema/version | Rocq rejection theorems and TLC | Corrupted schema/major/minor corpus | `contract_schema_rejection`, `contract_version_rejection` | Decoder header |
| Exact profile | Rocq profile rejection and tagged preimage | Three profiles per graph | `contract_profile_separation` | Header and digest |
| Exact lengths | Z3 arithmetic and Verus fit proof | 180,696 strict prefixes plus trailing bytes | `contract_length_index_and_trailing_rejection` | Decoder preallocation gate |
| Resource limits | Rocq limit theorems and TLC resource invariant | Vertex, edge, and byte limit mutations | `contract_resource_limits_fail_before_publication` | `SnapshotLimits` |
| Canonical CSR | Rocq `canonicalb` and TLC | Offset/target/order mutations | `contract_length_index_and_trailing_rejection` | Iterative validator |
| Stale digest rejection | TLC publication invariant and Z3 conjunction | Payload mutation | `contract_stale_digest_rejection` | `decode_verified_snapshot` |
| Digest separation | Rocq constructor theorems and Z3 datatype | Domain/schema/profile/payload mutations | `contract_digest_domain_schema_and_payload_separation` | BLAKE3 derive-key invocation |
| Stack independence | Rocq cursor machine and TLC depth invariant | 100,000 vertices on 64 KiB | `contract_deep_codec_lifecycle_is_native_stack_independent` | All codec lifecycle paths |
| Exact compatibility | Rocq major/minor rejection | Version replacement matrix | `contract_exact_cross_version_policy` | Decoder dispatch |

The exhaustive registry is
`formal/doc/interop-invariants.tsv`. Its checker requires exactly 65 mapped obligations,
all six evidence layers, every referenced source symbol, and every required-red property with no
unmapped property.

## Cost model

Let `V` denote the vertex count and `E` the canonical edge count. The encoder
allocates the exact output length:

```math
B(V,E) = 80 + 4(V + 1 + E).
```

The structural decoder's returned graph owns exactly `V` dense-node words, `V + 1` forward-offset
words, and `E` forward-target words:

```math
H(V,E) = 2V + 1 + E.
```

Its checked complete structural charge, including dense-node materialization and the core
canonical validator, is bounded by:

```math
W(V,E) = 8 + 2(V + 1) + 2V + 3E = 10 + 4V + 3E.
```

These equations exclude only the returned Rust object header and allocator bookkeeping, which are
constant with respect to graph size. Verified digesting adds one byte-linear BLAKE3 pass and
constant hasher state. No representation requires reverse CSR or a second payload copy.

## Production data structures

- Header fields are read into fixed scalars and fixed arrays.
- Offsets and targets use two preallocated contiguous `Vec<u32>` buffers.
- Dense nodes use one preallocated contiguous `Vec<DenseId>` buffer whose elements have the same
  32-bit representation size.
- Dense targets convert to `DenseId` without maps or per-edge allocation.
- The encoder writes directly into one exactly preallocated `Vec<u8>`.
- Digest updates borrow the caller's snapshot slice; they do not build the conceptual preimage.
- Decoder and validator indices are monotonically increasing `usize` cursors obtained
  only after checked conversion.

## Failure atomicity

No partially constructed `CsrGraph` leaves the decoder. Buffers remain request-local until
all structural checks pass. The verified path retains the same rule for digest mismatch and
cancellation. Errors own only bounded scalar context, so reporting and destruction do not walk a
recursive error chain.

## Deterministic concurrency

The codec API is synchronous and free of global mutable state. A scheduler can run independent
requests concurrently. The result for each request depends only on its bytes, expected profile,
expected digest, limits, and cancellation state. Ordered batch commit belongs to schedlib or the
caller; the codec neither invents an executor nor introduces nondeterministic shared saturation.

## Performance acceptance for implementation

The implementation task must preregister empty, sparse, dense, deep-chain, wide-star, malformed
early-reject, malformed late-reject, and verified-digest workloads. Measurements compare:

- encoder throughput and allocations;
- structural decoder throughput and allocations;
- verified decoder throughput;
- peak resident memory;
- small-stack completion; and
- error-path work.

The core release's default gates remain: no more than 5% wall-time or throughput regression and
no more than 10% peak resident-memory or binary-size regression unless an explicit, evidenced
tradeoff is approved. The relevant comparison is the new implementation against its independent
model or previous interop implementation—not Tarjan against Kosaraju and not unrelated graph
algorithms.
