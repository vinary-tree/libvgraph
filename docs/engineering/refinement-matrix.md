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

## Evidence handling

Verification commands write transient logs below `target/verification`. Record command lines,
tool versions, result summaries, and SHA-256 hashes in pgmcp. Remove transient binaries and logs
after the evidence record is durable; committed proof, model, test, documentation, and diagram
sources remain the reproducible authority.
