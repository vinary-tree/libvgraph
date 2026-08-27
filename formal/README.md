# Formal Verification Guide

The formal artifacts precede production implementation.

- `rocq/GraphQuotient.v` defines the exact SCC quotient contract and proves fiber totality and
  nonemptiness, quotient-edge witness equivalence, condensation acyclicity, bidirectional
  renaming preservation, permutation/duplicate/extensional enumeration invariance, and wavefront
  independence. `Print Assumptions` output must report a closed global context for every
  acceptance theorem.
- `tla/IterativeGraphMachine.tla` models a finite explicit-frame traversal with completion and
  cancellation. TLC checks type, ownership, uniqueness, frame-bound, and completion invariants.
- `model/exhaustive_graphs.rs` enumerates all directed graphs through four vertices, validates
  canonical forward/reverse CSR, compares the iterative SCC model with independent transitive
  closure, checks every vertex permutation together with its induced condensation and ranks, and
  runs a 20,000-vertex small-stack lifecycle.

Run:

```bash
scripts/verify-formal.sh
```

The default command compiles Rocq in a resource-limited user scope, validates TLA+ syntax and the
TLC model, and compiles/runs the exhaustive oracle. It writes transient evidence below
`target/verification`; record hashes and results in pgmcp before deleting that directory.
