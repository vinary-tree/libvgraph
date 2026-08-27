# Vinary Graph Kernel

Vinary Graph Kernel is the formally specified, deterministic graph substrate for Vinary projects.
This initial repository contains the pre-implementation contract only. Production Rust source is
intentionally absent until the proof, model-checking, exhaustive-oracle, and documentation gates
pass.

The first contract covers canonical compressed sparse row (CSR) graphs, strongly connected
component (SCC) quotients, condensation directed acyclic graphs (DAGs), and dependency wavefronts.
It preserves the semantics already exercised by `libcpg` while remaining independent of code
property graphs, equality graphs, parsers, and weighted automata.

## Start here

- [Graph quotient theory](docs/theory/graph-quotients.md)
- [Formal-first architecture](docs/architecture/formal-first-contract.md)
- [Exhaustive validation method](docs/science/exhaustive-validation.md)
- [Implementation refinement matrix](docs/engineering/refinement-matrix.md)
- [Resource and input safety](docs/security/resource-safety.md)
- [Verification workflow](docs/usage/verification-workflow.md)
- [Formal verification guide](formal/README.md)
- [Diagram catalog](docs/diagrams/README.md)

Run `scripts/verify-formal.sh` before adding production code. Run
`scripts/verify-docs.sh` for every documentation update.

## Status

The formal contract is being implemented under pgmcp task `vco-e2-formal-contracts`. The
production crate task `vco-e2-kernel-implementation` remains dependency-blocked until this
contract is checked and recorded.

## License

Apache-2.0. See [LICENSE](LICENSE).
