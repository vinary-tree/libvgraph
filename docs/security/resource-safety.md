# Resource and Input Safety

## Threat model

The graph kernel may receive adversarial vertices, edges, serialized CSR fields, resource limits,
or cancellation timing. Inputs can attempt integer overflow, out-of-range indexing, allocation
amplification, duplicate-edge amplification, native-stack exhaustion, or false completion.

## Required defenses

- Convert counts and offsets with checked conversions before allocation or indexing.
- Reject an input whose declared vertex or edge domain cannot fit the public identifier type.
- Validate all CSR fields before any unchecked indexed traversal.
- Deduplicate edges before allocating downstream per-edge state.
- Preallocate from validated counts. Enforce caller-visible vertex and edge input limits, derive
  allocation bounds from those limits, and enforce deterministic logical-work and cancellation
  controls during analysis.
- Keep input-depth traversal state in heap-owned vectors or queues; never map graph depth to native
  recursion depth.
- Keep canonical-CSR SCC work at exactly $`5|V| + |E|`$ logical events and its auxiliary
  vertex-slot peak at or below $`5|V|`$; reject an implementation whose hidden canonicalization
  changes that bound.
- Use nonrecursive fixed-width canonicalization for dense quotient edges. Charge scratch and
  bucket initialization, both $`2{,}048`$-bucket scans in every one of the six full radix passes,
  both candidate scans per pass, and final deduplication. Also charge flat-fiber materialization,
  paired condensation CSR construction, and flat wave offset/member construction; do not hide
  representation work behind asymptotic notation. Do not call a recursive comparison sort from an
  input-depth-sensitive public path.
- Keep phase-complete charged work at or below $`27|V| + 20|E| + 26{,}628`$ and reusable
  temporary storage at or below $`9|V| + 2|E| + 2{,}048`$ slots, excluding returned values.
- Treat cancellation and cap exhaustion as incomplete outcomes. They cannot certify acyclicity,
  reachability absence, or exact completion.
- Keep public ordering deterministic so attacker-controlled insertion order cannot perturb caches,
  reports, or evidence identities.
- Avoid hidden global interning or unbounded retained state.

The Rocq cost model proves parameterized linear work and heap bounds. The TLA+ lifecycle model
establishes exact discovery/edge/frame accounting and an explicit-frame bound for finite vertex
sets. The exhaustive model exercises every graph with at most four vertices and a 20,000-vertex
chain on a 256 KiB thread. Production acceptance raises lifecycle stress where feasible and adds
malformed CSR, fuzzing, Kani, Verus, and sanitizer evidence.

## Serialized CSR boundary

The optional `serde` feature uses an explicit format-version field. Version 1 stores stable nodes,
forward offsets and targets, and either both reverse arrays or neither. Deserialization rejects an
unknown version and a half-present reverse representation. It then calls `CsrGraph::try_from_parts`,
which validates node ordering, offset shape and monotonicity, target ranges, strictly ordered
adjacency, and exact reverse transposition before returning a traversable graph.

Serde support does not deserialize SCC decompositions or schedules. Those values are recomputed
from the validated graph so stale or forged derived evidence cannot cross the trust boundary.
