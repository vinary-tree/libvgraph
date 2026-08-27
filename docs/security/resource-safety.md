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
- Preallocate from validated counts and enforce caller-visible vertex, edge, work, memory, and time
  budgets.
- Keep input-depth traversal state in heap-owned vectors or queues; never map graph depth to native
  recursion depth.
- Keep canonical-CSR SCC work at exactly $`5|V| + |E|`$ logical events and its auxiliary
  vertex-slot peak at or below $`5|V|`$; reject an implementation whose hidden canonicalization
  changes that bound.
- Use nonrecursive fixed-width canonicalization for dense quotient edges. Charge both
  $`2{,}048`$-bucket scans in every one of the six full radix passes, both candidate scans per
  pass, and final deduplication; do not hide fixed scans behind asymptotic notation. Do not call a
  recursive comparison sort from an input-depth-sensitive public path.
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
