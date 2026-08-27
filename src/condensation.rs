use crate::control::{Unbounded, WorkControl};
use crate::csr::Csr;
use crate::radix::{pair_source, pair_target};
use crate::{ComponentId, ComputeError, Direction, ExecutionControl, GraphError};

/// Immutable SCC condensation DAG in forward and reverse CSR form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condensation {
    component_count: u32,
    forward: Csr<ComponentId>,
    reverse: Csr<ComponentId>,
}

impl Condensation {
    pub(crate) fn from_sorted_keys<C: WorkControl>(
        component_count: u32,
        keys: &[u64],
        control: &mut C,
    ) -> Result<Self, ComputeError> {
        let component_count_usize = component_count as usize;
        let offset_count =
            component_count_usize
                .checked_add(1)
                .ok_or(GraphError::VertexDomainOverflow {
                    count: u64::from(component_count),
                })?;
        let edge_count = u32::try_from(keys.len()).map_err(|_| GraphError::EdgeDomainOverflow {
            count: keys.len() as u64,
        })?;

        control.consume(2 * u64::from(component_count) + 2)?;
        let mut forward_offsets = vec![0u32; offset_count];
        let mut reverse_offsets = vec![0u32; offset_count];
        let mut forward_targets = Vec::with_capacity(keys.len());

        control.consume(u64::from(edge_count))?;
        let mut previous = None;
        for &key in keys {
            if previous.is_some_and(|prior| prior >= key) {
                return Err(GraphError::InvalidPartition {
                    reason: "condensation keys must be strictly ordered",
                }
                .into());
            }
            previous = Some(key);
            let source = pair_source(key);
            let target = pair_target(key);
            if source >= component_count || target >= component_count {
                return Err(GraphError::InvalidPartition {
                    reason: "a condensation edge lies outside its component domain",
                }
                .into());
            }
            if source == target {
                return Err(GraphError::InvalidPartition {
                    reason: "a condensation edge cannot be a self-loop",
                }
                .into());
            }
            let source_slot = &mut forward_offsets[source as usize + 1];
            *source_slot = source_slot
                .checked_add(1)
                .ok_or(GraphError::EdgeDomainOverflow {
                    count: u64::from(edge_count),
                })?;
            let target_slot = &mut reverse_offsets[target as usize + 1];
            *target_slot = target_slot
                .checked_add(1)
                .ok_or(GraphError::EdgeDomainOverflow {
                    count: u64::from(edge_count),
                })?;
            forward_targets.push(ComponentId::from_raw(target));
        }

        control.consume(2 * u64::from(component_count))?;
        for index in 1..offset_count {
            forward_offsets[index] = forward_offsets[index]
                .checked_add(forward_offsets[index - 1])
                .ok_or(GraphError::EdgeDomainOverflow {
                    count: u64::from(edge_count),
                })?;
            reverse_offsets[index] = reverse_offsets[index]
                .checked_add(reverse_offsets[index - 1])
                .ok_or(GraphError::EdgeDomainOverflow {
                    count: u64::from(edge_count),
                })?;
        }

        control.consume(u64::from(edge_count))?;
        let mut reverse_targets = vec![ComponentId::from_raw(0); keys.len()];
        control.consume(u64::from(edge_count))?;
        for &key in keys.iter().rev() {
            let source = pair_source(key);
            let target = pair_target(key);
            let cursor = &mut reverse_offsets[target as usize + 1];
            *cursor = cursor.checked_sub(1).ok_or(GraphError::InvalidPartition {
                reason: "reverse condensation cursor underflowed",
            })?;
            reverse_targets[*cursor as usize] = ComponentId::from_raw(source);
        }

        control.consume(u64::from(component_count))?;
        for index in 0..component_count_usize {
            reverse_offsets[index] = reverse_offsets[index + 1];
        }
        reverse_offsets[component_count_usize] = edge_count;

        Ok(Self {
            component_count,
            forward: Csr::from_parts(forward_offsets, forward_targets),
            reverse: Csr::from_parts(reverse_offsets, reverse_targets),
        })
    }

    /// Returns the number of SCC vertices.
    #[must_use]
    pub const fn component_count(&self) -> u32 {
        self.component_count
    }

    /// Returns the number of deduplicated condensation edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.forward.edge_count()
    }

    /// Returns sorted successor components.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::ComponentIdOutOfRange`] when `component` is outside
    /// this condensation's component domain.
    pub fn successors(&self, component: ComponentId) -> Result<&[ComponentId], GraphError> {
        self.validate_component(component)?;
        Ok(self.forward.slice(component))
    }

    /// Returns sorted predecessor components.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::ComponentIdOutOfRange`] when `component` is outside
    /// this condensation's component domain.
    pub fn predecessors(&self, component: ComponentId) -> Result<&[ComponentId], GraphError> {
        self.validate_component(component)?;
        Ok(self.reverse.slice(component))
    }

    /// Iterates canonical condensation edges.
    pub fn edges(&self) -> impl Iterator<Item = (ComponentId, ComponentId)> + '_ {
        (0..self.component_count).flat_map(move |source| {
            let source = ComponentId::from_raw(source);
            self.forward
                .slice(source)
                .iter()
                .copied()
                .map(move |target| (source, target))
        })
    }

    /// Computes deterministic topological order, longest-predecessor ranks,
    /// and dependency-independent waves in strict linear work.
    ///
    /// Initial ready components and every adjacency slice are ascending.
    /// FIFO admission therefore defines a reproducible order without a
    /// logarithmic priority queue. Rank and wave membership are independent of
    /// which valid deterministic topological order is chosen.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Invalid`] if internal condensation or rank
    /// invariants fail. A value constructed by libvgraph satisfies them.
    pub fn schedule(&self) -> Result<WavefrontSchedule, ComputeError> {
        self.schedule_impl(&mut Unbounded)
    }

    /// Computes the same exact schedule under logical-work and cancellation
    /// controls. An incomplete result contains no partial schedule.
    ///
    /// # Errors
    ///
    /// Returns [`ComputeError::Incomplete`] when the work limit or cancellation
    /// flag stops execution, and [`ComputeError::Invalid`] if an internal
    /// condensation or rank invariant fails.
    pub fn schedule_with_control(
        &self,
        control: ExecutionControl<'_>,
    ) -> Result<WavefrontSchedule, ComputeError> {
        self.schedule_impl(&mut control.meter())
    }

    /// Validates CSR transpose, self-loop absence, and acyclicity.
    ///
    /// # Errors
    ///
    /// Returns a structured [`GraphError`] for the first violated CSR,
    /// transpose, self-loop, or acyclicity invariant.
    pub fn validate(&self) -> Result<(), GraphError> {
        self.forward
            .validate(self.component_count, Direction::Forward)?;
        self.reverse
            .validate(self.component_count, Direction::Reverse)?;
        if self.forward.edge_count() != self.reverse.edge_count() {
            return Err(GraphError::ReverseEdgeCount {
                forward: self.forward.edge_count(),
                reverse: self.reverse.edge_count(),
            });
        }
        if self.forward.transpose(self.component_count)? != self.reverse {
            return Err(GraphError::InvalidPartition {
                reason: "forward and reverse condensation edges disagree",
            });
        }
        for source in 0..self.component_count {
            let source = ComponentId::from_raw(source);
            if self.forward.contains(source, source) {
                return Err(GraphError::InvalidPartition {
                    reason: "a condensation edge cannot be a self-loop",
                });
            }
        }
        match self.schedule() {
            Ok(_) => Ok(()),
            Err(ComputeError::Invalid(error)) => Err(error),
            Err(ComputeError::Incomplete(_)) => Err(GraphError::InvalidPartition {
                reason: "an unbounded condensation validation stopped incomplete",
            }),
        }
    }

    pub(crate) fn encoded_edges(&self) -> impl Iterator<Item = u64> + '_ {
        self.edges()
            .map(|(source, target)| crate::radix::encode_pair(source.get(), target.get()))
    }

    fn schedule_impl<C: WorkControl>(
        &self,
        control: &mut C,
    ) -> Result<WavefrontSchedule, ComputeError> {
        let component_count = self.component_count as usize;
        control.consume(u64::from(self.component_count))?;
        let mut remaining_predecessors = Vec::with_capacity(component_count);
        let mut topological_order = Vec::with_capacity(component_count);
        for raw_component in 0..self.component_count {
            let component = ComponentId::from_raw(raw_component);
            let count = self.reverse.slice(component).len();
            remaining_predecessors.push(count);
            if count == 0 {
                topological_order.push(component);
            }
        }

        control.consume(u64::from(self.component_count))?;
        let mut ranks = vec![0u32; component_count];
        let mut maximum_rank = 0u32;
        let mut ready_index = 0usize;
        while ready_index < topological_order.len() {
            control.step()?;
            let component = topological_order[ready_index];
            ready_index += 1;
            for &successor in self.forward.slice(component) {
                control.step()?;
                let candidate_rank = ranks[component.index()]
                    .checked_add(1)
                    .ok_or(GraphError::RankOverflow)?;
                ranks[successor.index()] = ranks[successor.index()].max(candidate_rank);
                maximum_rank = maximum_rank.max(ranks[successor.index()]);
                let remaining = &mut remaining_predecessors[successor.index()];
                if *remaining == 0 {
                    return Err(GraphError::InvalidPartition {
                        reason: "a condensation indegree was decremented below zero",
                    }
                    .into());
                }
                *remaining -= 1;
                if *remaining == 0 {
                    topological_order.push(successor);
                }
            }
        }
        if topological_order.len() != component_count {
            return Err(GraphError::CondensationCycle.into());
        }

        let wave_count = if component_count == 0 {
            0
        } else {
            maximum_rank
                .checked_add(1)
                .ok_or(GraphError::RankOverflow)?
        };
        let (wave_offsets, wave_members) =
            materialize_flat_waves(&ranks, self.component_count, wave_count, control)?;

        let logical_work = 6 * u64::from(self.component_count)
            + self.edge_count() as u64
            + 3 * u64::from(wave_count)
            + 1;
        if control
            .consumed()
            .is_some_and(|consumed| consumed != logical_work)
        {
            return Err(GraphError::InvalidPartition {
                reason: "wavefront work meter disagrees with the formal charge",
            }
            .into());
        }
        Ok(WavefrontSchedule {
            topological_order,
            ranks,
            wave_offsets,
            wave_members,
            logical_work,
        })
    }

    fn validate_component(&self, id: ComponentId) -> Result<(), GraphError> {
        if id.get() >= self.component_count {
            return Err(GraphError::ComponentIdOutOfRange {
                id,
                component_count: self.component_count,
            });
        }
        Ok(())
    }
}

fn materialize_flat_waves<C: WorkControl>(
    ranks: &[u32],
    component_count: u32,
    wave_count: u32,
    control: &mut C,
) -> Result<(Vec<u32>, Vec<ComponentId>), ComputeError> {
    let wave_count_usize = wave_count as usize;
    let wave_offset_count = wave_count_usize
        .checked_add(1)
        .ok_or(GraphError::RankOverflow)?;

    control.consume(u64::from(wave_count) + 1)?;
    let mut wave_offsets = vec![0u32; wave_offset_count];
    control.consume(u64::from(component_count))?;
    for &rank in ranks {
        let offset = &mut wave_offsets[rank as usize + 1];
        *offset = offset.checked_add(1).ok_or(GraphError::RankOverflow)?;
    }

    control.consume(u64::from(wave_count))?;
    for wave in 0..wave_count_usize {
        wave_offsets[wave + 1] = wave_offsets[wave + 1]
            .checked_add(wave_offsets[wave])
            .ok_or(GraphError::RankOverflow)?;
    }

    control.consume(u64::from(component_count))?;
    let mut wave_members = vec![ComponentId::from_raw(0); component_count as usize];
    control.consume(u64::from(component_count))?;
    for raw_component in (0..component_count).rev() {
        let component = ComponentId::from_raw(raw_component);
        let cursor = &mut wave_offsets[ranks[component.index()] as usize + 1];
        *cursor = cursor.checked_sub(1).ok_or(GraphError::InvalidPartition {
            reason: "flat wave cursor underflowed",
        })?;
        wave_members[*cursor as usize] = component;
    }

    control.consume(u64::from(wave_count))?;
    for wave in 0..wave_count_usize {
        wave_offsets[wave] = wave_offsets[wave + 1];
    }
    wave_offsets[wave_count_usize] = component_count;
    Ok((wave_offsets, wave_members))
}

/// Canonical topological and parallel-wavefront schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavefrontSchedule {
    topological_order: Vec<ComponentId>,
    ranks: Vec<u32>,
    wave_offsets: Vec<u32>,
    wave_members: Vec<ComponentId>,
    logical_work: u64,
}

impl WavefrontSchedule {
    /// Returns the deterministic FIFO-Kahn topological order.
    #[must_use]
    pub fn topological_order(&self) -> &[ComponentId] {
        &self.topological_order
    }

    /// Returns the longest-predecessor rank of a valid component.
    #[must_use]
    pub fn rank(&self, component: ComponentId) -> Option<u32> {
        self.ranks.get(component.index()).copied()
    }

    /// Returns ranks indexed by component identifier.
    #[must_use]
    pub fn ranks(&self) -> &[u32] {
        &self.ranks
    }

    /// Returns the number of nonempty dependency-independent waves.
    #[must_use]
    pub fn wave_count(&self) -> usize {
        self.wave_offsets.len() - 1
    }

    /// Returns one dependency-independent wave by zero-based rank.
    #[must_use]
    pub fn wave(&self, rank: usize) -> Option<&[ComponentId]> {
        let next_rank = rank.checked_add(1)?;
        let start = *self.wave_offsets.get(rank)? as usize;
        let end = *self.wave_offsets.get(next_rank)? as usize;
        Some(&self.wave_members[start..end])
    }

    /// Iterates dependency-independent waves in ascending rank order.
    #[must_use]
    pub fn waves(
        &self,
    ) -> impl DoubleEndedIterator<Item = &[ComponentId]> + ExactSizeIterator + '_ {
        self.wave_offsets
            .windows(2)
            .map(|offsets| &self.wave_members[offsets[0] as usize..offsets[1] as usize])
    }

    /// Returns flat wave offsets. Adjacent offsets delimit one wave.
    #[must_use]
    pub fn wave_offsets(&self) -> &[u32] {
        &self.wave_offsets
    }

    /// Returns all wave members in ascending wave and component order.
    #[must_use]
    pub fn wave_members(&self) -> &[ComponentId] {
        &self.wave_members
    }

    /// Returns exact charged schedule work, equal to six component passes,
    /// one quotient-edge scan, three wave-sized passes, and one terminal slot.
    #[must_use]
    pub const fn logical_work(&self) -> u64 {
        self.logical_work
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    #[kani::unwind(5)]
    fn flat_wave_buffers_are_exact_sorted_rank_fibers() {
        const COMPONENT_COUNT: u32 = 3;
        const WAVE_COUNT: u32 = 3;
        let raw_ranks = kani::any::<[u8; COMPONENT_COUNT as usize]>();
        for rank in raw_ranks {
            kani::assume(rank < WAVE_COUNT as u8);
        }
        let ranks = [
            u32::from(raw_ranks[0]),
            u32::from(raw_ranks[1]),
            u32::from(raw_ranks[2]),
        ];

        let Ok((offsets, members)) =
            materialize_flat_waves(&ranks, COMPONENT_COUNT, WAVE_COUNT, &mut Unbounded)
        else {
            panic!("valid rank fibers must materialize");
        };

        assert_eq!(offsets.len(), WAVE_COUNT as usize + 1);
        assert_eq!(members.len(), COMPONENT_COUNT as usize);
        assert_eq!(offsets[0], 0);
        assert_eq!(offsets[WAVE_COUNT as usize], COMPONENT_COUNT);

        let mut seen = [false; COMPONENT_COUNT as usize];
        for wave in 0..WAVE_COUNT as usize {
            let start = offsets[wave] as usize;
            let end = offsets[wave + 1] as usize;
            assert!(start <= end);
            assert!(end <= members.len());
            for position in start..end {
                let component = members[position].index();
                assert!(component < COMPONENT_COUNT as usize);
                assert_eq!(ranks[component] as usize, wave);
                assert!(!seen[component]);
                seen[component] = true;
                if position + 1 < end {
                    assert!(members[position] < members[position + 1]);
                }
            }
        }
        for member_was_seen in seen {
            assert!(member_was_seen);
        }
    }
}
