use libvgraph::{
    ComponentId, ComputeError, CsrGraph, DenseId, ExecutionControl, GraphError, IncompleteReason,
    SccDecomposition, SccWorkspace,
};

fn dense(values: &[u32]) -> Vec<DenseId> {
    values.iter().copied().map(DenseId::from_raw).collect()
}

fn components(values: &[u32]) -> Vec<ComponentId> {
    values.iter().copied().map(ComponentId::from_raw).collect()
}

fn decompose(graph: &CsrGraph<u32>) -> SccDecomposition {
    match SccDecomposition::compute(graph) {
        Ok(value) => value,
        Err(error) => panic!("valid SCC decomposition failed: {error}"),
    }
}

#[test]
fn computes_exact_fibers_cycles_condensation_and_ranks() {
    let graph = match CsrGraph::from_edges(
        0..8,
        [
            (0, 1),
            (1, 2),
            (2, 0),
            (2, 3),
            (3, 3),
            (3, 4),
            (4, 5),
            (5, 6),
            (6, 5),
            (6, 7),
        ],
    ) {
        Ok(graph) => graph,
        Err(error) => panic!("valid graph failed to construct: {error}"),
    };
    let decomposition = decompose(&graph);

    let fibers: Vec<Vec<DenseId>> = decomposition
        .fibers()
        .map(|(_, fiber)| fiber.to_vec())
        .collect();
    assert_eq!(
        fibers,
        vec![
            dense(&[0, 1, 2]),
            dense(&[3]),
            dense(&[4]),
            dense(&[5, 6]),
            dense(&[7]),
        ]
    );
    assert_eq!(
        decomposition
            .components()
            .iter()
            .map(libvgraph::SccComponent::is_cyclic)
            .collect::<Vec<_>>(),
        vec![true, true, false, true, false]
    );
    assert!(decomposition.components()[1].is_self_cycle());
    assert!(decomposition.components()[0].is_multi_vertex_cycle());
    assert!(!decomposition.is_acyclic());
    assert_eq!(decomposition.validate(&graph), Ok(()));

    let condensation = decomposition.condensation();
    assert_eq!(
        condensation.edges().collect::<Vec<_>>(),
        vec![
            (ComponentId::from_raw(0), ComponentId::from_raw(1)),
            (ComponentId::from_raw(1), ComponentId::from_raw(2)),
            (ComponentId::from_raw(2), ComponentId::from_raw(3)),
            (ComponentId::from_raw(3), ComponentId::from_raw(4)),
        ]
    );
    let schedule = match condensation.schedule() {
        Ok(schedule) => schedule,
        Err(error) => panic!("valid condensation failed to schedule: {error}"),
    };
    assert_eq!(schedule.topological_order(), components(&[0, 1, 2, 3, 4]));
    assert_eq!(schedule.ranks(), &[0, 1, 2, 3, 4]);
    let expected_waves = [
        components(&[0]),
        components(&[1]),
        components(&[2]),
        components(&[3]),
        components(&[4]),
    ];
    assert!(schedule
        .waves()
        .eq(expected_waves.iter().map(Vec::as_slice)));
    assert_eq!(schedule.wave_offsets(), &[0, 1, 2, 3, 4, 5]);
    assert_eq!(schedule.wave_members(), components(&[0, 1, 2, 3, 4]));
    assert_eq!(schedule.wave_count(), 5);
    assert_eq!(schedule.logical_work(), 50);
}

#[test]
fn fifo_ready_order_and_parallel_waves_are_canonical() {
    let graph = match CsrGraph::from_edges(0..5, [(0, 3), (1, 3), (1, 4), (2, 4)]) {
        Ok(graph) => graph,
        Err(error) => panic!("valid graph failed to construct: {error}"),
    };
    let decomposition = decompose(&graph);
    assert!(decomposition.is_acyclic());
    let schedule = match decomposition.condensation().schedule() {
        Ok(schedule) => schedule,
        Err(error) => panic!("valid condensation failed to schedule: {error}"),
    };
    assert_eq!(schedule.topological_order(), components(&[0, 1, 2, 3, 4]));
    assert_eq!(schedule.ranks(), &[0, 0, 0, 1, 1]);
    let expected_waves = [components(&[0, 1, 2]), components(&[3, 4])];
    assert!(schedule
        .waves()
        .eq(expected_waves.iter().map(Vec::as_slice)));
    assert_eq!(schedule.wave(0), Some(expected_waves[0].as_slice()));
    assert_eq!(schedule.wave(1), Some(expected_waves[1].as_slice()));
    assert_eq!(schedule.wave(2), None);
    assert_eq!(schedule.wave_offsets(), &[0, 3, 5]);
    assert_eq!(schedule.wave_members(), components(&[0, 1, 2, 3, 4]));
    assert_eq!(schedule.logical_work(), 41);
    for (source, target) in decomposition.condensation().edges() {
        assert!(schedule.rank(source) < schedule.rank(target));
    }
}

#[test]
fn empty_graph_has_empty_exact_quotient() {
    let graph = match CsrGraph::<u32>::from_edges([], []) {
        Ok(graph) => graph,
        Err(error) => panic!("empty graph failed to construct: {error}"),
    };
    let decomposition = decompose(&graph);
    assert_eq!(decomposition.component_count(), 0);
    assert!(decomposition.is_acyclic());
    let schedule = match decomposition.condensation().schedule() {
        Ok(schedule) => schedule,
        Err(error) => panic!("empty condensation failed to schedule: {error}"),
    };
    assert!(schedule.topological_order().is_empty());
    assert!(schedule.ranks().is_empty());
    assert_eq!(schedule.wave_count(), 0);
    assert!(schedule.waves().next().is_none());
    assert_eq!(schedule.wave_offsets(), &[0]);
    assert!(schedule.wave_members().is_empty());
    assert_eq!(schedule.logical_work(), 1);
    let profile = decomposition.work_profile();
    assert_eq!(profile.tarjan_work(), 0);
    assert_eq!(profile.partition_work(), 1);
    assert_eq!(profile.radix_work(), 0);
    assert_eq!(profile.condensation_work(), 2);
    assert_eq!(profile.decomposition_work(), 3);
    assert_eq!(profile.pipeline_work(&schedule), 4);
}

#[test]
fn invalid_ids_and_work_limits_are_structured() {
    let graph = match CsrGraph::from_edges([0], [(0, 0)]) {
        Ok(graph) => graph,
        Err(error) => panic!("valid graph failed to construct: {error}"),
    };
    assert_eq!(
        SccDecomposition::compute_with_control(&graph, ExecutionControl::with_work_limit(0)),
        Err(ComputeError::Incomplete(
            IncompleteReason::WorkLimitExceeded {
                limit: 0,
                consumed: 0,
            }
        ))
    );
    let decomposition = decompose(&graph);
    assert!(matches!(
        decomposition.component(ComponentId::from_raw(1)),
        Err(GraphError::ComponentIdOutOfRange { .. })
    ));
    assert!(matches!(
        decomposition.component_of(DenseId::from_raw(1)),
        Err(GraphError::DenseIdOutOfRange { .. })
    ));
    assert_eq!(
        decomposition
            .condensation()
            .schedule_with_control(ExecutionControl::with_work_limit(0)),
        Err(ComputeError::Incomplete(
            IncompleteReason::WorkLimitExceeded {
                limit: 0,
                consumed: 0,
            }
        ))
    );
}

#[test]
fn exact_work_profiles_limits_and_auxiliary_bounds_refine_the_proof() {
    let graph = match CsrGraph::from_edges(0..4, [(0, 1), (1, 2), (2, 3)]) {
        Ok(graph) => graph,
        Err(error) => panic!("valid graph failed to construct: {error}"),
    };
    let decomposition = decompose(&graph);
    let profile = decomposition.work_profile();

    assert_eq!(profile.vertex_count(), 4);
    assert_eq!(profile.edge_count(), 3);
    assert_eq!(profile.component_count(), 4);
    assert_eq!(profile.quotient_candidate_count(), 3);
    assert_eq!(profile.quotient_edge_count(), 3);
    assert_eq!(profile.root_checks(), 4);
    assert_eq!(profile.discoveries(), 4);
    assert_eq!(profile.edge_inspections(), 3);
    assert_eq!(profile.frame_finishes(), 4);
    assert_eq!(profile.active_pops(), 4);
    assert_eq!(profile.canonical_assignments(), 4);
    assert_eq!(profile.tarjan_work(), 23);
    assert_eq!(
        profile.root_checks()
            + profile.discoveries()
            + profile.edge_inspections()
            + profile.frame_finishes()
            + profile.active_pops()
            + profile.canonical_assignments(),
        profile.tarjan_work()
    );
    assert_eq!(profile.partition_work(), 56);
    assert_eq!(profile.radix_work(), 26_666);
    assert_eq!(profile.condensation_work(), 31);
    assert_eq!(profile.decomposition_work(), 26_756);
    assert_eq!(profile.tarjan_auxiliary_slots_upper_bound(), 20);
    assert_eq!(profile.decomposition_auxiliary_slots_upper_bound(), 2_082);
    assert_eq!(profile.pipeline_auxiliary_slots_upper_bound(), 2_090);

    let schedule = match decomposition.condensation().schedule() {
        Ok(schedule) => schedule,
        Err(error) => panic!("valid condensation failed to schedule: {error}"),
    };
    assert_eq!(schedule.logical_work(), 40);
    assert_eq!(profile.pipeline_work(&schedule), 26_796);
    assert_eq!(
        profile.pipeline_work(&schedule),
        27 * profile.vertex_count() + 20 * profile.edge_count() + 26_628
    );

    let exact_decomposition = match SccDecomposition::compute_with_control(
        &graph,
        ExecutionControl::with_work_limit(profile.decomposition_work()),
    ) {
        Ok(decomposition) => decomposition,
        Err(error) => panic!("exact decomposition work limit failed: {error}"),
    };
    assert_eq!(exact_decomposition, decomposition);
    assert!(matches!(
        SccDecomposition::compute_with_control(
            &graph,
            ExecutionControl::with_work_limit(profile.decomposition_work() - 1),
        ),
        Err(ComputeError::Incomplete(
            IncompleteReason::WorkLimitExceeded { .. }
        ))
    ));

    let exact_schedule = match decomposition
        .condensation()
        .schedule_with_control(ExecutionControl::with_work_limit(schedule.logical_work()))
    {
        Ok(schedule) => schedule,
        Err(error) => panic!("exact schedule work limit failed: {error}"),
    };
    assert_eq!(exact_schedule, schedule);
    assert!(matches!(
        decomposition
            .condensation()
            .schedule_with_control(ExecutionControl::with_work_limit(
                schedule.logical_work() - 1
            ),),
        Err(ComputeError::Incomplete(
            IncompleteReason::WorkLimitExceeded { .. }
        ))
    ));
}

#[test]
fn reusable_workspace_preserves_canonical_results_across_graph_shapes() {
    let chain = match CsrGraph::from_edges(0..32, (0..31).map(|source| (source, source + 1))) {
        Ok(graph) => graph,
        Err(error) => panic!("valid chain failed to construct: {error}"),
    };
    let cyclic = match CsrGraph::from_edges(
        0..8,
        (0..8).flat_map(|source| [(source, (source + 1) % 8), (source, source)]),
    ) {
        Ok(graph) => graph,
        Err(error) => panic!("valid cyclic graph failed to construct: {error}"),
    };

    let mut workspace = SccWorkspace::new();
    let first_chain = match workspace.compute(&chain) {
        Ok(value) => value,
        Err(error) => panic!("first workspace computation failed: {error}"),
    };
    let cyclic_result = match workspace.compute(&cyclic) {
        Ok(value) => value,
        Err(error) => panic!("second workspace computation failed: {error}"),
    };
    let second_chain = match workspace.compute(&chain) {
        Ok(value) => value,
        Err(error) => panic!("reused workspace computation failed: {error}"),
    };

    assert_eq!(first_chain, second_chain);
    assert_eq!(first_chain.validate(&chain), Ok(()));
    assert_eq!(cyclic_result.validate(&cyclic), Ok(()));
    assert_eq!(cyclic_result.component_count(), 1);
    assert!(!cyclic_result.is_acyclic());
}
