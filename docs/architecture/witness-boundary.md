# Witness and Provenance Architecture

## Decision

Graph evidence is a set of typed, separately allocated values over the payload-free `libvgraph`
kernel. The kernel's canonical CSR layout does not acquire generic payload fields. This preserves
cache locality for consumers that need only structural traversal and keeps provenance semantics
out of the shared graph crate.

The formal artifacts reserve behavior before Rust API implementation. They do not publish a
placeholder API.

![Witness evidence architecture](../diagrams/witness-evidence-flow.svg)

## Component boundaries

| Component | Owns | Does not own |
|---|---|---|
| Canonical CSR | Stable vertex order, canonical edge indices, forward/reverse adjacency | Provenance keys, source facts, codecs, hashes |
| Provenance sidecar | Edge-index-to-opaque-key fibers, flat validation, deterministic union | Domain interpretation, graph mutation |
| Path witness | Source, target, canonical edge-index sequence, replay status | Proof that an external semantic claim is true |
| Condensation witness index | Complete source-edge fiber for each quotient edge | Unqualified arbitrary single-witness choice |
| Representative policy | Strict total order transported with lawful renaming | Hidden dependence on local allocation or worker timing |
| Dominator result | Reachability class, immediate dominator tree, dominance frontiers | Control-program semantics, CPG identities |
| Interop layer | Future versioned encoding, digesting, provenance schemas | Hot traversal and analysis loops |

## Data-layout contract

### Flat provenance

The flat sidecar has:

- exactly $`|E|+1`$ offsets;
- origin offset zero;
- monotone adjacent offsets;
- terminal offset equal to the member count; and
- strictly increasing keys within every edge fiber.

This representation supports one allocation for offsets and one for members. Empty fibers cost no
member slots. A graph without provenance need not allocate a sidecar.

### Edge-index paths

Paths store edge indices, not copied endpoints. While replaying edge $`i`$ from current vertex
$`u`$, the validator checks that $`i`$ belongs to $`u`$'s CSR row before reading the target.
Consequently, replay is one sequential pass with no hash lookup and no binary search.

### Dominator tree and frontiers

The internal immediate-dominator representation uses dense vertex indices and an explicit
reachable bitmap. A later public result must distinguish root, reachable child, and unreachable
states. Frontiers use one flat offset/member representation when exposed as a bulk result; nested
vectors are an oracle convenience, not the intended returned hot-path layout.

## Algorithm decomposition

The algorithms remain separate because their laws and output sizes differ.

1. BFS answers rooted reachability and constructs shortest-hop paths.
2. Path replay validates supplied evidence in exact path length.
3. SCC quotient witness construction groups source edges by component pair.
4. Sidecar union propagates opaque keys through a witness fiber.
5. Transported-order selection chooses one representative only when requested.
6. Iterative Lengauer–Tarjan computes immediate dominators.
7. The local/up dominator-tree pass computes output-sensitive frontiers.

### Literate BFS and reconstruction

```text
SEARCH(graph, root)
  allocate discovered, parent_edge, and queue for the vertex domain
  mark root and append it to queue
  advance a numeric queue head until it reaches queue length
    scan the current CSR row in canonical order
    for each newly discovered target
      record the source edge as its only parent
      append the target once
  return discovered and parent_edge

RECONSTRUCT(graph, root, target, parent_edge)
  return UNREACHABLE if target was not discovered
  walk parent edges from target to root into a reverse vector
  reverse the vector in place
  return the edge-index path
```

### Literate Lengauer–Tarjan refinement

```text
DOMINATORS(graph, root)
  number reachable vertices with iterative depth-first search
  build predecessor lists over reachable canonical edges
  process depth-first numbers in reverse
    evaluate predecessor labels with iterative path compression
    update the semidominator
    link the vertex into the ancestor forest
    resolve the parent's pending bucket
  process numbers forward to finalize immediate dominators
  map dense depth-first numbers back to graph vertices
```

The evaluation path is an explicit reusable vector. The algorithm contains no recursive depth-first
search, recursive path compression, recursive dominator-tree walk, or recursive drop structure.

## Determinism and concurrency

The serial algorithm defines the exact observation. Canonical CSR order fixes queue and edge
scans. A future parallel construction may compute isolated candidates concurrently, but it must:

- preserve caller-provided policy keys;
- merge by canonical quotient-edge and provenance-key order;
- commit flat buffers in deterministic order;
- return byte-equivalent exact output across worker counts; and
- keep cancellation and budget exhaustion distinct from exact completion.

Parallel shared mutation of sidecar fibers or dominator state is not admitted by this contract.
Independent per-root dominance queries can be scheduled concurrently by a consumer because every
query owns its workspace and reads immutable CSR.

## Renaming object

A lawful renaming for structural observations contains bijections for vertices, components, and
canonical edges which commute with endpoints and the SCC quotient. For a single-choice operation,
the object additionally contains the strict total order used by selection, and the renaming
preserves that order.

This makes the selection law precise:

```math
\rho\!\left(\min_{\prec} F\right)
  = \min_{\rho(\prec)} \rho(F).
```

Without the order in the object, the equation is false on the documented two-edge automorphism.

## Error and resource boundary

All imported flat values are validated before indexed access. Exact operations accept an explicit
resource budget where work can depend on an untrusted graph or sidecar. The result variants are
exact, unreachable, invalid, limit exceeded, and cancelled. No partial member array or parent map
is returned as exact.

Versioned serialization, schema identity, stable digests, and cross-process provenance keys remain
in `libvgraph-interop`. The core crate must remain usable without JSON, serde, hashing, or a source
analysis framework.

## Supersession conditions

This architecture can be changed only with machine-checked replacement laws and evidence that:

- CSR-only workloads do not regress;
- arbitrary-renaming claims remain correctly qualified;
- stack safety holds on adversarial depth;
- asymptotic work is no worse for the applicable operation; and
- the replacement preserves exact serial observations.
