# Formal Verification Guide

The formal artifacts precede production implementation.

- `rocq/GraphQuotient.v` defines the exact SCC quotient contract and proves fiber totality and
  nonemptiness, quotient-edge witness equivalence, condensation acyclicity, bidirectional
  renaming preservation, permutation/duplicate/extensional enumeration invariance, and wavefront
  independence. It additionally proves exact $`5|V| + |E|`$ work for a complete canonical SCC
  trace, an auxiliary-heap bound of $`5|V| + |C| \le 6|V|`$ logical entries, constant native
  control depth, the exact
  piecewise charge for radix workspace preparation and six-pass 11-bit canonicalization, and a
  phase-complete partition/condensation/wavefront upper bound of
  $`27|V| + 20|E| + 26{,}628`$. It also proves a complete reusable-workspace bound of
  $`9|V| + 2|E| + 2{,}048`$ slots, excluding returned values, plus a constant two-buffer flat-wave
  output with exactly $`|C| + |W| + 1`$ element slots. `Print Assumptions` output must report a
  closed global context for every acceptance theorem.
- `tla/IterativeGraphMachine.tla` models a finite explicit-frame traversal with completion and
  cancellation. TLC checks type, ownership, uniqueness, frame bounds, exact discovery/edge/frame
  work accounting, linear work, and exact completed-state work.
- `rocq/BorrowedCsrRefinement.v` specifies a concrete caller-owned raw-`u32` forward-CSR
  observation. It proves header, row, target, and strict-order admission; exact borrowed/owned
  edge equivalence; zero input-clone slots; transactional publication; total and disjoint SCC
  fibers; least-member component numbering; exact quotient edges; singleton self-cycle
  equivalence; fused linear validation work; and preservation of the released pipeline workspace
  bound.
- `tla/BorrowedCsrMachine.tla` models header admission, per-row bounds, per-edge target
  range/order checks, indexing, completion, rejection, and cancellation. TLC checks that every
  index is preceded by both target checks, every edge belongs to an admitted row, publication is
  complete and canonical, rejection/cancellation publishes nothing, and completed validation
  performs exactly $`1+|V|+|E|`$ events. Six required-red configurations must violate their
  designated invariant when a header, offset, target, order, duplicate, or publication check is
  removed.
- `model/exhaustive_graphs.rs` enumerates all directed graphs through four vertices, validates
  canonical forward/reverse CSR, compares the iterative SCC model with independent transitive
  closure, checks every vertex permutation together with its induced condensation and ranks, and
  constructs flat wave offsets/members with exact work and constant-buffer checks, and runs a
  20,000-vertex small-stack lifecycle. It also checks 48,776 bounded raw borrowed
  representations, every cancellation point through three vertices, borrowed/owned SCC and
  condensation equality, input-buffer identity, zero clone slots, strict fused-validation work,
  and all malformed-input classes.
- `z3/BorrowedCsrRefinement.smt2` discharges eight arithmetic, safety, refinement, and
  transactionality obligations. `z3/BorrowedCsrRequiredRed.smt2` must exhibit six satisfiable
  counterexamples after mandatory checks are removed.
- `invariants/borrowed-csr.json` is the machine-readable invariant-to-proof/model/property ledger
  consumed by the verification script and the subsequent production implementation task.
- `verus/flat_wave_refinement.rs` proves rank-fiber totality and disjointness, flat-wave storage
  linearity, exact schedule charging, unsigned 64-bit fit in the graph domain, and the uniform
  phase-complete pipeline bound.
- The `#[cfg(kani)]` harnesses in `src/radix.rs`, `src/control.rs`, and `src/condensation.rs`
  execute concrete production functions symbolically. They prove pair encoding round trips,
  radix-work arithmetic cannot overflow in the graph domain, work admission is fail-atomic, and
  the flat-wave buffers are sorted exact rank fibers for the bounded harness domain.
- `scripts/check-core-boundary.sh` checks the Cargo metadata and source surface to ensure the
  payload-free kernel has no serialization dependency or feature. Portable encoding, schema
  identity, hashing, and provenance belong to `libvgraph-interop`.

Run:

```bash
scripts/verify-formal.sh
```

The default command checks eight layers. Each layer runs in a transient systemd user scope with
`MemorySwapMax=0`, one Cargo build job, and an explicit resident-memory ceiling. Rocq, TLA+/TLC,
the exhaustive model, Verus, and Kani/CBMC receive at most 2 GiB; Z3 and ledger validation receive
at most 512 MiB. A cap hit is a failed
proof gate and must be addressed by reducing verifier state, not by silently increasing the cap.
Every scope uses a 100% CPU quota and a repository-backed temporary directory. Java and the TLA+
launcher receive the same repository-local temporary path.

The command writes transient evidence below `target/verification`. Record commands, versions,
result summaries, peak memory, and SHA-256 hashes in pgmcp before deleting those files.
