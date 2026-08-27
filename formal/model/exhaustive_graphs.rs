use std::collections::BTreeSet;
use std::thread;

const EXHAUSTIVE_VERTEX_LIMIT: usize = 4;
const SMALL_STACK_BYTES: usize = 256 * 1024;
const DEEP_CHAIN_VERTICES: usize = 20_000;

#[derive(Clone, Copy)]
struct DfsFrame {
    node: usize,
    next_successor: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CanonicalCsr {
    vertex_count: usize,
    forward_offsets: Vec<usize>,
    forward_targets: Vec<usize>,
    reverse_offsets: Vec<usize>,
    reverse_targets: Vec<usize>,
}

impl CanonicalCsr {
    fn from_edges(vertex_count: usize, edges: &[(usize, usize)]) -> Self {
        let forward = canonical_graph(vertex_count, edges);
        let mut reverse_edges = Vec::with_capacity(edges.len());
        for &(source, target) in edges {
            if source < vertex_count && target < vertex_count {
                reverse_edges.push((target, source));
            }
        }
        let reverse = canonical_graph(vertex_count, &reverse_edges);
        let (forward_offsets, forward_targets) = flatten_adjacency(&forward);
        let (reverse_offsets, reverse_targets) = flatten_adjacency(&reverse);
        Self {
            vertex_count,
            forward_offsets,
            forward_targets,
            reverse_offsets,
            reverse_targets,
        }
    }

    fn forward_adjacency(&self) -> Vec<Vec<usize>> {
        expand_adjacency(
            self.vertex_count,
            &self.forward_offsets,
            &self.forward_targets,
        )
    }

    fn is_well_formed(&self) -> bool {
        if !csr_direction_is_well_formed(
            self.vertex_count,
            &self.forward_offsets,
            &self.forward_targets,
        ) || !csr_direction_is_well_formed(
            self.vertex_count,
            &self.reverse_offsets,
            &self.reverse_targets,
        ) {
            return false;
        }

        let mut forward_edges = BTreeSet::new();
        let mut reverse_edges = BTreeSet::new();
        for source in 0..self.vertex_count {
            for &target in &self.forward_targets
                [self.forward_offsets[source]..self.forward_offsets[source + 1]]
            {
                forward_edges.insert((source, target));
            }
            for &predecessor in &self.reverse_targets
                [self.reverse_offsets[source]..self.reverse_offsets[source + 1]]
            {
                reverse_edges.insert((predecessor, source));
            }
        }
        forward_edges == reverse_edges
    }
}

fn flatten_adjacency(adjacency: &[Vec<usize>]) -> (Vec<usize>, Vec<usize>) {
    let target_count: usize = adjacency.iter().map(Vec::len).sum();
    let mut offsets = Vec::with_capacity(adjacency.len() + 1);
    let mut targets = Vec::with_capacity(target_count);
    offsets.push(0);
    for successors in adjacency {
        targets.extend_from_slice(successors);
        offsets.push(targets.len());
    }
    (offsets, targets)
}

fn expand_adjacency(vertex_count: usize, offsets: &[usize], targets: &[usize]) -> Vec<Vec<usize>> {
    assert_eq!(offsets.len(), vertex_count + 1);
    (0..vertex_count)
        .map(|vertex| targets[offsets[vertex]..offsets[vertex + 1]].to_vec())
        .collect()
}

fn csr_direction_is_well_formed(vertex_count: usize, offsets: &[usize], targets: &[usize]) -> bool {
    if offsets.len() != vertex_count + 1
        || offsets.first() != Some(&0)
        || offsets.last() != Some(&targets.len())
        || offsets.windows(2).any(|pair| pair[0] > pair[1])
        || targets.iter().any(|target| *target >= vertex_count)
    {
        return false;
    }
    (0..vertex_count).all(|vertex| {
        targets[offsets[vertex]..offsets[vertex + 1]]
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    })
}

fn canonical_graph(vertex_count: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut adjacency = vec![Vec::new(); vertex_count];
    for &(source, target) in edges {
        assert!(
            source < vertex_count && target < vertex_count,
            "an edge endpoint must belong to the declared vertex domain"
        );
        adjacency[source].push(target);
    }
    for successors in &mut adjacency {
        successors.sort_unstable();
        successors.dedup();
    }
    adjacency
}

fn iterative_tarjan(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    const UNVISITED: usize = usize::MAX;
    const ASSIGNED: usize = usize::MAX - 1;

    let vertex_count = adjacency.len();
    let mut discovery = vec![UNVISITED; vertex_count];
    let mut low_link = vec![0usize; vertex_count];
    let mut next_index = 0usize;
    let mut active = Vec::with_capacity(vertex_count);
    let mut frames = Vec::with_capacity(vertex_count);
    let mut components = Vec::new();

    for start in 0..vertex_count {
        if discovery[start] != UNVISITED {
            continue;
        }
        discovery[start] = next_index;
        low_link[start] = next_index;
        next_index += 1;
        active.push(start);
        frames.push(DfsFrame {
            node: start,
            next_successor: 0,
        });

        while let Some(frame) = frames.last().copied() {
            let node = frame.node;
            if frame.next_successor < adjacency[node].len() {
                let successor = adjacency[node][frame.next_successor];
                frames
                    .last_mut()
                    .expect("the current DFS frame must exist")
                    .next_successor += 1;
                match discovery[successor] {
                    UNVISITED => {
                        discovery[successor] = next_index;
                        low_link[successor] = next_index;
                        next_index += 1;
                        active.push(successor);
                        frames.push(DfsFrame {
                            node: successor,
                            next_successor: 0,
                        });
                    }
                    ASSIGNED => {}
                    successor_index => {
                        low_link[node] = low_link[node].min(successor_index);
                    }
                }
                continue;
            }

            frames.pop();
            if low_link[node] == discovery[node] {
                let mut component = Vec::new();
                loop {
                    let member = active
                        .pop()
                        .expect("an SCC root must remain on the active stack");
                    discovery[member] = ASSIGNED;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                component.sort_unstable();
                components.push(component);
            }
            if let Some(parent) = frames.last() {
                low_link[parent.node] = low_link[parent.node].min(low_link[node]);
            }
        }
    }

    assert!(active.is_empty());
    components.sort_unstable();
    components
}

fn closure_oracle(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let vertex_count = adjacency.len();
    let mut reachable = vec![vec![false; vertex_count]; vertex_count];
    for vertex in 0..vertex_count {
        reachable[vertex][vertex] = true;
        for &successor in &adjacency[vertex] {
            reachable[vertex][successor] = true;
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
        let mut component = Vec::new();
        for target in 0..vertex_count {
            if reachable[source][target] && reachable[target][source] {
                assigned[target] = true;
                component.push(target);
            }
        }
        components.push(component);
    }
    components.sort_unstable();
    components
}

fn component_map(vertex_count: usize, components: &[Vec<usize>]) -> Vec<usize> {
    let mut component_of = vec![usize::MAX; vertex_count];
    for (component, members) in components.iter().enumerate() {
        assert!(!members.is_empty());
        for &member in members {
            assert_eq!(component_of[member], usize::MAX);
            component_of[member] = component;
        }
    }
    assert!(component_of
        .iter()
        .all(|component| *component != usize::MAX));
    component_of
}

fn condensation_edges(
    adjacency: &[Vec<usize>],
    components: &[Vec<usize>],
) -> BTreeSet<(usize, usize)> {
    let component_of = component_map(adjacency.len(), components);
    let mut edges = BTreeSet::new();
    for (source, successors) in adjacency.iter().enumerate() {
        for &target in successors {
            let source_component = component_of[source];
            let target_component = component_of[target];
            if source_component != target_component {
                edges.insert((source_component, target_component));
            }
        }
    }
    edges
}

fn topological_levels(component_count: usize, edges: &BTreeSet<(usize, usize)>) -> Vec<usize> {
    let mut successors = vec![Vec::new(); component_count];
    let mut indegree = vec![0usize; component_count];
    for &(source, target) in edges {
        successors[source].push(target);
        indegree[target] += 1;
    }
    let mut ready: BTreeSet<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(component, count)| (*count == 0).then_some(component))
        .collect();
    let mut levels = vec![0usize; component_count];
    let mut visited = 0usize;
    while let Some(component) = ready.pop_first() {
        visited += 1;
        for &successor in &successors[component] {
            levels[successor] = levels[successor].max(levels[component] + 1);
            indegree[successor] -= 1;
            if indegree[successor] == 0 {
                ready.insert(successor);
            }
        }
    }
    assert_eq!(
        visited, component_count,
        "the SCC condensation must be acyclic"
    );
    for &(source, target) in edges {
        assert!(levels[source] < levels[target]);
    }
    levels
}

fn permutations(vertex_count: usize) -> Vec<Vec<usize>> {
    fn extend(prefix: &mut Vec<usize>, remaining: &mut Vec<usize>, output: &mut Vec<Vec<usize>>) {
        if remaining.is_empty() {
            output.push(prefix.clone());
            return;
        }
        for index in 0..remaining.len() {
            let value = remaining.remove(index);
            prefix.push(value);
            extend(prefix, remaining, output);
            prefix.pop();
            remaining.insert(index, value);
        }
    }

    let mut output = Vec::new();
    extend(
        &mut Vec::with_capacity(vertex_count),
        &mut (0..vertex_count).collect(),
        &mut output,
    );
    output
}

fn rename_components(components: &[Vec<usize>], permutation: &[usize]) -> Vec<Vec<usize>> {
    let mut renamed: Vec<Vec<usize>> = components
        .iter()
        .map(|component| {
            let mut members: Vec<usize> = component
                .iter()
                .map(|vertex| permutation[*vertex])
                .collect();
            members.sort_unstable();
            members
        })
        .collect();
    renamed.sort_unstable();
    renamed
}

fn graph_edges(vertex_count: usize, mask: usize) -> Vec<(usize, usize)> {
    let mut edges = Vec::with_capacity((vertex_count * vertex_count) / 2);
    for source in 0..vertex_count {
        for target in 0..vertex_count {
            let bit = source * vertex_count + target;
            if mask & (1usize << bit) != 0 {
                edges.push((source, target));
            }
        }
    }
    edges
}

fn verify_graph(vertex_count: usize, edges: &[(usize, usize)], rename_cases: &mut u64) {
    let csr = CanonicalCsr::from_edges(vertex_count, edges);
    assert!(csr.is_well_formed());
    let adjacency = csr.forward_adjacency();
    let mut adversarial = edges.to_vec();
    adversarial.reverse();
    adversarial.extend(edges.iter().rev().copied());
    assert_eq!(csr, CanonicalCsr::from_edges(vertex_count, &adversarial));

    let tarjan = iterative_tarjan(&adjacency);
    let oracle = closure_oracle(&adjacency);
    assert_eq!(tarjan, oracle);

    let condensation = condensation_edges(&adjacency, &tarjan);
    let levels = topological_levels(tarjan.len(), &condensation);
    for left in 0..tarjan.len() {
        for right in 0..tarjan.len() {
            if levels[left] == levels[right] {
                assert!(!condensation.contains(&(left, right)));
                assert!(!condensation.contains(&(right, left)));
            }
        }
    }

    for permutation in permutations(vertex_count) {
        let renamed_edges: Vec<(usize, usize)> = edges
            .iter()
            .map(|&(source, target)| (permutation[source], permutation[target]))
            .collect();
        let renamed_graph = canonical_graph(vertex_count, &renamed_edges);
        let renamed_components = iterative_tarjan(&renamed_graph);
        assert_eq!(renamed_components, rename_components(&tarjan, &permutation));

        let old_component_of = component_map(vertex_count, &tarjan);
        let renamed_component_of = component_map(vertex_count, &renamed_components);
        let mut component_renaming = vec![usize::MAX; tarjan.len()];
        for (old_component, members) in tarjan.iter().enumerate() {
            let representative = *members
                .first()
                .expect("an SCC must contain a representative vertex");
            let renamed_component = renamed_component_of[permutation[representative]];
            for &member in members {
                assert_eq!(
                    renamed_component_of[permutation[member]], renamed_component,
                    "a vertex renaming must map one SCC fiber into one SCC fiber"
                );
            }
            component_renaming[old_component] = renamed_component;
        }
        let renamed_component_set: BTreeSet<usize> = component_renaming.iter().copied().collect();
        assert_eq!(renamed_component_set.len(), tarjan.len());
        for vertex in 0..vertex_count {
            assert_eq!(
                component_renaming[old_component_of[vertex]],
                renamed_component_of[permutation[vertex]]
            );
        }

        let expected_renamed_condensation: BTreeSet<(usize, usize)> = condensation
            .iter()
            .map(|&(source, target)| (component_renaming[source], component_renaming[target]))
            .collect();
        let actual_renamed_condensation = condensation_edges(&renamed_graph, &renamed_components);
        assert_eq!(actual_renamed_condensation, expected_renamed_condensation);

        let renamed_levels =
            topological_levels(renamed_components.len(), &actual_renamed_condensation);
        for old_component in 0..tarjan.len() {
            assert_eq!(
                levels[old_component],
                renamed_levels[component_renaming[old_component]]
            );
        }
        *rename_cases += 1;
    }
}

fn verify_deep_small_stack() {
    let worker = thread::Builder::new()
        .name("vinary-graph-kernel-formal-model".into())
        .stack_size(SMALL_STACK_BYTES)
        .spawn(|| {
            let mut edges = Vec::with_capacity(DEEP_CHAIN_VERTICES.saturating_sub(1));
            for vertex in 0..DEEP_CHAIN_VERTICES.saturating_sub(1) {
                edges.push((vertex, vertex + 1));
            }
            let graph = canonical_graph(DEEP_CHAIN_VERTICES, &edges);
            let components = iterative_tarjan(&graph);
            assert_eq!(components.len(), DEEP_CHAIN_VERTICES);
            let condensation = condensation_edges(&graph, &components);
            assert_eq!(condensation.len(), DEEP_CHAIN_VERTICES - 1);
        })
        .expect("the small-stack formal-model worker must spawn");
    worker
        .join()
        .expect("the iterative formal model must complete on a 256 KiB stack");
}

fn main() {
    let mut graph_cases = 0u64;
    let mut rename_cases = 0u64;
    for vertex_count in 0..=EXHAUSTIVE_VERTEX_LIMIT {
        let graph_count = 1usize << (vertex_count * vertex_count);
        for mask in 0..graph_count {
            let edges = graph_edges(vertex_count, mask);
            verify_graph(vertex_count, &edges, &mut rename_cases);
            graph_cases += 1;
        }
    }
    verify_deep_small_stack();
    println!(
        "verified {graph_cases} directed graphs, {rename_cases} renaming cases, and a {DEEP_CHAIN_VERTICES}-vertex 256 KiB-stack chain"
    );
}
