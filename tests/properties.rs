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

fn permutation_from_priorities(priorities: &[u32]) -> Vec<usize> {
    let mut permutation: Vec<usize> = (0..priorities.len()).collect();
    permutation.sort_unstable_by_key(|&vertex| (priorities[vertex], vertex));
    let mut renamed = vec![0usize; priorities.len()];
    for (new_vertex, old_vertex) in permutation.into_iter().enumerate() {
        renamed[old_vertex] = new_vertex;
    }
    renamed
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

    #[test]
    fn lawful_vertex_renaming_preserves_fibers_quotient_and_wave_ranks(
        (n, selected, priorities) in (0usize..9).prop_flat_map(|n| {
            (
                Just(n),
                prop::collection::vec(any::<bool>(), n * n),
                prop::collection::vec(any::<u32>(), n),
            )
        }),
    ) {
        let graph = graph_from(n, &selected);
        let original = SccDecomposition::compute(&graph)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let original_schedule = original.condensation().schedule()
            .map_err(|error| TestCaseError::fail(error.to_string()))?;

        let permutation = permutation_from_priorities(&priorities);
        let renamed_edges = selected
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(bit, set)| {
                set.then_some((
                    u32_index(permutation[bit / n]),
                    u32_index(permutation[bit % n]),
                ))
            });
        let renamed_graph = CsrGraph::from_edges(0..u32_index(n), renamed_edges)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let renamed = SccDecomposition::compute(&renamed_graph)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let renamed_schedule = renamed.condensation().schedule()
            .map_err(|error| TestCaseError::fail(error.to_string()))?;

        prop_assert_eq!(original.component_count(), renamed.component_count());
        let mut component_renaming = vec![usize::MAX; original.component_count()];
        let mut renamed_components = BTreeSet::new();
        for (component, fiber) in original.fibers() {
            let representative = usize_index(fiber[0].get());
            let renamed_component = renamed.component_of(DenseId::from_raw(u32_index(
                permutation[representative],
            )))?;
            for member in fiber {
                prop_assert_eq!(
                    renamed.component_of(DenseId::from_raw(u32_index(
                        permutation[usize_index(member.get())],
                    )))?,
                    renamed_component,
                );
            }
            component_renaming[usize_index(component.id().get())] =
                usize_index(renamed_component.get());
            renamed_components.insert(usize_index(renamed_component.get()));
            prop_assert_eq!(
                component.is_cyclic(),
                renamed.component(renamed_component)?.is_cyclic(),
            );
        }
        prop_assert_eq!(renamed_components.len(), original.component_count());

        let expected_quotient: BTreeSet<_> = original
            .condensation()
            .edges()
            .map(|(source, target)| {
                (
                    component_renaming[usize_index(source.get())],
                    component_renaming[usize_index(target.get())],
                )
            })
            .collect();
        let actual_quotient: BTreeSet<_> = renamed
            .condensation()
            .edges()
            .map(|(source, target)| {
                (usize_index(source.get()), usize_index(target.get()))
            })
            .collect();
        prop_assert_eq!(actual_quotient, expected_quotient);

        for (source, &target) in component_renaming.iter().enumerate() {
            prop_assert_eq!(
                original_schedule.rank(libvgraph::ComponentId::from_raw(u32_index(source))),
                renamed_schedule.rank(libvgraph::ComponentId::from_raw(u32_index(target))),
            );
        }
        prop_assert_eq!(original_schedule.wave_count(), renamed_schedule.wave_count());
        for wave in 0..original_schedule.wave_count() {
            let mut expected: Vec<usize> = original_schedule.wave(wave)
                .expect("an in-range wave must exist")
                .iter()
                .map(|component| component_renaming[usize_index(component.get())])
                .collect();
            expected.sort_unstable();
            let actual: Vec<usize> = renamed_schedule.wave(wave)
                .expect("an in-range renamed wave must exist")
                .iter()
                .map(|component| usize_index(component.get()))
                .collect();
            prop_assert_eq!(actual, expected);
        }
    }
}

fn u32_index(value: usize) -> u32 {
    match u32::try_from(value) {
        Ok(value) => value,
        Err(error) => panic!("test index does not fit u32: {error}"),
    }
}

fn usize_index(value: u32) -> usize {
    match usize::try_from(value) {
        Ok(value) => value,
        Err(error) => panic!("test index does not fit usize: {error}"),
    }
}
