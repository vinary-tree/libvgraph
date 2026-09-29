//! Executable refinement checks for every BCSR-001 through BCSR-015 obligation.

use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;

use libvgraph::{
    BorrowedCsr, ComponentId, ComputeError, CsrGraph, DenseId, Direction, ExecutionControl,
    GraphError, IncompleteReason, SccDecomposition, SccWorkspace,
};
use proptest::prelude::*;

fn owned_graph(vertex_count: u32, edges: &[(u32, u32)]) -> CsrGraph<u32> {
    CsrGraph::from_edges(0..vertex_count, edges.iter().copied())
        .expect("bounded edge corpus has known endpoints")
}

fn raw_targets(graph: &CsrGraph<u32>) -> Vec<u32> {
    graph
        .forward_targets()
        .iter()
        .map(|target| target.get())
        .collect()
}

fn canonical_raw(vertex_count: u32, offsets: &[u32], targets: &[u32]) -> bool {
    let Some(expected_offsets) = usize::try_from(vertex_count)
        .ok()
        .and_then(|count| count.checked_add(1))
    else {
        return false;
    };
    let Ok(edge_count) = u32::try_from(targets.len()) else {
        return false;
    };
    if offsets.len() != expected_offsets
        || offsets.first() != Some(&0)
        || offsets.last() != Some(&edge_count)
    {
        return false;
    }
    for pair in offsets.windows(2) {
        if pair[0] > pair[1] || pair[1] as usize > targets.len() {
            return false;
        }
        let row = &targets[pair[0] as usize..pair[1] as usize];
        if row.iter().any(|&target| target >= vertex_count)
            || row.windows(2).any(|adjacent| adjacent[0] >= adjacent[1])
        {
            return false;
        }
    }
    true
}

#[derive(Debug, PartialEq, Eq)]
struct SemanticSignature {
    fibers: Vec<(ComponentId, Vec<DenseId>, bool)>,
    quotient: Vec<(ComponentId, ComponentId)>,
}

fn semantic_signature(result: &SccDecomposition) -> SemanticSignature {
    let fibers = result
        .fibers()
        .map(|(component, members)| (component.id(), members.to_vec(), component.is_cyclic()))
        .collect();
    let quotient = result.condensation().edges().collect();
    SemanticSignature { fibers, quotient }
}

fn reaches(graph: &CsrGraph<u32>, source: u32, target: u32) -> bool {
    let mut seen = vec![false; graph.vertex_count()];
    let mut agenda = vec![source];
    while let Some(vertex) = agenda.pop() {
        if vertex == target {
            return true;
        }
        if seen[vertex as usize] {
            continue;
        }
        seen[vertex as usize] = true;
        for successor in graph
            .successors(DenseId::from_raw(vertex))
            .expect("the oracle vertex is in range")
        {
            agenda.push(successor.get());
        }
    }
    false
}

#[test]
fn borrowed_compute_preserves_input_buffers() {
    let offsets = [0, 2, 3, 3];
    let targets = [1, 2, 2];
    let borrowed = BorrowedCsr::new(3, &offsets, &targets).expect("canonical CSR");
    assert_eq!(borrowed.offsets().as_ptr(), offsets.as_ptr());
    assert_eq!(borrowed.targets().as_ptr(), targets.as_ptr());
    let result = SccDecomposition::compute_borrowed(&borrowed).expect("valid graph");
    assert_eq!(result.component_count(), 3);
    assert_eq!(borrowed.offsets().as_ptr(), offsets.as_ptr());
    assert_eq!(borrowed.targets().as_ptr(), targets.as_ptr());
}

#[test]
fn borrowed_rejects_every_malformed_header() {
    assert!(matches!(
        BorrowedCsr::new(2, &[0, 0], &[]),
        Err(GraphError::OffsetLength { .. })
    ));
    assert!(matches!(
        BorrowedCsr::new(1, &[1, 1], &[0]),
        Err(GraphError::OffsetOrigin { .. })
    ));
    assert!(matches!(
        BorrowedCsr::new(1, &[0, 0], &[0]),
        Err(GraphError::OffsetTerminal { .. })
    ));
    assert!(BorrowedCsr::new(0, &[0], &[]).is_ok());
}

#[test]
fn borrowed_rejects_every_invalid_row_bound() {
    let malformed = BorrowedCsr::new(2, &[0, 2, 1], &[0]).expect("valid header");
    assert!(matches!(
        SccDecomposition::compute_borrowed(&malformed),
        Err(ComputeError::Invalid(GraphError::OffsetOutOfRange { .. }))
    ));
    let decreasing = BorrowedCsr::new(3, &[0, 1, 0, 1], &[0]).expect("valid header");
    assert!(matches!(
        SccDecomposition::compute_borrowed(&decreasing),
        Err(ComputeError::Invalid(GraphError::OffsetOrder {
            direction: Direction::Forward,
            index: 2,
            previous: 1,
            next: 0,
        }))
    ));
    let malformed = BorrowedCsr::new(2, &[0, 1, 0], &[]).expect("valid header");
    assert!(matches!(
        SccDecomposition::compute_borrowed(&malformed),
        Err(ComputeError::Invalid(GraphError::OffsetOutOfRange { .. }))
    ));
}

#[test]
fn borrowed_never_indexes_an_unchecked_target() {
    let malformed = BorrowedCsr::new(2, &[0, 1, 1], &[2]).expect("valid header");
    assert!(matches!(
        SccDecomposition::compute_borrowed(&malformed),
        Err(ComputeError::Invalid(GraphError::TargetOutOfRange {
            direction: Direction::Forward,
            edge_index: 0,
            target: 2,
            vertex_count: 2,
        }))
    ));
}

#[test]
fn borrowed_rejects_noncanonical_target_order() {
    for targets in [[0, 0], [1, 0]] {
        let malformed = BorrowedCsr::new(2, &[0, 2, 2], &targets).expect("valid header");
        assert!(matches!(
            SccDecomposition::compute_borrowed(&malformed),
            Err(ComputeError::Invalid(GraphError::AdjacencyOrder { .. }))
        ));
    }
}

proptest! {
    #[test]
    fn borrowed_acceptance_matches_raw_csr_contract(
        vertex_count in 0u32..6,
        offsets in prop::collection::vec(0u32..8, 0..8),
        targets in prop::collection::vec(0u32..8, 0..8),
    ) {
        let admitted = BorrowedCsr::new(vertex_count, &offsets, &targets)
            .and_then(|graph| {
                SccDecomposition::compute_borrowed(&graph)
                    .map_err(|error| match error {
                        ComputeError::Invalid(graph_error) => graph_error,
                        ComputeError::Incomplete(_) => panic!("unbounded computation cannot stop"),
                    })
            });
        prop_assert_eq!(admitted.is_ok(), canonical_raw(vertex_count, &offsets, &targets));
    }

    #[test]
    fn borrowed_and_owned_graphs_are_extensionally_equal(
        raw in prop::collection::vec((0u8..6, 0u8..6), 0..48),
    ) {
        let edges: Vec<_> = raw.into_iter().map(|(a, b)| (u32::from(a), u32::from(b))).collect();
        let owned = owned_graph(6, &edges);
        let targets = raw_targets(&owned);
        let borrowed = BorrowedCsr::new(6, owned.forward_offsets(), &targets)
            .expect("owned forward CSR is canonical");
        for vertex in 0..6 {
            let owned_targets: Vec<_> = owned
                .successors(DenseId::from_raw(vertex))
                .expect("valid vertex")
                .iter()
                .map(|id| id.get())
                .collect();
            let start = borrowed.offsets()[vertex as usize] as usize;
            let stop = borrowed.offsets()[vertex as usize + 1] as usize;
            prop_assert_eq!(&borrowed.targets()[start..stop], owned_targets.as_slice());
        }
        let borrowed_result = SccDecomposition::compute_borrowed(&borrowed).expect("valid borrowed CSR");
        let owned_result = SccDecomposition::compute(&owned).expect("valid owned CSR");
        prop_assert_eq!(semantic_signature(&borrowed_result), semantic_signature(&owned_result));
        prop_assert_eq!(borrowed_result.work_profile().validation_work(), borrowed.validation_work());
        prop_assert_eq!(owned_result.work_profile().validation_work(), 0);
    }

    #[test]
    fn borrowed_scc_fibers_equal_reachability_classes(
        raw in prop::collection::vec((0u8..6, 0u8..6), 0..40),
    ) {
        let edges: Vec<_> = raw.into_iter().map(|(a, b)| (u32::from(a), u32::from(b))).collect();
        let owned = owned_graph(6, &edges);
        let targets = raw_targets(&owned);
        let borrowed = BorrowedCsr::new(6, owned.forward_offsets(), &targets).expect("canonical CSR");
        let result = SccDecomposition::compute_borrowed(&borrowed).expect("valid graph");
        for source in 0..6 {
            for target in 0..6 {
                let same_component = result.component_of(DenseId::from_raw(source))
                    == result.component_of(DenseId::from_raw(target));
                prop_assert_eq!(same_component, reaches(&owned, source, target) && reaches(&owned, target, source));
            }
        }
    }

    #[test]
    fn borrowed_condensation_is_exact_quotient(
        raw in prop::collection::vec((0u8..6, 0u8..6), 0..40),
    ) {
        let edges: Vec<_> = raw.into_iter().map(|(a, b)| (u32::from(a), u32::from(b))).collect();
        let owned = owned_graph(6, &edges);
        let targets = raw_targets(&owned);
        let borrowed = BorrowedCsr::new(6, owned.forward_offsets(), &targets).expect("canonical CSR");
        let result = SccDecomposition::compute_borrowed(&borrowed).expect("valid graph");
        let mut expected = BTreeSet::new();
        for (source, target) in owned.edges() {
            let source_component = result.component_of(source).expect("total quotient");
            let target_component = result.component_of(target).expect("total quotient");
            if source_component != target_component {
                expected.insert((source_component, target_component));
            }
        }
        let actual: BTreeSet<_> = result.condensation().edges().collect();
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn borrowed_singleton_cycle_flag_equals_self_loop(
        raw in prop::collection::vec((0u8..6, 0u8..6), 0..40),
    ) {
        let edges: Vec<_> = raw.into_iter().map(|(a, b)| (u32::from(a), u32::from(b))).collect();
        let owned = owned_graph(6, &edges);
        let targets = raw_targets(&owned);
        let borrowed = BorrowedCsr::new(6, owned.forward_offsets(), &targets).expect("canonical CSR");
        let result = SccDecomposition::compute_borrowed(&borrowed).expect("valid graph");
        for (component, fiber) in result.fibers() {
            if fiber.len() == 1 {
                let vertex = fiber[0];
                prop_assert_eq!(component.is_self_cycle(), owned
                    .successors(vertex).expect("member in domain").contains(&vertex));
            }
        }
    }

    #[test]
    fn borrowed_workspace_obeys_proven_bounds(
        raw in prop::collection::vec((0u8..6, 0u8..6), 0..40),
    ) {
        let edges: Vec<_> = raw.into_iter().map(|(a, b)| (u32::from(a), u32::from(b))).collect();
        let owned = owned_graph(6, &edges);
        let targets = raw_targets(&owned);
        let borrowed = BorrowedCsr::new(6, owned.forward_offsets(), &targets).expect("canonical CSR");
        let mut workspace = SccWorkspace::new();
        let result = workspace.compute_borrowed(&borrowed).expect("valid graph");
        let profile = result.work_profile();
        prop_assert_eq!(profile.vertex_count(), 6);
        prop_assert_eq!(profile.edge_count(), targets.len() as u64);
        prop_assert_eq!(profile.validation_work(), borrowed.validation_work());
        prop_assert!(profile.tarjan_auxiliary_slots_upper_bound() <= 6 * 6);
        let owned_result = SccDecomposition::compute(&owned).expect("valid owned graph");
        prop_assert_eq!(semantic_signature(&result), semantic_signature(&owned_result));
    }
}

#[test]
fn borrowed_failure_and_cancellation_are_transactional() {
    let malformed = BorrowedCsr::new(2, &[0, 1, 1], &[2]).expect("valid header");
    let mut workspace = SccWorkspace::new();
    assert!(matches!(
        workspace.compute_borrowed(&malformed),
        Err(ComputeError::Invalid(GraphError::TargetOutOfRange { .. }))
    ));
    let offsets = [0, 1, 2];
    let targets = [1, 0];
    let borrowed = BorrowedCsr::new(2, &offsets, &targets).expect("canonical CSR");
    let cancelled = AtomicBool::new(true);
    assert!(matches!(
        workspace.compute_borrowed_with_control(
            &borrowed,
            ExecutionControl::unlimited().with_cancellation(&cancelled),
        ),
        Err(ComputeError::Incomplete(IncompleteReason::Cancelled { .. }))
    ));
    assert!(matches!(
        workspace.compute_borrowed_with_control(&borrowed, ExecutionControl::with_work_limit(0)),
        Err(ComputeError::Incomplete(
            IncompleteReason::WorkLimitExceeded { .. }
        ))
    ));
    assert_eq!(
        workspace.compute_borrowed(&borrowed),
        SccDecomposition::compute_borrowed(&borrowed)
    );
}

#[test]
fn borrowed_component_ids_follow_least_member_order() {
    let offsets = [0, 1, 2, 4, 5];
    let targets = [1, 0, 2, 3, 2];
    let borrowed = BorrowedCsr::new(4, &offsets, &targets).expect("canonical CSR");
    let result = SccDecomposition::compute_borrowed(&borrowed).expect("valid graph");
    let mut previous_least = None;
    for (expected_id, (component, fiber)) in result.fibers().enumerate() {
        assert_eq!(
            component.id(),
            ComponentId::from_raw(u32::try_from(expected_id).expect("four-vertex graph"))
        );
        assert!(fiber.windows(2).all(|pair| pair[0] < pair[1]));
        let least = fiber[0];
        assert!(previous_least.is_none_or(|previous| previous < least));
        previous_least = Some(least);
        for &member in fiber {
            assert_eq!(result.component_of(member), Ok(component.id()));
        }
    }
}

#[test]
fn borrowed_validation_work_is_exact() {
    let offsets = [0, 2, 3, 3];
    let targets = [1, 2, 2];
    let borrowed = BorrowedCsr::new(3, &offsets, &targets).expect("canonical CSR");
    assert_eq!(borrowed.validation_work(), 1 + 3 + 3);
    let result = SccDecomposition::compute_borrowed(&borrowed).expect("valid graph");
    assert_eq!(
        result.work_profile().validation_work(),
        borrowed.validation_work()
    );
}

#[test]
fn borrowed_empty_graph_and_exact_work_limit() {
    let empty = BorrowedCsr::new(0, &[0], &[]).expect("empty header");
    let empty_result = SccDecomposition::compute_borrowed(&empty).expect("empty graph");
    assert_eq!(empty_result.component_count(), 0);
    assert_eq!(empty_result.work_profile().validation_work(), 1);

    let offsets = [0, 1, 2];
    let targets = [1, 0];
    let borrowed = BorrowedCsr::new(2, &offsets, &targets).expect("valid header");
    let full = SccDecomposition::compute_borrowed(&borrowed).expect("valid graph");
    let exact = full.work_profile().decomposition_work();
    assert_eq!(
        SccDecomposition::compute_borrowed_with_control(
            &borrowed,
            ExecutionControl::with_work_limit(exact)
        ),
        Ok(full)
    );
    assert!(matches!(
        SccDecomposition::compute_borrowed_with_control(
            &borrowed,
            ExecutionControl::with_work_limit(exact - 1)
        ),
        Err(ComputeError::Incomplete(
            IncompleteReason::WorkLimitExceeded { .. }
        ))
    ));
}

#[test]
fn borrowed_scc_is_stack_safe_on_deep_and_wide_graphs() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let vertex_count = 20_000u32;
            let offsets: Vec<_> = (0..=vertex_count)
                .map(|vertex| vertex.min(vertex_count - 1))
                .collect();
            let targets: Vec<_> = (1..vertex_count).collect();
            let borrowed = BorrowedCsr::new(vertex_count, &offsets, &targets).expect("chain CSR");
            let result = SccDecomposition::compute_borrowed(&borrowed).expect("deep graph");
            assert_eq!(result.component_count(), vertex_count as usize);
        })
        .expect("small-stack worker")
        .join()
        .expect("iterative traversal must not overflow the native stack");
}

#[test]
fn canonical_construction_then_borrowing_is_enumeration_invariant() {
    let canonical = owned_graph(3, &[(0, 1), (1, 0), (1, 2)]);
    let repeated = owned_graph(3, &[(1, 2), (1, 0), (0, 1), (1, 0), (1, 2)]);
    assert_eq!(canonical, repeated);
    let targets = raw_targets(&canonical);
    let borrowed =
        BorrowedCsr::new(3, canonical.forward_offsets(), &targets).expect("canonical CSR");
    let borrowed_result =
        SccDecomposition::compute_borrowed(&borrowed).expect("valid borrowed CSR");
    let owned_result = SccDecomposition::compute(&repeated).expect("valid owned CSR");
    assert_eq!(
        semantic_signature(&borrowed_result),
        semantic_signature(&owned_result)
    );
    let duplicate = BorrowedCsr::new(3, &[0, 2, 2, 2], &[1, 1]).expect("valid header");
    assert!(matches!(
        SccDecomposition::compute_borrowed(&duplicate),
        Err(ComputeError::Invalid(GraphError::AdjacencyOrder { .. }))
    ));
}
