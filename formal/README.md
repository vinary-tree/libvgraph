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
- `tla/PortableLockfile.tla` models release admission over two representative Cargo lockfiles.
  A successful Git enumeration returns the complete tracked set; a failed enumeration can emit
  a nonempty prefix. Each lockfile can independently contain ambient patch metadata or be
  unreadable. The guard may certify only after successful enumeration, nonempty complete coverage,
  readable lockfiles, and absence of ambient patches. Any changed input or repeated enumeration
  invalidates the earlier certificate; publication requires a current certificate. TLC checks six
  state invariants. Two causal configurations must violate the targeted invariant when the
  enumeration-result check or the read-error check is removed. The Git-completeness assumption
  applies only to a successful `git ls-files`; the concrete guard must reject every nonzero Git
  exit status, even after partial output. This model is not a TOML parser proof: shell and Rust
  mutation properties refine its concrete lockfile observation.
- `model/exhaustive_graphs.rs` enumerates all directed graphs through four vertices, validates
  canonical forward/reverse CSR, compares the iterative SCC model with independent transitive
  closure, checks every vertex permutation together with its induced condensation and ranks, and
  constructs flat wave offsets/members with exact work and constant-buffer checks, and runs a
  20,000-vertex small-stack lifecycle.
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
- `rocq/GraphSnapshot.v` proves the flat word-tape round trip, uniqueness, exact schema
  and profile rejection, complete three-buffer heap/work equations, explicit cursor progress,
  constant native control depth, and tagged digest-preimage separation. Nineteen acceptance
  theorems must report a closed global context.
- `tla/InteropCodecMachine.tla` checks two concurrent requests across eleven input
  classes, cancellation, publication, and resource release. The positive model generates 33,270
  states, reaches 16,900 distinct states at depth 15, and preserves native control depth one. Three
  causal configurations must violate `PublicationSound` when schema, canonicality, or cancellation
  enforcement is removed; a fourth must violate `NativeControlDepthBound` when a native frame is
  grown per read step.
- `tla/ReleaseMachine.tla` checks the external publication boundary over every Boolean trust,
  gate-result, and portable-tool-closure combination and over absent, matching, and mismatched
  registry states. The positive model generates 644 states, reaches 452 distinct states at depth 7,
  and requires protected signer policy, identity with protected default-branch head, successful
  gates backed by a closed portable tool set, a draft containing complete evidence after registry
  verification, an exact formal-source binding, equal package/registry hashes, and at most one
  publication. Nine causal
  configurations each remove exactly one control and must violate its targeted invariant.
- `smt/interop_snapshot.smt2` proves nine unsatisfiable overflow, bound, separation, and
  fail-closed obligations and produces two constructive satisfiable models.
- `model/exhaustive_interop.rs` refines the exact 80-byte header, little-endian payload, returned
  dense-node vector, and complete work/heap bounds over all 531 graphs through three vertices,
  1,593 profile encodings, 9,321 lawful renamings, 180,696 strict prefixes, two golden vectors,
  targeted corruptions, and a 100,000-vertex 64 KiB-stack lifecycle.
- `verus/interop_refinement.rs` proves six Rust-shaped arithmetic, admission, cursor,
  and work refinements. `doc/interop-invariants.tsv` maps 76 obligations bijectively onto
  fifteen required-red production properties.

## Lockfile guard refinement

A *certificate* here means only that the release-admission guard returned success for the
tracked lockfiles. It is not a publication event. The finite TLA+ model permits Git to emit a
nonempty prefix and then fail, and permits the lockfile reader to fail independently. The concrete
guard accepts a lockfile only when a regular non-symlink file produces grep status 1, meaning a
completed search found no `[[patch.unused]]` header. Status 0 means the header was found; every
other status means the observation failed. Git enumeration is consumed only after its exit status
is 0, and the successful result must contain at least one tracked lockfile.

| Model invariant | Concrete refinement and acceptance evidence |
| --- | --- |
| `TypeOK` | TLC checks each finite state component; the Rust property test varies both external exit statuses over 0–3 and the emitted-path Boolean. |
| `CertifiedHasCompleteEnumeration` | The checked Git command, nonempty-path test, and property cases reject a nonzero status even after `Cargo.lock` has been emitted. |
| `CertifiedHasReadableLockfiles` | The tri-state grep check and property cases accept status 1 only; a status-2 read error has a named regression test. |
| `CertifiedHasNoAmbientPatch` | A grep match rejects admission; the guard also constructs and rejects an ambient-patch mutant for every tracked lockfile. |
| `PublishedOnlyAfterCertificate` | The `PortableLockfile.tla` publication action requires its certificate. The shell test proves guard admission only; it does not verify a concrete publisher invokes the guard. `ReleaseMachine.tla` independently checks generic publication gates. |
| `PublishedHasPortableSource` | The model checks the source conjunction at modeled publication. The 32-case bounded matrix and randomized Rust property test check only the concrete guard-admission conjunction. |

Run the concrete refinement tests with:

```bash
cargo test --locked --offline --test lockfile_guard_properties
```

The fake Git and grep executables inject failures without changing tracked files. This fault
injection tests command-status handling; it does not establish the
semantic completeness of a successful Git response or prove TOML parsing. Those remain explicit
boundary assumptions, not hidden conclusions of the state-machine proof.

Run:

```bash
scripts/verify-formal.sh
```

Run only the snapshot/digest contract:

```bash
scripts/verify-formal.sh interop
```

The interop target succeeds only when all positive layers pass, all four codec and nine release
causal mutants fail on their intended invariants, and the required-red suite fails solely at the
unresolved `libvgraph_interop` import.

The default command checks all six layers. Each layer runs in a transient systemd user scope with
`MemorySwapMax=0`, one Cargo build job, and an explicit resident-memory ceiling. Rocq, TLA+/TLC,
the exhaustive model, and Verus receive 4 GiB; Kani/CBMC receives 2 GiB. A cap hit is a failed
proof gate and must be addressed by reducing verifier state, not by silently increasing the cap.
Every scope uses a 100% CPU quota and a repository-backed temporary directory. Java and the TLA+
launcher receive the same repository-local temporary path.

The command writes transient evidence below `target/verification`. Record commands, versions,
result summaries, peak memory, and SHA-256 hashes in pgmcp before deleting those files.
