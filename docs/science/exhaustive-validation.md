# Exhaustive Validation Method

## Question and method

The executable model asks whether a concrete, iterative SCC construction agrees with an
algorithmically independent definition on every small directed graph. A **model subject** is the
iterative Tarjan implementation. An **oracle** is a separately implemented procedure used to
derive expected results; here it is Boolean transitive closure followed by mutual-reachability
partitioning.

For a labeled graph of size $`n`$, each of the $`n^2`$ possible directed edges is present or
absent. The model therefore checks:

```math
\sum_{n=0}^{4} 2^{n^2} = 66{,}067
```

graphs. For each graph, it also checks every permutation of the vertex domain. This produces
1,575,971 renaming cases.

## Independent observations

| Observation | Subject | Independent comparison |
|---|---|---|
| SCC partition | Iterative low-link traversal | Floyd–Warshall-style Boolean closure and mutual reachability |
| Edge enumeration | Canonical CSR builder | Reversed enumeration with every edge duplicated |
| Quotient edges | Cross-component scan | Induced edge-set renaming under every vertex permutation |
| Wavefront rank | Deterministic topological pass | Rank equality after induced component renaming |
| Native stack use | Explicit heap frames | 20,000-vertex chain on a 256 KiB thread stack |

The CSR check validates both directions independently and then compares their extensional edge
sets. This prevents a forward representation and a reverse representation from being internally
well-shaped but mutually inconsistent.

## Adversarial coverage

Self-loops, empty graphs, complete graphs, disconnected vertices, one large SCC, many singleton
SCCs, duplicate enumerations, reverse enumerations, and every stable-ID permutation occur within
the exhaustive domain. The deep chain separately stresses input-depth independence from native
stack depth.

## Interpretation and limits

Exhaustive small-model agreement is strong executable evidence, but it is not a proof for
unbounded graph size. The Rocq development establishes the size-independent relational laws, and
the TLA+ model establishes the bounded-state lifecycle invariants. Production acceptance must
add refinement evidence connecting the Rust crate to all three pre-implementation layers,
including malformed serialized CSR and resource-limit behavior that the valid-input exhaustive
enumeration does not model.
