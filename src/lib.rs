//! Deterministic structural graph algorithms for Vinary projects.
//!
//! The crate owns canonical compressed sparse row (CSR) construction,
//! validation, iterative traversals, strongly connected components (SCCs),
//! condensation directed acyclic graphs (DAGs), and dependency wavefronts.
//! It deliberately contains no parser, code-property-graph, e-graph,
//! weighted-automata, or generic fixed-point semantics.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod condensation;
mod control;
mod csr;
mod error;
mod id;
mod radix;
mod scc;
mod traversal;

pub use condensation::{Condensation, WavefrontSchedule};
pub use control::{ExecutionControl, IncompleteReason};
pub use csr::{BuildOptions, CsrGraph, GraphLimits, ReversePolicy};
pub use error::{ComputeError, Direction, Endpoint, GraphError};
pub use id::{ComponentId, DenseId};
pub use scc::{SccComponent, SccDecomposition, SccWorkProfile, SccWorkspace};
