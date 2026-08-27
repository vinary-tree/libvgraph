use core::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::control::Unbounded;
use crate::error::{Direction, Endpoint, GraphError};
use crate::id::{DenseId, IndexId};
use crate::radix::{encode_pair, pair_source, pair_target, RadixWorkspace};

/// Caller-visible construction limits.
///
/// Limits count supplied iterator items before node or edge deduplication, so
/// adversarial duplicate inputs cannot bypass work and allocation controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphLimits {
    /// Maximum supplied vertex items.
    pub max_vertex_inputs: u64,
    /// Maximum supplied edge items.
    pub max_edge_inputs: u64,
}

impl Default for GraphLimits {
    fn default() -> Self {
        Self {
            max_vertex_inputs: u64::from(u32::MAX),
            max_edge_inputs: u64::from(u32::MAX),
        }
    }
}

/// Whether canonical construction materializes predecessor CSR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReversePolicy {
    /// Build predecessor CSR in strict linear work from canonical forward CSR.
    #[default]
    Build,
    /// Retain only successor CSR.
    Omit,
}

/// Canonical graph-construction options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildOptions {
    /// Input limits.
    pub limits: GraphLimits,
    /// Reverse-adjacency policy.
    pub reverse: ReversePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Csr<I> {
    offsets: Vec<u32>,
    targets: Vec<I>,
}

impl<I: IndexId> Csr<I> {
    pub(crate) fn from_sorted_keys(vertex_count: u32, keys: &[u64]) -> Result<Self, GraphError> {
        let edge_count = u32::try_from(keys.len()).map_err(|_| GraphError::EdgeDomainOverflow {
            count: keys.len() as u64,
        })?;
        let offset_count =
            (vertex_count as usize)
                .checked_add(1)
                .ok_or(GraphError::VertexDomainOverflow {
                    count: u64::from(vertex_count),
                })?;
        let mut offsets = vec![0u32; offset_count];
        let mut targets = Vec::with_capacity(edge_count as usize);
        let mut previous = None;
        for &key in keys {
            if previous.is_some_and(|prior| prior >= key) {
                return Err(GraphError::InvalidPartition {
                    reason: "encoded edges must be strictly ordered",
                });
            }
            previous = Some(key);
            let source = pair_source(key);
            let target = pair_target(key);
            if source >= vertex_count || target >= vertex_count {
                return Err(GraphError::InvalidPartition {
                    reason: "an encoded edge lies outside its vertex domain",
                });
            }
            offsets[source as usize + 1] += 1;
            targets.push(I::from_raw(target));
        }
        for index in 1..offsets.len() {
            offsets[index] = offsets[index].checked_add(offsets[index - 1]).ok_or(
                GraphError::EdgeDomainOverflow {
                    count: u64::from(edge_count),
                },
            )?;
        }
        Ok(Self { offsets, targets })
    }

    pub(crate) fn from_parts(offsets: Vec<u32>, targets: Vec<I>) -> Self {
        Self { offsets, targets }
    }

    pub(crate) fn transpose(&self, vertex_count: u32) -> Result<Self, GraphError> {
        let vertex_count_usize = vertex_count as usize;
        let mut counts = vec![0u32; vertex_count_usize];
        for target in &self.targets {
            let slot = &mut counts[target.get() as usize];
            *slot = slot.checked_add(1).ok_or(GraphError::EdgeDomainOverflow {
                count: self.targets.len() as u64,
            })?;
        }
        let mut offsets = Vec::with_capacity(vertex_count_usize + 1);
        offsets.push(0u32);
        for count in counts {
            let next = offsets
                .last()
                .copied()
                .unwrap_or_default()
                .checked_add(count)
                .ok_or(GraphError::EdgeDomainOverflow {
                    count: self.targets.len() as u64,
                })?;
            offsets.push(next);
        }
        let mut cursors: Vec<usize> = offsets[..vertex_count_usize]
            .iter()
            .map(|offset| *offset as usize)
            .collect();
        let mut targets = vec![I::from_raw(0); self.targets.len()];
        for source in 0..vertex_count {
            for &target in self.slice(I::from_raw(source)) {
                let cursor = &mut cursors[target.get() as usize];
                targets[*cursor] = I::from_raw(source);
                *cursor += 1;
            }
        }
        Ok(Self { offsets, targets })
    }

    pub(crate) fn validate(
        &self,
        vertex_count: u32,
        direction: Direction,
    ) -> Result<(), GraphError> {
        let expected_offsets =
            (vertex_count as usize)
                .checked_add(1)
                .ok_or(GraphError::VertexDomainOverflow {
                    count: u64::from(vertex_count),
                })?;
        if self.offsets.len() != expected_offsets {
            return Err(GraphError::OffsetLength {
                direction,
                expected: expected_offsets,
                actual: self.offsets.len(),
            });
        }
        if self.offsets.first().copied() != Some(0) {
            return Err(GraphError::OffsetOrigin {
                direction,
                actual: self.offsets.first().copied(),
            });
        }
        for (index, pair) in self.offsets.windows(2).enumerate() {
            if pair[0] > pair[1] {
                return Err(GraphError::OffsetOrder {
                    direction,
                    index: index + 1,
                    previous: pair[0],
                    next: pair[1],
                });
            }
        }
        let terminal = self.offsets.last().copied().unwrap_or_default();
        if terminal as usize != self.targets.len() {
            return Err(GraphError::OffsetTerminal {
                direction,
                expected: self.targets.len(),
                actual: terminal,
            });
        }
        for (edge_index, target) in self.targets.iter().copied().enumerate() {
            if target.get() >= vertex_count {
                return Err(GraphError::TargetOutOfRange {
                    direction,
                    edge_index,
                    target: target.get(),
                    vertex_count,
                });
            }
        }
        for source in 0..vertex_count {
            let start = self.offsets[source as usize] as usize;
            let end = self.offsets[source as usize + 1] as usize;
            for (relative, pair) in self.targets[start..end].windows(2).enumerate() {
                if pair[0] >= pair[1] {
                    return Err(GraphError::AdjacencyOrder {
                        direction,
                        source,
                        edge_index: start + relative + 1,
                        previous: pair[0].get(),
                        next: pair[1].get(),
                    });
                }
            }
        }
        Ok(())
    }

    pub(crate) fn slice(&self, source: I) -> &[I] {
        let index = source.get() as usize;
        let start = self.offsets[index] as usize;
        let end = self.offsets[index + 1] as usize;
        &self.targets[start..end]
    }

    pub(crate) fn contains(&self, source: I, target: I) -> bool {
        self.slice(source).binary_search(&target).is_ok()
    }

    pub(crate) fn offsets(&self) -> &[u32] {
        &self.offsets
    }

    pub(crate) fn targets(&self) -> &[I] {
        &self.targets
    }

    pub(crate) fn edge_count(&self) -> usize {
        self.targets.len()
    }
}

/// Immutable canonical graph with stable identifiers and dense CSR storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrGraph<K> {
    nodes: Vec<K>,
    forward: Csr<DenseId>,
    reverse: Option<Csr<DenseId>>,
}

impl<K: Ord> CsrGraph<K> {
    /// Builds canonical forward and reverse CSR with default limits.
    ///
    /// Arbitrary stable identifiers require comparison ordering. This boundary
    /// uses one contiguous binary heap and binary searches rather than
    /// allocation-heavy ordered trees. Dense graph analysis after construction
    /// remains strict linear work.
    ///
    /// # Errors
    ///
    /// Returns a structured [`GraphError`] when an input limit or identifier
    /// domain is exceeded, or an edge endpoint is absent from `nodes`.
    pub fn from_edges(
        nodes: impl IntoIterator<Item = K>,
        edges: impl IntoIterator<Item = (K, K)>,
    ) -> Result<Self, GraphError> {
        Self::from_edges_with_options(nodes, edges, BuildOptions::default())
    }

    /// Builds canonical CSR under explicit limits and reverse policy.
    ///
    /// # Errors
    ///
    /// Returns a structured [`GraphError`] when `options` reject an input,
    /// identifiers exceed the dense domain, or an edge endpoint is unknown.
    pub fn from_edges_with_options(
        nodes: impl IntoIterator<Item = K>,
        edges: impl IntoIterator<Item = (K, K)>,
        options: BuildOptions,
    ) -> Result<Self, GraphError> {
        let nodes = canonical_nodes(nodes, options.limits.max_vertex_inputs)?;
        let vertex_count =
            u32::try_from(nodes.len()).map_err(|_| GraphError::VertexDomainOverflow {
                count: nodes.len() as u64,
            })?;
        let iterator = edges.into_iter();
        let initial_capacity =
            bounded_capacity(iterator.size_hint().0, options.limits.max_edge_inputs);
        let mut keys = Vec::with_capacity(initial_capacity);
        for (raw_edge_index, (source, target)) in iterator.enumerate() {
            let edge_index = u64::try_from(raw_edge_index)
                .map_err(|_| GraphError::EdgeDomainOverflow { count: u64::MAX })?;
            if edge_index >= options.limits.max_edge_inputs {
                return Err(GraphError::EdgeInputLimitExceeded {
                    limit: options.limits.max_edge_inputs,
                });
            }
            let source = nodes
                .binary_search(&source)
                .map_err(|_| GraphError::UnknownEndpoint {
                    edge_index,
                    endpoint: Endpoint::Source,
                })?;
            let target = nodes
                .binary_search(&target)
                .map_err(|_| GraphError::UnknownEndpoint {
                    edge_index,
                    endpoint: Endpoint::Target,
                })?;
            let source = u32::try_from(source).map_err(|_| GraphError::VertexDomainOverflow {
                count: nodes.len() as u64,
            })?;
            let target = u32::try_from(target).map_err(|_| GraphError::VertexDomainOverflow {
                count: nodes.len() as u64,
            })?;
            keys.push(encode_pair(source, target));
        }
        canonicalize_keys(&mut keys)?;
        Self::from_canonical_parts(nodes, vertex_count, &keys, options.reverse)
    }

    /// Validates and imports explicit CSR parts in strict linear work.
    ///
    /// Stable nodes and adjacency slices must already be strictly ordered and
    /// unique. Malformed representations are rejected rather than normalized.
    ///
    /// # Errors
    ///
    /// Returns a structured [`GraphError`] for the first malformed stable-node,
    /// offset, endpoint, ordering, or reverse-transpose invariant.
    pub fn try_from_parts(
        nodes: Vec<K>,
        forward_offsets: Vec<u32>,
        forward_targets: Vec<DenseId>,
        reverse: Option<(Vec<u32>, Vec<DenseId>)>,
    ) -> Result<Self, GraphError> {
        let graph = Self {
            nodes,
            forward: Csr::from_parts(forward_offsets, forward_targets),
            reverse: reverse.map(|(offsets, targets)| Csr::from_parts(offsets, targets)),
        };
        graph.validate()?;
        Ok(graph)
    }

    /// Validates stable-node, CSR-shape, ordering, endpoint, and transpose
    /// invariants.
    ///
    /// # Errors
    ///
    /// Returns a structured [`GraphError`] for the first violated invariant.
    pub fn validate(&self) -> Result<(), GraphError> {
        for (index, pair) in self.nodes.windows(2).enumerate() {
            if pair[0] >= pair[1] {
                return Err(GraphError::StableNodeOrder { index: index + 1 });
            }
        }
        let vertex_count =
            u32::try_from(self.nodes.len()).map_err(|_| GraphError::VertexDomainOverflow {
                count: self.nodes.len() as u64,
            })?;
        self.forward.validate(vertex_count, Direction::Forward)?;
        if let Some(reverse) = &self.reverse {
            reverse.validate(vertex_count, Direction::Reverse)?;
            if self.forward.edge_count() != reverse.edge_count() {
                return Err(GraphError::ReverseEdgeCount {
                    forward: self.forward.edge_count(),
                    reverse: reverse.edge_count(),
                });
            }
            let expected_reverse = self.forward.transpose(vertex_count)?;
            if expected_reverse != *reverse {
                for target in 0..vertex_count {
                    let target = DenseId::from_raw(target);
                    let expected = expected_reverse.slice(target);
                    let actual = reverse.slice(target);
                    let mut expected_index = 0usize;
                    let mut actual_index = 0usize;
                    while expected_index < expected.len() && actual_index < actual.len() {
                        match expected[expected_index].cmp(&actual[actual_index]) {
                            core::cmp::Ordering::Less => {
                                return Err(GraphError::ReverseEdgeMissing {
                                    source: expected[expected_index],
                                    target,
                                });
                            }
                            core::cmp::Ordering::Equal => {
                                expected_index += 1;
                                actual_index += 1;
                            }
                            core::cmp::Ordering::Greater => actual_index += 1,
                        }
                    }
                    if expected_index < expected.len() {
                        return Err(GraphError::ReverseEdgeMissing {
                            source: expected[expected_index],
                            target,
                        });
                    }
                }
                return Err(GraphError::InvalidPartition {
                    reason: "reverse CSR contains an edge absent from forward CSR",
                });
            }
        }
        Ok(())
    }

    /// Returns the canonical stable vertex domain.
    #[must_use]
    pub fn nodes(&self) -> &[K] {
        &self.nodes
    }

    /// Returns the number of canonical vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of canonical directed edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.forward.edge_count()
    }

    /// Returns whether predecessor CSR is present.
    #[must_use]
    pub const fn has_reverse(&self) -> bool {
        self.reverse.is_some()
    }

    /// Maps a stable identifier to its dense identifier.
    #[must_use]
    pub fn dense_id(&self, stable: &K) -> Option<DenseId> {
        self.nodes
            .binary_search(stable)
            .ok()
            .and_then(|dense| u32::try_from(dense).ok())
            .map(DenseId::from_raw)
    }

    /// Maps a valid dense identifier back to its stable identifier.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::DenseIdOutOfRange`] when `dense` is outside the
    /// graph's dense vertex domain.
    pub fn stable_id(&self, dense: DenseId) -> Result<&K, GraphError> {
        self.nodes
            .get(dense.index())
            .ok_or(GraphError::DenseIdOutOfRange {
                id: dense,
                vertex_count: self.vertex_count_u32(),
            })
    }

    /// Returns sorted successors for a valid dense identifier.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::DenseIdOutOfRange`] when `source` is outside the
    /// graph's dense vertex domain.
    pub fn successors(&self, source: DenseId) -> Result<&[DenseId], GraphError> {
        self.validate_dense(source)?;
        Ok(self.forward.slice(source))
    }

    /// Returns sorted predecessors when reverse CSR is present.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::DenseIdOutOfRange`] when `target` is outside the
    /// graph's dense vertex domain.
    pub fn predecessors(&self, target: DenseId) -> Result<Option<&[DenseId]>, GraphError> {
        self.validate_dense(target)?;
        Ok(self.reverse.as_ref().map(|reverse| reverse.slice(target)))
    }

    /// Returns forward CSR offsets.
    #[must_use]
    pub fn forward_offsets(&self) -> &[u32] {
        self.forward.offsets()
    }

    /// Returns forward CSR targets.
    #[must_use]
    pub fn forward_targets(&self) -> &[DenseId] {
        self.forward.targets()
    }

    /// Returns reverse CSR offsets when present.
    #[must_use]
    pub fn reverse_offsets(&self) -> Option<&[u32]> {
        self.reverse.as_ref().map(Csr::offsets)
    }

    /// Returns reverse CSR targets when present.
    #[must_use]
    pub fn reverse_targets(&self) -> Option<&[DenseId]> {
        self.reverse.as_ref().map(Csr::targets)
    }

    /// Iterates edges in canonical source/target order.
    pub fn edges(&self) -> impl Iterator<Item = (DenseId, DenseId)> + '_ {
        (0..self.vertex_count_u32()).flat_map(move |source| {
            let source = DenseId::from_raw(source);
            self.forward
                .slice(source)
                .iter()
                .copied()
                .map(move |target| (source, target))
        })
    }

    /// Materializes reverse CSR in strict linear work if it was omitted.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::EdgeDomainOverflow`] if transpose offsets cannot
    /// represent the canonical edge domain.
    pub fn with_reverse(mut self) -> Result<Self, GraphError> {
        if self.reverse.is_none() {
            self.reverse = Some(self.forward.transpose(self.vertex_count_u32())?);
        }
        Ok(self)
    }

    pub(crate) fn successors_unchecked(&self, source: DenseId) -> &[DenseId] {
        self.forward.slice(source)
    }

    pub(crate) fn contains_edge(&self, source: DenseId, target: DenseId) -> bool {
        self.forward.contains(source, target)
    }

    pub(crate) fn vertex_count_u32(&self) -> u32 {
        u32::try_from(self.nodes.len()).unwrap_or(u32::MAX)
    }

    fn from_canonical_parts(
        nodes: Vec<K>,
        vertex_count: u32,
        keys: &[u64],
        reverse_policy: ReversePolicy,
    ) -> Result<Self, GraphError> {
        let forward = Csr::from_sorted_keys(vertex_count, keys)?;
        let reverse = match reverse_policy {
            ReversePolicy::Build => Some(forward.transpose(vertex_count)?),
            ReversePolicy::Omit => None,
        };
        Ok(Self {
            nodes,
            forward,
            reverse,
        })
    }

    fn validate_dense(&self, id: DenseId) -> Result<(), GraphError> {
        if id.get() >= self.vertex_count_u32() {
            return Err(GraphError::DenseIdOutOfRange {
                id,
                vertex_count: self.vertex_count_u32(),
            });
        }
        Ok(())
    }
}

impl CsrGraph<DenseId> {
    /// Builds a graph whose stable and dense domains are both zero based.
    ///
    /// Endpoint validation, fixed-width radix canonicalization, CSR assembly,
    /// and optional transpose construction are nonrecursive and linear in the
    /// word-RAM model.
    ///
    /// # Errors
    ///
    /// Returns a structured [`GraphError`] when an input limit or identifier
    /// domain is exceeded or an endpoint lies outside `0..vertex_count`.
    pub fn from_dense_edges(
        vertex_count: u32,
        edges: impl IntoIterator<Item = (DenseId, DenseId)>,
    ) -> Result<Self, GraphError> {
        Self::from_dense_edges_with_options(vertex_count, edges, BuildOptions::default())
    }

    /// Builds a dense-identity graph under explicit limits and reverse policy.
    ///
    /// # Errors
    ///
    /// Returns a structured [`GraphError`] when `options` reject an input or an
    /// endpoint lies outside `0..vertex_count`.
    pub fn from_dense_edges_with_options(
        vertex_count: u32,
        edges: impl IntoIterator<Item = (DenseId, DenseId)>,
        options: BuildOptions,
    ) -> Result<Self, GraphError> {
        if u64::from(vertex_count) > options.limits.max_vertex_inputs {
            return Err(GraphError::VertexInputLimitExceeded {
                limit: options.limits.max_vertex_inputs,
            });
        }
        let iterator = edges.into_iter();
        let mut keys = Vec::with_capacity(bounded_capacity(
            iterator.size_hint().0,
            options.limits.max_edge_inputs,
        ));
        for (raw_edge_index, (source, target)) in iterator.enumerate() {
            let edge_index = u64::try_from(raw_edge_index)
                .map_err(|_| GraphError::EdgeDomainOverflow { count: u64::MAX })?;
            if edge_index >= options.limits.max_edge_inputs {
                return Err(GraphError::EdgeInputLimitExceeded {
                    limit: options.limits.max_edge_inputs,
                });
            }
            if source.get() >= vertex_count {
                return Err(GraphError::UnknownEndpoint {
                    edge_index,
                    endpoint: Endpoint::Source,
                });
            }
            if target.get() >= vertex_count {
                return Err(GraphError::UnknownEndpoint {
                    edge_index,
                    endpoint: Endpoint::Target,
                });
            }
            keys.push(encode_pair(source.get(), target.get()));
        }
        canonicalize_keys(&mut keys)?;
        let nodes = (0..vertex_count).map(DenseId::from_raw).collect();
        Self::from_canonical_parts(nodes, vertex_count, &keys, options.reverse)
    }
}

fn canonical_nodes<K: Ord>(
    nodes: impl IntoIterator<Item = K>,
    limit: u64,
) -> Result<Vec<K>, GraphError> {
    let iterator = nodes.into_iter();
    let mut heap = BinaryHeap::with_capacity(bounded_capacity(iterator.size_hint().0, limit));
    for (raw_input, node) in iterator.enumerate() {
        let input = u64::try_from(raw_input)
            .map_err(|_| GraphError::VertexDomainOverflow { count: u64::MAX })?;
        if input >= limit {
            return Err(GraphError::VertexInputLimitExceeded { limit });
        }
        heap.push(Reverse(node));
    }
    let mut ordered = Vec::with_capacity(heap.len());
    while let Some(Reverse(node)) = heap.pop() {
        if ordered.last().is_none_or(|previous| previous != &node) {
            ordered.push(node);
        }
    }
    Ok(ordered)
}

fn bounded_capacity(hint: usize, limit: u64) -> usize {
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    hint.min(limit).min(u32::MAX as usize)
}

fn canonicalize_keys(keys: &mut Vec<u64>) -> Result<(), GraphError> {
    let mut workspace = RadixWorkspace::default();
    workspace
        .sort_dedup(keys, &mut Unbounded)
        .map_err(|_| GraphError::InvalidPartition {
            reason: "an unbounded radix canonicalization stopped incomplete",
        })
}

#[cfg(feature = "serde")]
mod serde_impl {
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

    use super::CsrGraph;
    use crate::DenseId;

    const CSR_GRAPH_FORMAT_VERSION: u32 = 1;

    #[derive(Serialize)]
    struct CsrGraphRef<'a, K> {
        format_version: u32,
        nodes: &'a [K],
        forward_offsets: &'a [u32],
        forward_targets: &'a [DenseId],
        reverse_offsets: Option<&'a [u32]>,
        reverse_targets: Option<&'a [DenseId]>,
    }

    #[derive(Deserialize)]
    struct CsrGraphOwned<K> {
        format_version: u32,
        nodes: Vec<K>,
        forward_offsets: Vec<u32>,
        forward_targets: Vec<DenseId>,
        reverse_offsets: Option<Vec<u32>>,
        reverse_targets: Option<Vec<DenseId>>,
    }

    impl<K: Serialize + Ord> Serialize for CsrGraph<K> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            CsrGraphRef {
                format_version: CSR_GRAPH_FORMAT_VERSION,
                nodes: self.nodes(),
                forward_offsets: self.forward_offsets(),
                forward_targets: self.forward_targets(),
                reverse_offsets: self.reverse_offsets(),
                reverse_targets: self.reverse_targets(),
            }
            .serialize(serializer)
        }
    }

    impl<'de, K: Deserialize<'de> + Ord> Deserialize<'de> for CsrGraph<K> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let wire = CsrGraphOwned::<K>::deserialize(deserializer)?;
            if wire.format_version != CSR_GRAPH_FORMAT_VERSION {
                return Err(de::Error::custom(format_args!(
                    "unsupported libvgraph CSR format version {}, expected {}",
                    wire.format_version, CSR_GRAPH_FORMAT_VERSION
                )));
            }
            let reverse = match (wire.reverse_offsets, wire.reverse_targets) {
                (Some(offsets), Some(targets)) => Some((offsets, targets)),
                (None, None) => None,
                _ => {
                    return Err(de::Error::custom(
                        "reverse offsets and targets must either both be present or both be absent",
                    ));
                }
            };
            CsrGraph::try_from_parts(
                wire.nodes,
                wire.forward_offsets,
                wire.forward_targets,
                reverse,
            )
            .map_err(de::Error::custom)
        }
    }
}
