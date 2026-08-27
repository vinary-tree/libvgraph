# Implementation Refinement Matrix

## Purpose

A production implementation **refines** the formal contract when every public observation agrees
with the corresponding mathematical, lifecycle, or executable-model observation. Passing unit
tests alone is insufficient unless those tests exercise the mapped obligation below.

| Contract obligation | Production evidence required |
|---|---|
| Stable vertex domain | Duplicate and unordered stable IDs are rejected or canonicalized exactly as documented |
| Forward CSR shape | Offset length, zero origin, monotonicity, terminal count, endpoint range, and slice order tests |
| Reverse CSR exactness | Every forward pair has exactly one reverse pair and vice versa |
| SCC exact kernel | Differential comparison with mutual transitive closure on bounded graphs and `libcpg` parity corpora |
| Fiber totality/disjointness | Every vertex has one in-range component and every component has at least one member |
| Quotient exactness | Every cross-component source edge appears once; every quotient edge has a source witness |
| Condensation acyclicity | Topological traversal visits every component and rejects inconsistent imported data |
| Renaming equivariance | Property tests compare the induced component bijection, quotient edges, and ranks |
| Enumeration invariance | Input edge permutations and duplicates produce identical canonical bytes |
| Stack safety | Deep/wide construction, SCC, quotient, formatting, serialization, clone, and drop on a 256 KiB thread |
| Strict-linear SCC work | Instrumented production counters equal $`5\lvert V\rvert + \lvert E\rvert`$ on canonical CSR and scale proportionally on deep, wide, sparse, dense, and adversarial graphs |
| Linear SCC auxiliary space | Allocation/peak-state evidence stays within the declared $`5\lvert V\rvert`$ vertex-slot model, excluding returned output |
| Constant native control depth | Source and call-graph census finds no recursive edge on input-depth-sensitive paths; small-stack tests cover all public lifecycle operations |
| Linear quotient and waves | Counted work uses the exact piecewise six-pass radix charge and is at most $`8\lvert V\rvert + 16\lvert E\rvert + 24{,}576`$ for SCC, exact quotient construction, and wavefront ranking |
| Cancellation and limits | Exhaustion returns a structured incomplete result and never certifies exact completion |

## Malformed-representation matrix

The validator must cover empty offsets, wrong offset length, nonzero first offset, decreasing
offsets, a terminal offset beyond or before the target count, out-of-range targets, unsorted or
duplicate adjacency entries, mismatched reverse edges, payload-length mismatch, and integer
conversion overflow. Each case must return a stable structured error before indexed traversal.

## Deterministic concurrency condition

Parallel traversal is not admitted merely because individual graph operations are thread-safe.
The serial implementation defines the reference observation. A parallel implementation must use
stable task identities, dependency-valid wavefronts, isolated task state, and ordered commit, then
produce the same canonical result under varied worker counts and randomized completion schedules.

## Algorithm-selection discipline

The canonical dense path uses algorithms whose bounds match the information that must be read or
returned: CSR validation and traversal are linear, Tarjan SCC is linear, fixed-width quotient
canonicalization is linear in the word-RAM model, and rank/wave construction is linear in the
condensation size. Arbitrary `Ord` stable labels use a separately exposed comparison-model path;
its ordering lower bound is not charged to dense graph analysis. Existing libcpg algorithm-choice
evidence is reused. Only new or materially refactored paths receive new performance experiments.

## Evidence handling

Verification commands write transient logs below `target/verification`. Record command lines,
tool versions, result summaries, and SHA-256 hashes in pgmcp. Remove transient binaries and logs
after the evidence record is durable; committed proof, model, test, documentation, and diagram
sources remain the reproducible authority.
