# Formal-First Architecture Contract

## Purpose

This document fixes the boundary between verified semantics and future implementation choices.
A **formal contract** is a machine-checked mathematical or state-transition specification. A
**refinement** is evidence that a concrete implementation preserves the observations of that
contract.

![Formal-first verification flow](../diagrams/formal-first-flow.svg)

## Verification layers

| Layer | Artifact | Obligation |
|---|---|---|
| Relational and resource semantics | `formal/rocq/GraphQuotient.v` | Fibers, quotient edges, acyclicity, renaming, enumeration, wavefront laws, exact logical work, linear heap space, and constant native control depth |
| Lifecycle model | `formal/tla/IterativeGraphMachine.tla` | Explicit frames, bounded frame count, exact discovery/edge/frame accounting, linear work, completion, and cancellation |
| Exhaustive oracle | `formal/model/exhaustive_graphs.rs` | Canonical forward/reverse CSR, every graph through four vertices, independent SCC oracle, all renamings, induced condensation/rank equivalence, adversarial enumerations, exact work counters, and a small-stack deep graph |
| Production refinement | Rust tests and verifier harnesses | CSR validation, iterative Tarjan parity, stable ordering, malformed-input behavior, measured work/allocation bounds, recursion census, and small-stack lifecycle |

The first three layers precede production implementation. The fourth is required before the
implementation task can be verified.

## Public boundary reserved by the contract

The formal work reserves behaviors rather than premature Rust signatures:

- canonical construction from stable vertices and directed edges;
- non-panicking validation of public or deserialized CSR;
- deterministic forward and reverse adjacency;
- strict-linear iterative SCC decomposition and explicit component lookup;
- exact condensation construction and deterministic topological wavefronts; and
- structured incomplete or invalid outcomes when a resource or representation bound is exceeded.

Fixed-point solving, CPG semantics, parser semantics, equality saturation, and weighted algebra are
outside this crate until two consumers demonstrate one identical, independently specified
contract.

## Refinement checklist

A future implementation is admitted only when all answers are “yes.”

1. Does construction preserve the extensional edge relation after sorting and duplicate removal?
2. Does validation reject every malformed offset, endpoint, reverse-edge, and payload alignment?
3. Does SCC output equal independent mutual-reachability classes?
4. Does the quotient contain exactly the witnessed cross-component edges?
5. Is the condensation acyclic and every reported wavefront dependency-valid?
6. Are results invariant under edge enumeration and equivariant under stable-ID renaming?
7. Do 20,000 semantic vertices and 100,000 lifecycle operations fit a 256 KiB native stack where
   the public operation applies?
8. Are caps, cancellation, malformed input, and overflow explicit rather than reported as success?
9. Do serial and future parallel paths produce the same canonical output?
10. Do Rocq, TLC, the exhaustive oracle, Rust tests, strict lint, and documentation lint all pass?
11. On canonical CSR, does SCC work equal $`5|V| + |E|`$ logical events and stay within
    $`5|V|`$ auxiliary vertex slots, excluding returned output?
12. Does phase-complete charging include safe workspace initialization, flat fiber
    materialization, paired condensation CSR construction, wave containers, and exact radix work,
    with a bound at or below $`23|V| + 20|E| + 26{,}627`$?
13. Does reusable temporary storage stay at or below $`10|V| + 2|E| + 2{,}048`$ slots,
    excluding returned partition, condensation, and schedule values?
14. Does a source and call-graph census establish zero recursive control edges on every public
    input-depth-sensitive path?

The symbols $`V`$ and $`E`$ denote source vertices and canonical source edges. The symbol $`R`$
denotes cross-component candidates before quotient deduplication. The symbols $`C`$, $`Q`$, and
$`W`$ denote SCC components, distinct condensation edges, and nonempty dependency waves.
Returned graph, partition, condensation, and schedule storage is output; temporary arrays, queues,
stacks, and sorting buffers are auxiliary storage.
