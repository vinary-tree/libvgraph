# Witness Formalization Validation

## Research questions

The validation campaign asks:

1. Does every returned path replay to its stated endpoints using canonical edge indices?
2. Are provenance and quotient witness fibers total over their declared key domains, canonical,
   deterministic, and natural under lawful renaming?
3. Does the iterative Lengauer–Tarjan model agree with a definition-level dominance oracle?
4. Does the local/up frontier algorithm agree with the predecessor-based frontier definition?
5. Are invalid, unreachable, exhausted, and cancelled outcomes prevented from becoming exact?
6. Do all input-depth-sensitive control structures remain on the heap?

## Preregistered hypotheses

| Identifier | Hypothesis | Falsifier |
|---|---|---|
| W1 | Path replay is equivalent to indexed relational reachability | A replay with a missing edge, wrong endpoint, or false positive |
| W2 | Flat sidecar union is associative, commutative, idempotent, and duplicate-free | Any corpus fiber violates one law |
| W3 | Complete quotient witness fibers commute with every vertex permutation | A mapped source edge is missing or added |
| W4 | A local numeric selector is not natural under arbitrary automorphism | The symmetric two-edge counterexample commutes |
| W5 | Transporting the strict total order restores selector naturality | A least witness maps to a non-least witness |
| W6 | Iterative Lengauer–Tarjan equals the removal-based dominance definition | Any rooted vertex pair differs |
| W7 | Dominator-tree local/up frontiers equal the predecessor definition | Any owner/target membership differs |
| W8 | Deep operations complete on a 256 KiB native stack | Stack overflow, recursion, or incomplete result |

## Independent oracles

The executable model deliberately uses structurally different references.

| Observation under test | Candidate model | Independent oracle |
|---|---|---|
| Reachability and shortest distance | Explicit BFS with parent edges | Floyd–Warshall Boolean closure and all-pairs distance |
| SCC quotient witness fibers | Canonical edge grouping through SCC map | Mutual reachability classes from closure |
| Dominators | Iterative Lengauer–Tarjan with iterative link-eval compression | Vertex-removal reachability definition |
| Dominance frontiers | Dominator-tree local/up propagation | Direct predecessor-and-dominance definition |
| Provenance union | Two-cursor canonical merge | Set laws and exact sorted member comparison |
| Renaming | Induced edge/component maps | Full permutation transport of relational observations |

The oracle does not call production `libvgraph` functions. This prevents a copied defect from
passing by agreement with itself.

## Exhaustive corpus

`formal/model/exhaustive_witnesses.rs` enumerates every directed simple graph with one through
three dense vertices, including self-loops:

```math
2^{1^2} + 2^{2^2} + 2^{3^2} = 530\ \text{graphs}.
```

Every vertex is used as a root, producing 1,570 rooted dominator/frontier cases. Every vertex
permutation is checked for each graph, producing 3,106 lawful-renaming cases. Each graph is also
reconstructed from a reversed, duplicated edge enumeration to check canonical insertion-order
invariance.

The existing SCC formal model remains complementary: it enumerates 66,067 graphs through four
vertices and 1,575,971 vertex renamings. The witness model does not repeat the Tarjan-versus-
Kosaraju benchmark or algorithm choice.

## Negative controls

Each mutant represents a plausible but incorrect implementation.

| Mutant or malformed input | Required rejection |
|---|---|
| Duplicate provenance members | Flat sidecar validation fails |
| Empty/wrong-length/nonzero-origin offsets | Flat sidecar validation fails |
| Decreasing or terminal-mismatched offsets | Flat sidecar validation fails |
| Unsorted provenance fiber | Flat sidecar validation fails |
| Out-of-range path edge | Replay fails |
| Edge whose source differs from current vertex | Replay fails |
| Truncated path with wrong target | Exact endpoint comparison fails |
| Unreachable vertex dominates itself | Dominance oracle rejects the fact |
| Root is reported as every immediate dominator | Three-vertex chain rejects the result |
| Dominance frontier removes owner self-membership | One-vertex self-loop rejects the result |
| Least local edge index is claimed natural | Symmetric quotient-edge automorphism rejects it |
| Concatenation is used as union without deduplication | Idempotence and shape checks fail |

The process exits unsuccessfully if any negative control stops being detected.

## Lifecycle model

`formal/tla/WitnessMachine.tla` explores a deterministic canonical-edge search, iterative parent
reconstruction, exact replay, work budgets, cancellation, invalid parent state, and unreachable
completion. Two configurations are mandatory:

- `WitnessMachine.cfg` has a reachable goal and budgets that exercise both exact completion and
  exhaustion;
- `WitnessMachineUnreachable.cfg` uses a disconnected goal and exercises unreachable completion
  plus exhaustion.

TLC checks type/ownership, queue uniqueness, parent validity, replay suffix validity, exact witness
validity, outcome separation, work bounds, and heap bounds.

## Deep-stack experiment

The adversarial graph is a 20,000-vertex directed chain. A thread with a 256 KiB native stack:

- constructs canonical CSR;
- runs BFS and reconstructs the 19,999-edge path;
- replays the complete path;
- computes every immediate dominator with iterative Lengauer–Tarjan;
- computes empty dominance frontiers with an explicit dominator-tree stack; and
- constructs and unions an empty flat sidecar.

Every input-shaped frame, queue, ancestor path, parent map, and returned value is heap-resident.

## Formal layers

| Layer | Artifact | Checked obligation |
|---|---|---|
| Relational proof | `formal/rocq/GraphWitnesses.v` | Sidecar laws, replay, reachability, dominance, frontiers, naturality, selector impossibility/correction, outcomes, work, heap control |
| Lifecycle model | `formal/tla/WitnessMachine.tla` | Search/reconstruction state transitions, budgets, cancellation, replay, bounded state |
| Executable oracle | `formal/model/exhaustive_witnesses.rs` | Algorithms versus independent definitions, all bounded graphs/roots/renamings, mutants, deep stack |
| Arithmetic refinement | `formal/verus/witness_refinement.rs` | Flat storage, exact replay work, linear/near-linear charges, graph-domain integer fit |

Run only the pre-implementation witness gate:

```bash
scripts/verify-formal.sh witness
```

Run the full kernel gate:

```bash
scripts/verify-formal.sh all
```

Both commands use repository-backed temporary storage and resource-limited systemd user scopes.
Evidence is transient under `target/verification` and is hashed into pgmcp before cleanup.
