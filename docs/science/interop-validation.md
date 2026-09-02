# Snapshot validation method

The snapshot campaign uses independent evidence layers so one implementation mistake cannot make
its own oracle pass. This document states the hypotheses, experiments, negative controls, and
acceptance interpretation.

## Hypotheses

1. Every canonical forward CSR graph and semantic profile has one version 1.0 encoding.
2. Decoding an emitted encoding returns the same dense graph and profile.
3. Raw edge insertion order and duplication do not change bytes after canonical construction.
4. A dense bijection transports graph structure; it is not falsely claimed to preserve bytes.
5. Schema, profile, payload, and digest-purpose changes produce distinct digest invocations.
6. Every malformed identity, length, limit, offset, target, row, stale digest, or cancellation
   condition prevents exact publication.
7. Encoder and decoder work, heap state, and native stack satisfy their stated bounds.
8. Exact-version compatibility is deterministic and fail-closed.

## Evidence layers

| Layer | Question | Independence |
|---|---|---|
| Rocq | Do abstract word-tape functions satisfy unbounded laws? | Proof terms checked by the Rocq kernel |
| TLA+/TLC | Can concurrent admission, rejection, cancellation, and publication interleave unsafely? | Explicit-state transition system |
| Z3 | Can arithmetic or tagged-admission constraints violate their bounds? | Independent satisfiability queries |
| Verus | Do arithmetic and cursor refinement lemmas hold in a Rust-shaped model? | SMT-backed auto-active verification |
| Exhaustive Rust model | Do exact bytes and all small graphs refine the abstract contract? | Standalone codec with no production interop dependency |
| Required-red properties | Is every formal invariant expressed against the intended public API? | Production import is deliberately unresolved |

No layer treats a green result from another layer as its oracle.

## Exhaustive corpus

The independent model enumerates every simple directed graph through three vertices. The corpus
size is:

```math
\sum_{n=0}^{3} 2^{n^2} = 1 + 2 + 16 + 512 = 531.
```

Each graph is paired with three semantic profiles, producing 1,593 profile-bound encodings. Every
vertex permutation is checked, producing 9,321 lawful renaming cases. Reversed and duplicated raw
edge enumerations must canonicalize to the same bytes. Every strict prefix of every encoding is
rejected, totaling 180,696 truncation cases.

The model also injects targeted corruptions into magic, schema, major and minor versions, flags,
profile, declared payload length, actual length, limits, offset origin, offset order, terminal
offset, target range, and row order.

## Golden vectors

Two vectors are literal constants in
`formal/model/exhaustive_interop.rs`:

- an 84-byte empty graph with a zero profile; and
- a 96-byte two-vertex, one-edge graph with profile bytes 0 through 31.

The model constructs real bytes and compares their hex encoding to those constants. The
documentation repeats the same vectors for cross-language consumers. A producer that changes a
field width, endianness, ordering, or padding fails before publication.

## Causal negative controls

The positive TLC model explores two requests, eleven input classes, cancellation at every
reachable reading/admission point, publication, release, and all interleavings. It reaches 16,900
distinct states with no invariant violation.

Each mutant changes one cause:

| Mutant | Removed condition | Required counterexample |
|---|---|---|
| `SkipSchema` | Schema mismatch rejection | Bad schema reaches exact publication |
| `SkipCanonical` | Noncanonical CSR rejection | Noncanonical input reaches exact publication |
| `IgnoreCancellation` | Sticky cancellation | Cancelled request reaches exact publication |

A mutant is accepted as evidence only when TLC fails specifically on
`PublicationSound`. A crash, syntax failure, or unrelated invariant failure does not
count.

## Deep-stack experiment

The standalone model constructs a 100,000-vertex directed chain, encodes it, decodes it, compares
it, and destroys all owned values on a thread configured with a 64 KiB native stack. The
experiment is designed to exercise graph-depth-sensitive lifecycle operations, not merely the
main parsing loop. Its purpose is to refute hidden recursive construction, comparison, error, or
drop paths.

The stack experiment does not substitute for the Rocq constant-control theorem. Together they
provide unbounded semantic evidence and concrete implementation-shaped evidence.

## Digest interpretation

The structural models compare digest invocations, not cryptographic outputs. They prove that the
context and ordered key-material fields differ when domain, schema, profile, length, or bytes
differ. BLAKE3's derive-key construction supplies the cryptographic compression boundary
([BLAKE3 specification and rationale](https://github.com/BLAKE3-team/BLAKE3-specs)).

This separation avoids an invalid proof claim: no 256-bit digest can be injective over all finite
byte strings.

## Reproduction

Run the consolidated interop gate:

```bash
scripts/verify-formal.sh interop
```

The runner creates repository-backed scratch under `target/` and places the complete
process inside a user systemd scope with a 4 GiB resident-memory ceiling, no swap, a 100% CPU
quota, at most 64 tasks, and one Cargo build worker. Each proof/model output is captured before
inspection.

Documentation verification is separate:

```bash
scripts/verify-docs.sh
```

The linter is run read-only. Recommendations are manually reviewed; its known auto-repair path is
not used. Every confirmed vinary-doc-lint defect is filed in pgmcp with a reproducer and evidence.

## Acceptance interpretation

Passing this tranche establishes the formal and executable contract that the production
`libvgraph-interop` crate must refine. It does not pretend that the missing crate already
exists: the required-red suite must fail only at the unresolved crate import. The subsequent
implementation turns those same properties green without editing or weakening them.
