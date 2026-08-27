# libvgraph

libvgraph is the formally specified, deterministic structural-graph substrate for Vinary projects.
It provides a production Rust implementation backed by machine-checked semantics, explicit work
and heap bounds, exhaustive independent oracles, bounded model checking, and documentation
verification.

The first contract covers canonical compressed sparse row (CSR) graphs, strongly connected
component (SCC) quotients, condensation directed acyclic graphs (DAGs), and dependency wavefronts.
It preserves the semantics already exercised by `libcpg` while remaining independent of code
property graphs, equality graphs, parsers, and weighted automata.

On validated canonical CSR, the required SCC path is iterative, uses strict linear work, retains
all graph-depth state on the heap, and preserves a constant native control depth. Arbitrary stable
labels are canonicalized at a separately named comparison-model boundary so their unavoidable
ordering cost is never conflated with graph-analysis complexity.

## Start here

- [Graph quotient theory](docs/theory/graph-quotients.md)
- [Formal-first architecture](docs/architecture/formal-first-contract.md)
- [Exhaustive validation method](docs/science/exhaustive-validation.md)
- [Implementation refinement matrix](docs/engineering/refinement-matrix.md)
- [Resource and input safety](docs/security/resource-safety.md)
- [Rust API and usage](docs/usage/rust-api.md)
- [Performance and deterministic concurrency](docs/engineering/performance-and-concurrency.md)
- [Verification workflow](docs/usage/verification-workflow.md)
- [Formal verification guide](formal/README.md)
- [Diagram catalog](docs/diagrams/README.md)

Run `scripts/verify-formal.sh all` before changing production semantics. The runner places every
heavy proof layer in an explicit no-swap systemd memory scope. Run `scripts/verify-docs.sh` for
every documentation update.

## Status

The strengthened formal contract is tracked by pgmcp task `vco-e2-formal-contracts`. Production
implementation and refinement closeout are tracked by `vco-e2-kernel-implementation`.

## License

Apache-2.0. See [LICENSE](LICENSE).
