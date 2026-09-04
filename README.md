# libvgraph

libvgraph is the formally specified, deterministic structural-graph substrate for Vinary projects.
It provides a production Rust implementation backed by machine-checked semantics, explicit work
and heap bounds, exhaustive independent oracles, bounded model checking, and documentation
verification.

The first contract covers canonical compressed sparse row (CSR) graphs, strongly connected
component (SCC) quotients, condensation directed acyclic graphs (DAGs), and dependency wavefronts.
It preserves the semantics already exercised by `libcpg` while remaining independent of code
property graphs, equality graphs, parsers, and weighted automata.

The core has no serialization or hashing dependency. Portable snapshots, schema identities,
digests, and provenance sidecars belong to the separately versioned `libvgraph-interop` boundary.

On validated canonical CSR, the required SCC path is iterative, uses strict linear work, retains
all graph-depth state on the heap, and preserves a constant native control depth. Arbitrary stable
labels are canonicalized at a separately named comparison-model boundary so their unavoidable
ordering cost is never conflated with graph-analysis complexity.

## Start here

- [Graph quotient theory](docs/theory/graph-quotients.md)
- [Canonical snapshot laws](docs/theory/canonical-snapshot-laws.md)
- [Formal-first architecture](docs/architecture/formal-first-contract.md)
- [Portable snapshot boundary](docs/architecture/interop-boundary.md)
- [Borrowed CSR refinement contract](docs/architecture/borrowed-csr-refinement.md)
- [Exhaustive validation method](docs/science/exhaustive-validation.md)
- [Snapshot validation method](docs/science/interop-validation.md)
- [Implementation refinement matrix](docs/engineering/refinement-matrix.md)
- [Snapshot refinement matrix](docs/engineering/interop-refinement-matrix.md)
- [Resource and input safety](docs/security/resource-safety.md)
- [Snapshot security and resource safety](docs/security/interop-resource-safety.md)
- [Rust API and usage](docs/usage/rust-api.md)
- [Canonical snapshot wire format](docs/usage/interop-wire-format.md)
- [Performance and deterministic concurrency](docs/engineering/performance-and-concurrency.md)
- [Verification workflow](docs/usage/verification-workflow.md)
- [Formal verification guide](formal/README.md)
- [Diagram catalog](docs/diagrams/README.md)

Run `scripts/verify-formal.sh all` before changing production semantics. The runner places every
heavy proof layer in an explicit no-swap systemd memory scope. Run `scripts/verify-docs.sh` for
every documentation update.

## Status

The graph-kernel contract and implementation are tracked by pgmcp tasks
`vco-e2-formal-contracts`, `vco-e2-kernel-implementation`, and
`vco-e2-kernel-release`. The independent snapshot/digest contract is tracked by
`vco-e2-interop-formal`; its required-red properties intentionally name the separately
owned `libvgraph-interop` package.

## License

Apache-2.0. See [LICENSE](LICENSE).
