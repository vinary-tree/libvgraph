use crate::control::{Unbounded, WorkControl};
use crate::radix::{
    encode_pair, logical_work as radix_logical_work, RadixWorkspace, RADIX_BUCKET_COUNT,
};
use crate::{
    ComponentId, ComputeError, Condensation, CsrGraph, DenseId, ExecutionControl, GraphError,
    WavefrontSchedule,
};

const UNVISITED: u32 = u32::MAX;
const UNASSIGNED: u32 = u32::MAX;

#[derive(Debug, Clone, Copy)]
struct DfsFrame {
    vertex: DenseId,
    next_successor: usize,
}

#[derive(Debug, Clone, Copy)]
struct TarjanSummary {
    component_count: u32,
    peak_active_slots: usize,
    peak_frame_slots: usize,
}

struct PartitionParts {
    components: Vec<SccComponent>,
    component_of: Vec<ComponentId>,
    members: Vec<DenseId>,
}

struct QuotientParts {
    condensation: Condensation,
    candidate_count: u64,
    quotient_edge_count: u64,
}

/// Metadata for one maximal strongly connected component.
///
/// Members live in one flat allocation owned by [`SccDecomposition`]. Use
/// [`SccDecomposition::fiber`] to obtain the sorted member slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SccComponent {
    id: ComponentId,
    member_start: usize,
    member_end: usize,
    cyclic: bool,
}

impl SccComponent {
    /// Returns the canonical component identifier.
    #[must_use]
    pub const fn id(&self) -> ComponentId {
        self.id
    }

    /// Returns the number of vertices in this quotient fiber.
    #[must_use]
    pub const fn member_count(&self) -> usize {
        self.member_end - self.member_start
    }

    /// Returns whether this component contains a directed cycle.
    #[must_use]
    pub const fn is_cyclic(&self) -> bool {
        self.cyclic
    }

    /// Returns whether this is a singleton component with a self-loop.
    #[must_use]
    pub const fn is_self_cycle(&self) -> bool {
        self.cyclic && self.member_count() == 1
    }

    /// Returns whether the cycle contains multiple vertices.
    #[must_use]
    pub const fn is_multi_vertex_cycle(&self) -> bool {
        self.member_count() > 1
    }
}

/// Exact dimensions, semantic events, phase charges, and peak stack depths for
/// one completed SCC decomposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SccWorkProfile {
    vertex_count: u64,
    edge_count: u64,
    component_count: u64,
    quotient_candidate_count: u64,
    quotient_edge_count: u64,
    peak_active_slots: usize,
    peak_frame_slots: usize,
    tarjan_work: u64,
    partition_work: u64,
    radix_work: u64,
    condensation_work: u64,
    decomposition_work: u64,
}

impl SccWorkProfile {
    fn complete(
        vertex_count: u64,
        edge_count: u64,
        component_count: u64,
        quotient_candidate_count: u64,
        quotient_edge_count: u64,
        peak_active_slots: usize,
        peak_frame_slots: usize,
    ) -> Self {
        let tarjan_work = 5 * vertex_count + edge_count;
        let partition_work = 10 * vertex_count + edge_count + 3 * component_count + 1;
        let radix_work = radix_logical_work(quotient_candidate_count);
        let condensation_work = 5 * component_count + 3 * quotient_edge_count + 2;
        let decomposition_work = partition_work + edge_count + radix_work + condensation_work;
        Self {
            vertex_count,
            edge_count,
            component_count,
            quotient_candidate_count,
            quotient_edge_count,
            peak_active_slots,
            peak_frame_slots,
            tarjan_work,
            partition_work,
            radix_work,
            condensation_work,
            decomposition_work,
        }
    }

    /// Returns the canonical source vertex count.
    #[must_use]
    pub const fn vertex_count(self) -> u64 {
        self.vertex_count
    }

    /// Returns the canonical source edge count.
    #[must_use]
    pub const fn edge_count(self) -> u64 {
        self.edge_count
    }

    /// Returns the SCC count.
    #[must_use]
    pub const fn component_count(self) -> u64 {
        self.component_count
    }

    /// Returns cross-component candidates before quotient deduplication.
    #[must_use]
    pub const fn quotient_candidate_count(self) -> u64 {
        self.quotient_candidate_count
    }

    /// Returns the distinct condensation edge count.
    #[must_use]
    pub const fn quotient_edge_count(self) -> u64 {
        self.quotient_edge_count
    }

    /// Returns the observed peak number of active Tarjan vertices.
    #[must_use]
    pub const fn peak_active_slots(self) -> usize {
        self.peak_active_slots
    }

    /// Returns the observed peak number of explicit DFS frames.
    #[must_use]
    pub const fn peak_frame_slots(self) -> usize {
        self.peak_frame_slots
    }

    /// Returns exact Tarjan semantic work, equal to `5 * V + E`.
    #[must_use]
    pub const fn tarjan_work(self) -> u64 {
        self.tarjan_work
    }

    /// Returns dense outer-loop root checks, exactly one per source vertex.
    #[must_use]
    pub const fn root_checks(self) -> u64 {
        self.vertex_count
    }

    /// Returns first-discovery events, exactly one per source vertex.
    #[must_use]
    pub const fn discoveries(self) -> u64 {
        self.vertex_count
    }

    /// Returns canonical CSR edge inspections, exactly one per source edge.
    #[must_use]
    pub const fn edge_inspections(self) -> u64 {
        self.edge_count
    }

    /// Returns explicit DFS-frame finishes, exactly one per source vertex.
    #[must_use]
    pub const fn frame_finishes(self) -> u64 {
        self.vertex_count
    }

    /// Returns removals from Tarjan's active stack, exactly one per vertex.
    #[must_use]
    pub const fn active_pops(self) -> u64 {
        self.vertex_count
    }

    /// Returns canonical component assignments, exactly one per vertex.
    #[must_use]
    pub const fn canonical_assignments(self) -> u64 {
        self.vertex_count
    }

    /// Returns phase-complete partition work.
    #[must_use]
    pub const fn partition_work(self) -> u64 {
        self.partition_work
    }

    /// Returns exact charged radix work, including deterministic preparation.
    #[must_use]
    pub const fn radix_work(self) -> u64 {
        self.radix_work
    }

    /// Returns exact paired forward/reverse condensation CSR work.
    #[must_use]
    pub const fn condensation_work(self) -> u64 {
        self.condensation_work
    }

    /// Returns exact work charged through completed decomposition construction.
    #[must_use]
    pub const fn decomposition_work(self) -> u64 {
        self.decomposition_work
    }

    /// Returns exact phase-complete work after adding a schedule produced for
    /// this decomposition's condensation graph.
    #[must_use]
    pub const fn pipeline_work(self, schedule: &WavefrontSchedule) -> u64 {
        self.decomposition_work + schedule.logical_work()
    }

    /// Returns the proven Tarjan-only auxiliary slot bound.
    #[must_use]
    pub const fn tarjan_auxiliary_slots_upper_bound(self) -> u64 {
        5 * self.vertex_count
    }

    /// Returns the proven decomposition workspace upper bound, excluding
    /// returned data.
    #[must_use]
    pub const fn decomposition_auxiliary_slots_upper_bound(self) -> u64 {
        5 * self.vertex_count
            + 2 * self.component_count
            + 2 * self.quotient_candidate_count
            + (RADIX_BUCKET_COUNT as u64)
    }

    /// Returns the proven complete pipeline workspace bound, including schedule
    /// temporaries but excluding returned data.
    #[must_use]
    pub const fn pipeline_auxiliary_slots_upper_bound(self) -> u64 {
        5 * self.vertex_count
            + 4 * self.component_count
            + 2 * self.quotient_candidate_count
            + (RADIX_BUCKET_COUNT as u64)
    }
}

/// Reusable heap workspace for stack-safe SCC and quotient construction.
#[derive(Debug, Default)]
pub struct SccWorkspace {
    discovery: Vec<u32>,
    low_link: Vec<u32>,
    raw_component_of: Vec<u32>,
    raw_component_sizes: Vec<u32>,
    raw_to_canonical: Vec<u32>,
    active: Vec<DenseId>,
    frames: Vec<DfsFrame>,
    quotient_candidates: Vec<u64>,
    radix: RadixWorkspace,
}

impl SccWorkspace {
    /// Creates an empty reusable workspace.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            discovery: Vec::new(),
            low_link: Vec::new(),
            raw_component_of: Vec::new(),
            raw_component_sizes: Vec::new(),
            raw_to_canonical: Vec::new(),
            active: Vec::new(),
            frames: Vec::new(),
            quotient_candidates: Vec::new(),
            radix: RadixWorkspace::new(),
        }
    }

    /// Computes an exact decomposition while reusing this workspace.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Invalid`] if a checked domain or internal graph
    /// invariant fails. A graph constructed by libvgraph satisfies its CSR
    /// invariants.
    pub fn compute<K: Ord>(
        &mut self,
        graph: &CsrGraph<K>,
    ) -> Result<SccDecomposition, ComputeError> {
        self.compute_impl(graph, &mut Unbounded)
    }

    /// Computes an exact decomposition under deterministic work and
    /// cancellation controls while reusing this workspace.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Incomplete`] when `control` stops execution, or
    /// [`ComputeError::Invalid`] for a checked domain or invariant failure.
    pub fn compute_with_control<K: Ord>(
        &mut self,
        graph: &CsrGraph<K>,
        control: ExecutionControl<'_>,
    ) -> Result<SccDecomposition, ComputeError> {
        self.compute_impl(graph, &mut control.meter())
    }

    fn compute_impl<K: Ord, C: WorkControl>(
        &mut self,
        graph: &CsrGraph<K>,
        control: &mut C,
    ) -> Result<SccDecomposition, ComputeError> {
        let vertex_count = graph.vertex_count();
        let vertex_count_u32 = graph.vertex_count_u32();
        let vertex_count_u64 = u64::from(vertex_count_u32);
        let edge_count_u64 = graph.edge_count() as u64;
        self.prepare(vertex_count, vertex_count_u64, control)?;
        let tarjan = self.run_tarjan(graph, vertex_count_u64, control)?;
        let mut partition =
            self.materialize_partition(vertex_count_u32, tarjan.component_count, control)?;
        let quotient =
            self.build_quotient(graph, tarjan.component_count, &mut partition, control)?;
        let profile = SccWorkProfile::complete(
            vertex_count_u64,
            edge_count_u64,
            u64::from(tarjan.component_count),
            quotient.candidate_count,
            quotient.quotient_edge_count,
            tarjan.peak_active_slots,
            tarjan.peak_frame_slots,
        );
        if control
            .consumed()
            .is_some_and(|consumed| consumed != profile.decomposition_work())
        {
            return Err(GraphError::InvalidPartition {
                reason: "decomposition work meter disagrees with the formal charge",
            }
            .into());
        }

        Ok(SccDecomposition {
            components: partition.components,
            component_of: partition.component_of,
            members: partition.members,
            condensation: quotient.condensation,
            profile,
        })
    }

    fn prepare<C: WorkControl>(
        &mut self,
        vertex_count: usize,
        vertex_count_u64: u64,
        control: &mut C,
    ) -> Result<(), ComputeError> {
        control.consume(3 * vertex_count_u64)?;
        reset_copy(&mut self.discovery, vertex_count, UNVISITED);
        reset_copy(&mut self.low_link, vertex_count, 0);
        reset_copy(&mut self.raw_component_of, vertex_count, UNASSIGNED);
        self.raw_component_sizes.clear();
        self.active.clear();
        self.frames.clear();
        self.quotient_candidates.clear();
        Ok(())
    }

    fn run_tarjan<K: Ord, C: WorkControl>(
        &mut self,
        graph: &CsrGraph<K>,
        vertex_count: u64,
        control: &mut C,
    ) -> Result<TarjanSummary, ComputeError> {
        let mut next_index = 0u32;
        let mut peak_active_slots = 0usize;
        let mut peak_frame_slots = 0usize;
        for raw_start in 0..graph.vertex_count_u32() {
            control.step()?;
            let start = DenseId::from_raw(raw_start);
            if self.discovery[start.index()] != UNVISITED {
                continue;
            }
            self.discover(
                start,
                &mut next_index,
                vertex_count,
                &mut peak_active_slots,
                &mut peak_frame_slots,
                control,
            )?;
            while !self.frames.is_empty() {
                self.advance_frame(
                    graph,
                    &mut next_index,
                    vertex_count,
                    &mut peak_active_slots,
                    &mut peak_frame_slots,
                    control,
                )?;
            }
        }
        if !self.active.is_empty() {
            return Err(GraphError::InvalidPartition {
                reason: "Tarjan traversal ended with active vertices",
            }
            .into());
        }
        let component_count = u32::try_from(self.raw_component_sizes.len()).map_err(|_| {
            GraphError::VertexDomainOverflow {
                count: self.raw_component_sizes.len() as u64,
            }
        })?;
        Ok(TarjanSummary {
            component_count,
            peak_active_slots,
            peak_frame_slots,
        })
    }

    fn discover<C: WorkControl>(
        &mut self,
        vertex: DenseId,
        next_index: &mut u32,
        vertex_count: u64,
        peak_active_slots: &mut usize,
        peak_frame_slots: &mut usize,
        control: &mut C,
    ) -> Result<(), ComputeError> {
        control.step()?;
        let discovery_index = *next_index;
        *next_index = next_index
            .checked_add(1)
            .ok_or(GraphError::VertexDomainOverflow {
                count: vertex_count,
            })?;
        self.discovery[vertex.index()] = discovery_index;
        self.low_link[vertex.index()] = discovery_index;
        self.active.push(vertex);
        self.frames.push(DfsFrame {
            vertex,
            next_successor: 0,
        });
        *peak_active_slots = (*peak_active_slots).max(self.active.len());
        *peak_frame_slots = (*peak_frame_slots).max(self.frames.len());
        Ok(())
    }

    fn advance_frame<K: Ord, C: WorkControl>(
        &mut self,
        graph: &CsrGraph<K>,
        next_index: &mut u32,
        vertex_count: u64,
        peak_active_slots: &mut usize,
        peak_frame_slots: &mut usize,
        control: &mut C,
    ) -> Result<(), ComputeError> {
        let frame = self
            .frames
            .last()
            .copied()
            .ok_or(GraphError::InvalidPartition {
                reason: "Tarjan attempted to advance an absent frame",
            })?;
        let successors = graph.successors_unchecked(frame.vertex);
        if frame.next_successor == successors.len() {
            return self.finish_vertex(frame.vertex, vertex_count, control);
        }

        control.step()?;
        let successor = successors[frame.next_successor];
        let current = self.frames.last_mut().ok_or(GraphError::InvalidPartition {
            reason: "the current Tarjan frame disappeared",
        })?;
        current.next_successor =
            current
                .next_successor
                .checked_add(1)
                .ok_or(GraphError::InvalidPartition {
                    reason: "a Tarjan successor cursor overflowed",
                })?;
        if self.discovery[successor.index()] == UNVISITED {
            self.discover(
                successor,
                next_index,
                vertex_count,
                peak_active_slots,
                peak_frame_slots,
                control,
            )?;
        } else if self.raw_component_of[successor.index()] == UNASSIGNED {
            self.low_link[frame.vertex.index()] =
                self.low_link[frame.vertex.index()].min(self.discovery[successor.index()]);
        }
        Ok(())
    }

    fn finish_vertex<C: WorkControl>(
        &mut self,
        vertex: DenseId,
        vertex_count: u64,
        control: &mut C,
    ) -> Result<(), ComputeError> {
        control.step()?;
        self.frames.pop();
        if self.low_link[vertex.index()] == self.discovery[vertex.index()] {
            let raw_component = u32::try_from(self.raw_component_sizes.len()).map_err(|_| {
                GraphError::VertexDomainOverflow {
                    count: self.raw_component_sizes.len() as u64,
                }
            })?;
            let mut component_size = 0u32;
            loop {
                control.step()?;
                let member = self.active.pop().ok_or(GraphError::InvalidPartition {
                    reason: "a Tarjan root had no active member",
                })?;
                self.raw_component_of[member.index()] = raw_component;
                component_size =
                    component_size
                        .checked_add(1)
                        .ok_or(GraphError::VertexDomainOverflow {
                            count: vertex_count,
                        })?;
                if member == vertex {
                    break;
                }
            }
            self.raw_component_sizes.push(component_size);
        }
        if let Some(parent) = self.frames.last() {
            self.low_link[parent.vertex.index()] =
                self.low_link[parent.vertex.index()].min(self.low_link[vertex.index()]);
        }
        Ok(())
    }

    fn materialize_partition<C: WorkControl>(
        &mut self,
        vertex_count: u32,
        component_count: u32,
        control: &mut C,
    ) -> Result<PartitionParts, ComputeError> {
        control.consume(u64::from(component_count))?;
        reset_copy(
            &mut self.raw_to_canonical,
            component_count as usize,
            UNASSIGNED,
        );
        let (mut components, component_of) =
            self.canonical_components(vertex_count, component_count, control)?;
        let members = self.materialize_members(
            vertex_count,
            component_count,
            &mut components,
            &component_of,
            control,
        )?;
        Ok(PartitionParts {
            components,
            component_of,
            members,
        })
    }

    fn canonical_components<C: WorkControl>(
        &mut self,
        vertex_count: u32,
        component_count: u32,
        control: &mut C,
    ) -> Result<(Vec<SccComponent>, Vec<ComponentId>), ComputeError> {
        let mut component_of = Vec::with_capacity(vertex_count as usize);
        let mut components = Vec::with_capacity(component_count as usize);
        for raw_vertex in 0..vertex_count {
            control.step()?;
            let raw_component = self.raw_component_of[raw_vertex as usize];
            if raw_component == UNASSIGNED {
                return Err(GraphError::InvalidPartition {
                    reason: "Tarjan left a vertex without a raw component",
                }
                .into());
            }
            let slot = &mut self.raw_to_canonical[raw_component as usize];
            if *slot == UNASSIGNED {
                let canonical = u32::try_from(components.len()).map_err(|_| {
                    GraphError::VertexDomainOverflow {
                        count: components.len() as u64,
                    }
                })?;
                *slot = canonical;
                let member_count = self.raw_component_sizes[raw_component as usize] as usize;
                components.push(SccComponent {
                    id: ComponentId::from_raw(canonical),
                    member_start: 0,
                    member_end: member_count,
                    cyclic: member_count > 1,
                });
            }
            component_of.push(ComponentId::from_raw(*slot));
        }
        Ok((components, component_of))
    }

    fn materialize_members<C: WorkControl>(
        &mut self,
        vertex_count: u32,
        component_count: u32,
        components: &mut [SccComponent],
        component_of: &[ComponentId],
        control: &mut C,
    ) -> Result<Vec<DenseId>, ComputeError> {
        control.consume(u64::from(component_count) + 1)?;
        let mut member_offset = 0usize;
        for component in &mut *components {
            let member_count = component.member_end;
            component.member_start = member_offset;
            member_offset = member_offset.checked_add(member_count).ok_or(
                GraphError::VertexDomainOverflow {
                    count: u64::from(vertex_count),
                },
            )?;
            component.member_end = member_offset;
        }
        if member_offset != vertex_count as usize {
            return Err(GraphError::InvalidPartition {
                reason: "component sizes do not cover the vertex domain",
            }
            .into());
        }

        control.consume(u64::from(vertex_count))?;
        let mut members = vec![DenseId::from_raw(0); vertex_count as usize];
        control.consume(u64::from(component_count))?;
        for component in &*components {
            self.raw_to_canonical[component.id.index()] = u32::try_from(component.member_start)
                .map_err(|_| GraphError::VertexDomainOverflow {
                    count: component.member_start as u64,
                })?;
        }
        for raw_vertex in 0..vertex_count {
            control.step()?;
            let vertex = DenseId::from_raw(raw_vertex);
            let component = component_of[vertex.index()];
            let cursor = &mut self.raw_to_canonical[component.index()];
            members[*cursor as usize] = vertex;
            *cursor = cursor
                .checked_add(1)
                .ok_or(GraphError::VertexDomainOverflow {
                    count: u64::from(vertex_count),
                })?;
        }
        Ok(members)
    }

    fn build_quotient<K: Ord, C: WorkControl>(
        &mut self,
        graph: &CsrGraph<K>,
        component_count: u32,
        partition: &mut PartitionParts,
        control: &mut C,
    ) -> Result<QuotientParts, ComputeError> {
        for (source, target) in graph.edges() {
            control.step()?;
            let source_component = partition.component_of[source.index()];
            let target_component = partition.component_of[target.index()];
            if source_component == target_component {
                if source == target {
                    partition.components[source_component.index()].cyclic = true;
                }
            } else {
                self.quotient_candidates
                    .push(encode_pair(source_component.get(), target_component.get()));
            }
        }
        let candidate_count = self.quotient_candidates.len() as u64;
        self.radix
            .sort_dedup(&mut self.quotient_candidates, control)?;
        let quotient_edge_count = self.quotient_candidates.len() as u64;
        let condensation =
            Condensation::from_sorted_keys(component_count, &self.quotient_candidates, control)?;
        Ok(QuotientParts {
            condensation,
            candidate_count,
            quotient_edge_count,
        })
    }
}

/// Total SCC partition and its exact condensation quotient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SccDecomposition {
    components: Vec<SccComponent>,
    component_of: Vec<ComponentId>,
    members: Vec<DenseId>,
    condensation: Condensation,
    profile: SccWorkProfile,
}

impl SccDecomposition {
    /// Computes the exact SCC partition without a practical logical-work cap.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Invalid`] if a checked domain or internal graph
    /// invariant fails.
    pub fn compute<K: Ord>(graph: &CsrGraph<K>) -> Result<Self, ComputeError> {
        SccWorkspace::default().compute(graph)
    }

    /// Computes the exact SCC partition under deterministic work and
    /// cancellation control. An incomplete computation returns no partition.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Incomplete`] when `control` stops execution, or
    /// [`ComputeError::Invalid`] for a checked domain or invariant failure.
    pub fn compute_with_control<K: Ord>(
        graph: &CsrGraph<K>,
        control: ExecutionControl<'_>,
    ) -> Result<Self, ComputeError> {
        SccWorkspace::default().compute_with_control(graph, control)
    }

    /// Returns components ordered by their least dense member.
    #[must_use]
    pub fn components(&self) -> &[SccComponent] {
        &self.components
    }

    /// Iterates component metadata together with each sorted flat fiber.
    pub fn fibers(&self) -> impl Iterator<Item = (&SccComponent, &[DenseId])> {
        self.components.iter().map(move |component| {
            (
                component,
                &self.members[component.member_start..component.member_end],
            )
        })
    }

    /// Returns the number of components.
    #[must_use]
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns the total component containing `vertex`.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::DenseIdOutOfRange`] when `vertex` is outside the
    /// decomposition's dense vertex domain.
    pub fn component_of(&self, vertex: DenseId) -> Result<ComponentId, GraphError> {
        self.component_of
            .get(vertex.index())
            .copied()
            .ok_or(GraphError::DenseIdOutOfRange {
                id: vertex,
                vertex_count: u32::try_from(self.component_of.len()).unwrap_or(u32::MAX),
            })
    }

    /// Returns one component by identifier.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::ComponentIdOutOfRange`] when `id` is outside the
    /// decomposition's component domain.
    pub fn component(&self, id: ComponentId) -> Result<&SccComponent, GraphError> {
        self.components
            .get(id.index())
            .ok_or(GraphError::ComponentIdOutOfRange {
                id,
                component_count: u32::try_from(self.components.len()).unwrap_or(u32::MAX),
            })
    }

    /// Returns the exact sorted quotient fiber above one component.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::ComponentIdOutOfRange`] when `id` is outside the
    /// decomposition's component domain.
    pub fn fiber(&self, id: ComponentId) -> Result<&[DenseId], GraphError> {
        let component = self.component(id)?;
        Ok(&self.members[component.member_start..component.member_end])
    }

    /// Returns the exact condensation DAG.
    #[must_use]
    pub const fn condensation(&self) -> &Condensation {
        &self.condensation
    }

    /// Returns the exact completed work profile.
    #[must_use]
    pub const fn work_profile(&self) -> SccWorkProfile {
        self.profile
    }

    /// Returns whether the source graph contains no directed cycle.
    #[must_use]
    pub fn is_acyclic(&self) -> bool {
        self.components.iter().all(|component| !component.cyclic)
    }

    /// Iterates components containing a directed cycle.
    pub fn cyclic_components(&self) -> impl Iterator<Item = &SccComponent> {
        self.components.iter().filter(|component| component.cyclic)
    }

    /// Revalidates partition totality/disjointness, cycle flags, exact
    /// quotient edges, and condensation acyclicity against `graph`.
    ///
    /// # Errors
    ///
    /// Returns a structured [`GraphError`] for the first partition, cycle,
    /// quotient, CSR, or acyclicity invariant that does not hold.
    pub fn validate<K: Ord>(&self, graph: &CsrGraph<K>) -> Result<(), GraphError> {
        if self.component_of.len() != graph.vertex_count()
            || self.members.len() != graph.vertex_count()
        {
            return Err(GraphError::InvalidPartition {
                reason: "partition arrays and graph vertex domains differ",
            });
        }
        if self.components.len() != self.condensation.component_count() as usize {
            return Err(GraphError::InvalidPartition {
                reason: "component and condensation domains differ",
            });
        }

        let mut seen = vec![false; graph.vertex_count()];
        let mut expected_start = 0usize;
        for (expected_id, component) in self.components.iter().enumerate() {
            if component.id.index() != expected_id {
                return Err(GraphError::InvalidPartition {
                    reason: "component identifiers must equal their positions",
                });
            }
            if component.member_start != expected_start
                || component.member_end <= component.member_start
                || component.member_end > self.members.len()
            {
                return Err(GraphError::InvalidPartition {
                    reason: "component fiber ranges must be contiguous and nonempty",
                });
            }
            let fiber = &self.members[component.member_start..component.member_end];
            if fiber.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(GraphError::InvalidPartition {
                    reason: "component members must be strictly ordered",
                });
            }
            for &member in fiber {
                if member.get() >= graph.vertex_count_u32() {
                    return Err(GraphError::InvalidPartition {
                        reason: "a component member is outside the graph domain",
                    });
                }
                if seen[member.index()] {
                    return Err(GraphError::InvalidPartition {
                        reason: "component fibers overlap",
                    });
                }
                seen[member.index()] = true;
                if self.component_of[member.index()] != component.id {
                    return Err(GraphError::InvalidPartition {
                        reason: "component lookup and fiber membership disagree",
                    });
                }
            }
            let expected_cyclic = fiber.len() > 1 || graph.contains_edge(fiber[0], fiber[0]);
            if component.cyclic != expected_cyclic {
                return Err(GraphError::InvalidPartition {
                    reason: "a component cycle flag is inconsistent",
                });
            }
            expected_start = component.member_end;
        }
        if expected_start != self.members.len() || seen.iter().any(|member_seen| !member_seen) {
            return Err(GraphError::InvalidPartition {
                reason: "the component fibers are not total",
            });
        }

        let mut expected_edges = Vec::with_capacity(graph.edge_count());
        for (source, target) in graph.edges() {
            let source_component = self.component_of[source.index()];
            let target_component = self.component_of[target.index()];
            if source_component != target_component {
                expected_edges.push(encode_pair(source_component.get(), target_component.get()));
            }
        }
        RadixWorkspace::default()
            .sort_dedup(&mut expected_edges, &mut Unbounded)
            .map_err(|_| GraphError::InvalidPartition {
                reason: "unbounded quotient validation stopped incomplete",
            })?;
        if !expected_edges
            .iter()
            .copied()
            .eq(self.condensation.encoded_edges())
        {
            return Err(GraphError::InvalidPartition {
                reason: "condensation edges are not the exact quotient",
            });
        }
        self.condensation.validate()
    }
}

fn reset_copy<T: Copy>(values: &mut Vec<T>, len: usize, value: T) {
    let retained = values.len().min(len);
    values.truncate(len);
    values[..retained].fill(value);
    values.resize(len, value);
}
