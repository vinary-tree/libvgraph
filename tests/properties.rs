use std::collections::BTreeSet;

use libvgraph::{CsrGraph, DenseId, SccDecomposition};
use petgraph::algo::kosaraju_scc;
use petgraph::graph::DiGraph;
use proptest::prelude::*;

fn graph_from(n: usize, selected: &[bool]) -> CsrGraph<u32> {
    let n_u32 = u32_index(n);
    let edges = selected
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(bit, set)| set.then_some((u32_index(bit / n), u32_index(bit % n))));
    match CsrGraph::from_edges(0..n_u32, edges) {
        Ok(graph) => graph,
        Err(error) => panic!("generated graph failed to construct: {error}"),
    }
}

fn kernel_components(graph: &CsrGraph<u32>) -> Vec<Vec<u32>> {
    let decomposition = match SccDecomposition::compute(graph) {
        Ok(value) => value,
        Err(error) => panic!("generated graph failed to decompose: {error}"),
    };
    decomposition
        .fibers()
        .map(|(_, fiber)| fiber.iter().map(|id| id.get()).collect())
        .collect()
}

fn petgraph_components(n: usize, selected: &[bool]) -> Vec<Vec<u32>> {
    let mut graph = DiGraph::<(), ()>::new();
    let nodes: Vec<_> = (0..n).map(|_| graph.add_node(())).collect();
    for (bit, set) in selected.iter().copied().enumerate() {
        if set {
            graph.add_edge(nodes[bit / n], nodes[bit % n], ());
        }
    }
    let mut components: Vec<Vec<u32>> = kosaraju_scc(&graph)
        .into_iter()
        .map(|component| {
            let mut members: Vec<u32> = component
                .into_iter()
                .map(|node| u32_index(node.index()))
                .collect();
            members.sort_unstable();
            members
        })
        .collect();
    components.sort_unstable();
    components
}

proptest! {
    #[test]
    fn agrees_with_petgraph_and_preserves_quotient_laws(
        (n, selected) in (0usize..9).prop_flat_map(|n| {
            (Just(n), prop::collection::vec(any::<bool>(), n * n))
        }),
    ) {
        let graph = graph_from(n, &selected);
        prop_assert_eq!(kernel_components(&graph), petgraph_components(n, &selected));

        let decomposition = SccDecomposition::compute(&graph)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let schedule = decomposition
            .condensation()
            .schedule()
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let quotient_edges: BTreeSet<_> = decomposition.condensation().edges().collect();
        for (source, target) in quotient_edges {
            prop_assert!(schedule.rank(source) < schedule.rank(target));
        }
        for vertex in 0..u32_index(n) {
            let component = decomposition
                .component_of(DenseId::from_raw(vertex))
                .map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert!(decomposition
                .fiber(component)
                .map_err(|error| TestCaseError::fail(error.to_string()))?
                .contains(&DenseId::from_raw(vertex)));
        }
    }

    #[test]
    fn input_permutation_and_duplicates_do_not_change_canonical_graph(
        (n, selected) in (0usize..9).prop_flat_map(|n| {
            (Just(n), prop::collection::vec(any::<bool>(), n * n))
        }),
    ) {
        let canonical = graph_from(n, &selected);
        let n_u32 = u32_index(n);
        let mut nodes: Vec<u32> = (0..n_u32).rev().collect();
        nodes.extend((0..n_u32).rev());
        let mut edges: Vec<(u32, u32)> = selected
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(bit, set)| {
                set.then_some((u32_index(bit / n), u32_index(bit % n)))
            })
            .collect();
        edges.reverse();
        let duplicates = edges.clone();
        edges.extend(duplicates);
        let permuted = CsrGraph::from_edges(nodes, edges)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(permuted, canonical);
    }
}

fn u32_index(value: usize) -> u32 {
    match u32::try_from(value) {
        Ok(value) => value,
        Err(error) => panic!("test index does not fit u32: {error}"),
    }
}
