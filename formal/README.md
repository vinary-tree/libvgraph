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
- `model/exhaustive_graphs.rs` enumerates all directed graphs through four vertices, validates
  canonical forward/reverse CSR, compares the iterative SCC model with independent transitive
  closure, checks every vertex permutation together with its induced condensation and ranks, and
  constructs flat wave offsets/members with exact work and constant-buffer checks, and runs a
  20,000-vertex small-stack lifecycle.
- `rocq/GraphWitnesses.v` specifies opaque provenance fibers, canonical union, edge-index replay,
  reachability, condensation source-edge fibers, rooted dominance, dominance frontiers,
  transported-order selection, incomplete outcomes, and logical resource bounds. It proves the
  two-witness counterexample which forbids an unqualified natural selector, then proves
  uniqueness and naturality when a strict total-order policy is transported with the witness
  fiber. Every printed acceptance theorem must be closed under the global context.
- `tla/WitnessMachine.tla` models canonical search, iterative parent reconstruction, replay,
  budgets, cancellation, invalid parent state, and exact/unreachable terminal separation.
  `WitnessMachine.cfg` and `WitnessMachineUnreachable.cfg` check reachable and unreachable goals.
- `model/exhaustive_witnesses.rs` enumerates 530 directed graphs, 1,570 rooted
  dominator/frontier cases, and 3,106 lawful renamings. It compares iterative
  Lengauer–Tarjan with a vertex-removal dominance oracle, compares local/up frontiers with their
  predecessor definition, executes every registered mutant/malformed case, and runs the complete
  witness stack on a 20,000-vertex chain in a 256 KiB native-stack thread.
- `verus/flat_wave_refinement.rs` proves rank-fiber totality and disjointness, flat-wave storage
  linearity, exact schedule charging, unsigned 64-bit fit in the graph domain, and the uniform
  phase-complete pipeline bound.
- `verus/witness_refinement.rs` proves flat sidecar storage, union/reachability/frontier charges,
  exact path-replay work, constant replay auxiliary state, near-linear dominator charging,
  unsigned 64-bit graph-domain fit, and constant native-control depth.
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

Run only the new pre-implementation witness layers:

```bash
scripts/verify-formal.sh witness
```

The default command checks the complete core and witness layers. Each layer runs in a transient
systemd user scope with
`MemorySwapMax=0`, one Cargo build job, and an explicit resident-memory ceiling. Rocq, TLA+/TLC,
the exhaustive model, and Verus receive 4 GiB; Kani/CBMC receives 2 GiB. A cap hit is a failed
proof gate and must be addressed by reducing verifier state, not by silently increasing the cap.
Every scope uses a 100% CPU quota and a repository-backed temporary directory. Java and the TLA+
launcher receive the same repository-local temporary path.

The command writes transient evidence below `target/verification`. Record commands, versions,
result summaries, peak memory, and SHA-256 hashes in pgmcp before deleting those files.
