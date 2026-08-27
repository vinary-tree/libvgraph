use libvgraph::{CsrGraph, DenseId, SccDecomposition};

fn closure_components(vertex_count: usize, mask: u32) -> Vec<Vec<u32>> {
    let mut reachable = vec![vec![false; vertex_count]; vertex_count];
    for (source, row) in reachable.iter_mut().enumerate() {
        row[source] = true;
        for (target, cell) in row.iter_mut().enumerate() {
            let bit = source * vertex_count + target;
            if mask & (1 << bit) != 0 {
                *cell = true;
            }
        }
    }
    for middle in 0..vertex_count {
        for source in 0..vertex_count {
            for target in 0..vertex_count {
                reachable[source][target] |= reachable[source][middle] && reachable[middle][target];
            }
        }
    }

    let mut assigned = vec![false; vertex_count];
    let mut components = Vec::new();
    for source in 0..vertex_count {
        if assigned[source] {
            continue;
        }
        let mut members = Vec::new();
        for target in 0..vertex_count {
            if reachable[source][target] && reachable[target][source] {
                assigned[target] = true;
                members.push(u32_index(target));
            }
        }
        components.push(members);
    }
    components
}

#[test]
fn all_graphs_through_four_vertices_refine_the_closure_oracle() {
    let mut graph_count = 0u64;
    for vertex_count in 0usize..=4 {
        let vertex_count_u32 = u32_index(vertex_count);
        let masks = 1u32 << (vertex_count * vertex_count);
        for mask in 0..masks {
            let edges = (0..vertex_count).flat_map(|source| {
                (0..vertex_count).filter_map(move |target| {
                    let bit = source * vertex_count + target;
                    (mask & (1 << bit) != 0).then_some((u32_index(source), u32_index(target)))
                })
            });
            let graph = match CsrGraph::from_edges(0..vertex_count_u32, edges) {
                Ok(graph) => graph,
                Err(error) => panic!("exhaustive graph {vertex_count}/{mask} failed: {error}"),
            };
            let decomposition = match SccDecomposition::compute(&graph) {
                Ok(value) => value,
                Err(error) => panic!("exhaustive SCC {vertex_count}/{mask} failed: {error}"),
            };
            let actual: Vec<Vec<u32>> = decomposition
                .fibers()
                .map(|(_, fiber)| fiber.iter().map(|id| id.get()).collect())
                .collect();
            assert_eq!(actual, closure_components(vertex_count, mask));
            assert_eq!(decomposition.validate(&graph), Ok(()));

            let schedule = match decomposition.condensation().schedule() {
                Ok(value) => value,
                Err(error) => panic!("exhaustive schedule {vertex_count}/{mask} failed: {error}"),
            };
            for (source, target) in decomposition.condensation().edges() {
                assert!(schedule.rank(source) < schedule.rank(target));
            }
            for vertex in 0..vertex_count_u32 {
                let vertex = DenseId::from_raw(vertex);
                let component = match decomposition.component_of(vertex) {
                    Ok(component) => component,
                    Err(error) => panic!("total component lookup failed: {error}"),
                };
                assert!(match decomposition.fiber(component) {
                    Ok(fiber) => fiber.contains(&vertex),
                    Err(error) => panic!("fiber lookup failed: {error}"),
                });
            }
            graph_count += 1;
        }
    }
    assert_eq!(graph_count, 66_067);
}

fn u32_index(value: usize) -> u32 {
    match u32::try_from(value) {
        Ok(value) => value,
        Err(error) => panic!("test index does not fit u32: {error}"),
    }
}
