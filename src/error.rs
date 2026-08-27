use core::fmt;

use crate::{ComponentId, DenseId, IncompleteReason};

/// One direction of a CSR representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Successor adjacency.
    Forward,
    /// Predecessor adjacency.
    Reverse,
}

/// Endpoint of an input edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    /// Edge source.
    Source,
    /// Edge target.
    Target,
}

/// Structured construction, validation, and query failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// More vertex inputs were supplied than the configured limit.
    VertexInputLimitExceeded {
        /// Configured input limit.
        limit: u64,
    },
    /// More edge inputs were supplied than the configured limit.
    EdgeInputLimitExceeded {
        /// Configured input limit.
        limit: u64,
    },
    /// The canonical vertex domain cannot be represented by `u32` identifiers.
    VertexDomainOverflow {
        /// Canonical vertex count.
        count: u64,
    },
    /// The canonical edge domain cannot be represented by `u32` offsets.
    EdgeDomainOverflow {
        /// Canonical edge count.
        count: u64,
    },
    /// An input edge references a stable identifier outside the vertex domain.
    UnknownEndpoint {
        /// Zero-based edge input position.
        edge_index: u64,
        /// Invalid endpoint.
        endpoint: Endpoint,
    },
    /// Imported stable identifiers are not strictly increasing.
    StableNodeOrder {
        /// Index of the second invalid pair member.
        index: usize,
    },
    /// CSR offsets do not have exactly `vertex_count + 1` entries.
    OffsetLength {
        /// CSR direction.
        direction: Direction,
        /// Required offset count.
        expected: usize,
        /// Supplied offset count.
        actual: usize,
    },
    /// CSR offsets are empty or do not begin at zero.
    OffsetOrigin {
        /// CSR direction.
        direction: Direction,
        /// Supplied first offset, if present.
        actual: Option<u32>,
    },
    /// Adjacent CSR offsets decrease.
    OffsetOrder {
        /// CSR direction.
        direction: Direction,
        /// Index of the second offset.
        index: usize,
        /// Previous offset.
        previous: u32,
        /// Decreasing offset.
        next: u32,
    },
    /// The final CSR offset differs from the target count.
    OffsetTerminal {
        /// CSR direction.
        direction: Direction,
        /// Required target count.
        expected: usize,
        /// Supplied final offset.
        actual: u32,
    },
    /// A CSR target is outside the dense vertex domain.
    TargetOutOfRange {
        /// CSR direction.
        direction: Direction,
        /// Position in the target array.
        edge_index: usize,
        /// Invalid target.
        target: u32,
        /// Dense vertex count.
        vertex_count: u32,
    },
    /// One adjacency slice is not strictly increasing.
    AdjacencyOrder {
        /// CSR direction.
        direction: Direction,
        /// Dense source whose slice is invalid.
        source: u32,
        /// Position of the second invalid pair member.
        edge_index: usize,
        /// Previous target.
        previous: u32,
        /// Duplicate or decreasing target.
        next: u32,
    },
    /// Forward and reverse directions contain different edge counts.
    ReverseEdgeCount {
        /// Forward edge count.
        forward: usize,
        /// Reverse edge count.
        reverse: usize,
    },
    /// One forward edge has no matching reverse edge.
    ReverseEdgeMissing {
        /// Forward edge source.
        source: DenseId,
        /// Forward edge target.
        target: DenseId,
    },
    /// A dense vertex query is outside the graph domain.
    DenseIdOutOfRange {
        /// Invalid identifier.
        id: DenseId,
        /// Dense vertex count.
        vertex_count: u32,
    },
    /// A component query is outside the condensation domain.
    ComponentIdOutOfRange {
        /// Invalid identifier.
        id: ComponentId,
        /// Component count.
        component_count: u32,
    },
    /// An SCC partition violates a named internal contract.
    InvalidPartition {
        /// Stable explanation suitable for diagnostics and tests.
        reason: &'static str,
    },
    /// A purported condensation graph contains a directed cycle.
    CondensationCycle,
    /// A wavefront rank cannot be represented by `u32`.
    RankOverflow,
}

impl fmt::Display for GraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VertexInputLimitExceeded { limit } => {
                write!(formatter, "vertex input limit exceeded: {limit}")
            }
            Self::EdgeInputLimitExceeded { limit } => {
                write!(formatter, "edge input limit exceeded: {limit}")
            }
            Self::VertexDomainOverflow { count } => {
                write!(formatter, "vertex domain does not fit u32: {count}")
            }
            Self::EdgeDomainOverflow { count } => {
                write!(formatter, "edge domain does not fit u32: {count}")
            }
            Self::UnknownEndpoint {
                edge_index,
                endpoint,
            } => write!(
                formatter,
                "edge {edge_index} has an unknown {endpoint:?} endpoint"
            ),
            Self::StableNodeOrder { index } => {
                write!(formatter, "stable nodes are not strictly ordered at index {index}")
            }
            Self::OffsetLength {
                direction,
                expected,
                actual,
            } => write!(
                formatter,
                "{direction:?} offset length is {actual}, expected {expected}"
            ),
            Self::OffsetOrigin { direction, actual } => write!(
                formatter,
                "{direction:?} offsets must begin at zero, found {actual:?}"
            ),
            Self::OffsetOrder {
                direction,
                index,
                previous,
                next,
            } => write!(
                formatter,
                "{direction:?} offsets decrease at {index}: {previous} then {next}"
            ),
            Self::OffsetTerminal {
                direction,
                expected,
                actual,
            } => write!(
                formatter,
                "{direction:?} terminal offset is {actual}, expected {expected}"
            ),
            Self::TargetOutOfRange {
                direction,
                edge_index,
                target,
                vertex_count,
            } => write!(
                formatter,
                "{direction:?} target {target} at {edge_index} is outside 0..{vertex_count}"
            ),
            Self::AdjacencyOrder {
                direction,
                source,
                edge_index,
                previous,
                next,
            } => write!(
                formatter,
                "{direction:?} adjacency for {source} is not strictly ordered at {edge_index}: {previous} then {next}"
            ),
            Self::ReverseEdgeCount { forward, reverse } => write!(
                formatter,
                "forward/reverse edge counts differ: {forward} versus {reverse}"
            ),
            Self::ReverseEdgeMissing { source, target } => {
                write!(formatter, "reverse CSR is missing edge {target} <- {source}")
            }
            Self::DenseIdOutOfRange { id, vertex_count } => {
                write!(formatter, "dense id {id} is outside 0..{vertex_count}")
            }
            Self::ComponentIdOutOfRange {
                id,
                component_count,
            } => write!(formatter, "component id {id} is outside 0..{component_count}"),
            Self::InvalidPartition { reason } => {
                write!(formatter, "invalid SCC partition: {reason}")
            }
            Self::CondensationCycle => formatter.write_str("condensation graph contains a cycle"),
            Self::RankOverflow => formatter.write_str("wavefront rank does not fit u32"),
        }
    }
}

impl std::error::Error for GraphError {}

/// Failure of a bounded or cancellable graph computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeError {
    /// A graph or identifier failed validation.
    Invalid(GraphError),
    /// The computation stopped without returning a partial result as exact.
    Incomplete(IncompleteReason),
}

impl fmt::Display for ComputeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => error.fmt(formatter),
            Self::Incomplete(reason) => reason.fmt(formatter),
        }
    }
}

impl std::error::Error for ComputeError {}

impl From<GraphError> for ComputeError {
    fn from(error: GraphError) -> Self {
        Self::Invalid(error)
    }
}

impl From<IncompleteReason> for ComputeError {
    fn from(reason: IncompleteReason) -> Self {
        Self::Incomplete(reason)
    }
}
