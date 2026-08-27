use std::sync::atomic::AtomicBool;

use libvgraph::{
    BuildOptions, ComputeError, CsrGraph, DenseId, Direction, Endpoint, ExecutionControl,
    GraphError, GraphLimits, IncompleteReason, ReversePolicy,
};

fn dense(values: &[u32]) -> Vec<DenseId> {
    values.iter().copied().map(DenseId::from_raw).collect()
}

fn graph() -> CsrGraph<u32> {
    match CsrGraph::from_edges(0..6, [(0, 2), (0, 1), (1, 4), (2, 3), (5, 5), (0, 1)]) {
        Ok(graph) => graph,
        Err(error) => panic!("valid test graph failed to construct: {error}"),
    }
}

#[test]
fn canonicalizes_nodes_edges_and_reverse_csr() {
    let graph =
        match CsrGraph::from_edges([30, 10, 20, 10], [(10, 20), (30, 10), (20, 20), (10, 20)]) {
            Ok(graph) => graph,
            Err(error) => panic!("valid graph failed to construct: {error}"),
        };

    assert_eq!(graph.nodes(), &[10, 20, 30]);
    assert_eq!(graph.vertex_count(), 3);
    assert_eq!(graph.edge_count(), 3);
    assert_eq!(graph.forward_offsets(), &[0, 1, 2, 3]);
    assert_eq!(graph.forward_targets(), dense(&[1, 1, 0]));
    assert_eq!(graph.reverse_offsets(), Some(&[0, 1, 3, 3][..]));
    assert_eq!(graph.reverse_targets(), Some(dense(&[2, 0, 1]).as_slice()));
    assert_eq!(graph.dense_id(&20), Some(DenseId::from_raw(1)));
    assert_eq!(graph.stable_id(DenseId::from_raw(2)), Ok(&30));
    assert_eq!(
        graph.edges().collect::<Vec<_>>(),
        vec![
            (DenseId::from_raw(0), DenseId::from_raw(1)),
            (DenseId::from_raw(1), DenseId::from_raw(1)),
            (DenseId::from_raw(2), DenseId::from_raw(0)),
        ]
    );
    assert_eq!(graph.validate(), Ok(()));
}

#[test]
fn rejects_unknown_endpoints_and_counts_raw_inputs() {
    assert_eq!(
        CsrGraph::from_edges([0], [(1, 0)]),
        Err(GraphError::UnknownEndpoint {
            edge_index: 0,
            endpoint: Endpoint::Source,
        })
    );
    assert_eq!(
        CsrGraph::from_edges([0], [(0, 1)]),
        Err(GraphError::UnknownEndpoint {
            edge_index: 0,
            endpoint: Endpoint::Target,
        })
    );

    let vertex_limited = BuildOptions {
        limits: GraphLimits {
            max_vertex_inputs: 1,
            max_edge_inputs: 8,
        },
        reverse: ReversePolicy::Build,
    };
    assert_eq!(
        CsrGraph::<u32>::from_edges_with_options([0, 0], [], vertex_limited),
        Err(GraphError::VertexInputLimitExceeded { limit: 1 })
    );

    let edge_limited = BuildOptions {
        limits: GraphLimits {
            max_vertex_inputs: 8,
            max_edge_inputs: 1,
        },
        reverse: ReversePolicy::Build,
    };
    assert_eq!(
        CsrGraph::from_edges_with_options([0], [(0, 0), (0, 0)], edge_limited),
        Err(GraphError::EdgeInputLimitExceeded { limit: 1 })
    );
}

#[test]
fn validates_imported_csr_failure_matrix() {
    assert!(matches!(
        CsrGraph::try_from_parts(vec![1, 1], vec![0, 0, 0], vec![], None),
        Err(GraphError::StableNodeOrder { index: 1 })
    ));
    assert!(matches!(
        CsrGraph::try_from_parts(vec![0], vec![0], vec![], None),
        Err(GraphError::OffsetLength {
            direction: Direction::Forward,
            ..
        })
    ));
    assert!(matches!(
        CsrGraph::try_from_parts(vec![0], vec![1, 1], vec![DenseId::from_raw(0)], None),
        Err(GraphError::OffsetOrigin {
            direction: Direction::Forward,
            ..
        })
    ));
    assert!(matches!(
        CsrGraph::try_from_parts(vec![0, 1], vec![0, 1, 0], vec![DenseId::from_raw(1)], None,),
        Err(GraphError::OffsetOrder {
            direction: Direction::Forward,
            ..
        })
    ));
    assert!(matches!(
        CsrGraph::try_from_parts(vec![0], vec![0, 0], vec![DenseId::from_raw(0)], None),
        Err(GraphError::OffsetTerminal {
            direction: Direction::Forward,
            ..
        })
    ));
    assert!(matches!(
        CsrGraph::try_from_parts(vec![0], vec![0, 1], vec![DenseId::from_raw(1)], None),
        Err(GraphError::TargetOutOfRange {
            direction: Direction::Forward,
            ..
        })
    ));
    assert!(matches!(
        CsrGraph::try_from_parts(vec![0, 1], vec![0, 2, 2], dense(&[1, 1]), None,),
        Err(GraphError::AdjacencyOrder {
            direction: Direction::Forward,
            ..
        })
    ));
    assert!(matches!(
        CsrGraph::try_from_parts(
            vec![0, 1],
            vec![0, 1, 1],
            dense(&[1]),
            Some((vec![0, 0, 0], vec![])),
        ),
        Err(GraphError::ReverseEdgeCount { .. })
    ));
    assert!(matches!(
        CsrGraph::try_from_parts(
            vec![0, 1],
            vec![0, 1, 1],
            dense(&[1]),
            Some((vec![0, 1, 1], dense(&[1]))),
        ),
        Err(GraphError::ReverseEdgeMissing { .. })
    ));
}

#[test]
fn optional_reverse_can_be_materialized_without_changing_forward_csr() {
    let options = BuildOptions {
        reverse: ReversePolicy::Omit,
        ..BuildOptions::default()
    };
    let forward_only = match CsrGraph::from_edges_with_options([0, 1], [(0, 1)], options) {
        Ok(graph) => graph,
        Err(error) => panic!("valid graph failed to construct: {error}"),
    };
    assert!(!forward_only.has_reverse());
    assert_eq!(forward_only.predecessors(DenseId::from_raw(1)), Ok(None));

    let with_reverse = match forward_only.clone().with_reverse() {
        Ok(graph) => graph,
        Err(error) => panic!("reverse CSR failed to materialize: {error}"),
    };
    assert!(with_reverse.has_reverse());
    assert_eq!(
        with_reverse.predecessors(DenseId::from_raw(1)),
        Ok(Some(&dense(&[0])[..]))
    );
    assert_eq!(
        with_reverse.forward_offsets(),
        forward_only.forward_offsets()
    );
    assert_eq!(
        with_reverse.forward_targets(),
        forward_only.forward_targets()
    );
}

#[test]
fn traversals_are_deterministic_bounded_and_cancellable() {
    let graph = graph();
    assert_eq!(
        graph.breadth_first(DenseId::from_raw(0)),
        Ok(dense(&[0, 1, 2, 4, 3]))
    );
    assert_eq!(
        graph.depth_first_preorder(DenseId::from_raw(0)),
        Ok(dense(&[0, 1, 4, 2, 3]))
    );
    assert_eq!(graph.depth_first_forest(), Ok(dense(&[0, 1, 4, 2, 3, 5])));
    assert_eq!(
        graph.breadth_first_with_control(
            DenseId::from_raw(0),
            ExecutionControl::with_work_limit(0),
        ),
        Err(ComputeError::Incomplete(
            IncompleteReason::WorkLimitExceeded {
                limit: 0,
                consumed: 0,
            }
        ))
    );

    let cancelled = AtomicBool::new(true);
    assert_eq!(
        graph.depth_first_forest_with_control(
            ExecutionControl::unlimited().with_cancellation(&cancelled),
        ),
        Err(ComputeError::Incomplete(IncompleteReason::Cancelled {
            consumed: 0,
        }))
    );
    assert!(matches!(
        graph.breadth_first(DenseId::from_raw(99)),
        Err(ComputeError::Invalid(GraphError::DenseIdOutOfRange { .. }))
    ));
}

#[test]
fn traversal_work_is_one_event_per_reached_vertex_and_scanned_edge() {
    let graph = match CsrGraph::from_edges(0..4, [(0, 1), (0, 2), (0, 3), (2, 3)]) {
        Ok(graph) => graph,
        Err(error) => panic!("valid graph failed to construct: {error}"),
    };
    let exact_work = 8;
    assert_eq!(
        graph.depth_first_preorder_with_control(
            DenseId::from_raw(0),
            ExecutionControl::with_work_limit(exact_work),
        ),
        Ok(dense(&[0, 1, 2, 3]))
    );
    assert_eq!(
        graph.breadth_first_with_control(
            DenseId::from_raw(0),
            ExecutionControl::with_work_limit(exact_work),
        ),
        Ok(dense(&[0, 1, 2, 3]))
    );
    assert_eq!(
        graph.depth_first_forest_with_control(ExecutionControl::with_work_limit(exact_work)),
        Ok(dense(&[0, 1, 2, 3]))
    );
    assert!(matches!(
        graph.depth_first_preorder_with_control(
            DenseId::from_raw(0),
            ExecutionControl::with_work_limit(exact_work - 1),
        ),
        Err(ComputeError::Incomplete(
            IncompleteReason::WorkLimitExceeded { .. }
        ))
    ));
}
