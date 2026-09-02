use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::thread;

const EXHAUSTIVE_VERTEX_LIMIT: usize = 3;
const DEEP_CHAIN_VERTICES: usize = 20_000;
const SMALL_STACK_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CanonicalCsr {
    vertex_count: usize,
    offsets: Vec<usize>,
    targets: Vec<usize>,
    edges: Vec<(usize, usize)>,
}

impl CanonicalCsr {
    fn from_edges(vertex_count: usize, input: &[(usize, usize)]) -> Self {
        let mut edges = input.to_vec();
        assert!(
            edges
                .iter()
                .all(|&(source, target)| source < vertex_count && target < vertex_count),
            "every edge endpoint must belong to the declared vertex domain"
        );
        edges.sort_unstable();
        edges.dedup();

        let mut offsets = vec![0usize; vertex_count + 1];
        for &(source, _) in &edges {
            offsets[source + 1] += 1;
        }
        for vertex in 0..vertex_count {
            offsets[vertex + 1] += offsets[vertex];
        }
        let targets = edges.iter().map(|&(_, target)| target).collect();
        let graph = Self {
            vertex_count,
            offsets,
            targets,
            edges,
        };
        assert!(graph.is_well_formed());
        graph
    }

    fn is_well_formed(&self) -> bool {
        self.offsets.len() == self.vertex_count + 1
            && self.offsets.first() == Some(&0)
            && self.offsets.last() == Some(&self.targets.len())
            && self.offsets.windows(2).all(|pair| pair[0] <= pair[1])
            && self
                .targets
                .iter()
                .all(|&target| target < self.vertex_count)
            && self.edges.len() == self.targets.len()
            && self.edges.windows(2).all(|pair| pair[0] < pair[1])
            && (0..self.vertex_count).all(|source| {
                let row = self.row(source);
                row.windows(2).all(|pair| pair[0] < pair[1])
                    && self.edges[self.offsets[source]..self.offsets[source + 1]]
                        .iter()
                        .all(|&(edge_source, _)| edge_source == source)
            })
    }

    fn row_range(&self, source: usize) -> std::ops::Range<usize> {
        self.offsets[source]..self.offsets[source + 1]
    }

    fn row(&self, source: usize) -> &[usize] {
        &self.targets[self.row_range(source)]
    }

    fn edge_index(&self, source: usize, target: usize) -> Option<usize> {
        self.edges.binary_search(&(source, target)).ok()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FlatSidecar {
    offsets: Vec<usize>,
    members: Vec<u32>,
}

impl FlatSidecar {
    fn from_slots(edge_count: usize, slots: &[Vec<u32>]) -> Self {
        assert_eq!(slots.len(), edge_count);
        let total_members = slots.iter().map(Vec::len).sum();
        let mut offsets = Vec::with_capacity(edge_count + 1);
        let mut members = Vec::with_capacity(total_members);
        offsets.push(0);
        for slot in slots {
            let mut canonical = slot.clone();
            canonical.sort_unstable();
            canonical.dedup();
            members.extend(canonical);
            offsets.push(members.len());
        }
        let sidecar = Self { offsets, members };
        assert!(sidecar.is_well_formed(edge_count));
        sidecar
    }

    fn is_well_formed(&self, edge_count: usize) -> bool {
        self.offsets.len() == edge_count + 1
            && self.offsets.first() == Some(&0)
            && self.offsets.last() == Some(&self.members.len())
            && self.offsets.windows(2).all(|pair| pair[0] <= pair[1])
            && (0..edge_count).all(|edge| self.slot(edge).windows(2).all(|pair| pair[0] < pair[1]))
    }

    fn slot(&self, edge: usize) -> &[u32] {
        &self.members[self.offsets[edge]..self.offsets[edge + 1]]
    }

    fn union(&self, other: &Self) -> Self {
        assert_eq!(self.offsets.len(), other.offsets.len());
        let edge_count = self.offsets.len() - 1;
        let mut offsets = Vec::with_capacity(edge_count + 1);
        let mut members = Vec::with_capacity(self.members.len() + other.members.len());
        offsets.push(0);
        for edge in 0..edge_count {
            let left = self.slot(edge);
            let right = other.slot(edge);
            let (mut left_index, mut right_index) = (0usize, 0usize);
            while left_index < left.len() || right_index < right.len() {
                let value = match (left.get(left_index), right.get(right_index)) {
                    (Some(&left_value), Some(&right_value)) if left_value < right_value => {
                        left_index += 1;
                        left_value
                    }
                    (Some(&left_value), Some(&right_value)) if right_value < left_value => {
                        right_index += 1;
                        right_value
                    }
                    (Some(&value), Some(_)) => {
                        left_index += 1;
                        right_index += 1;
                        value
                    }
                    (Some(&value), None) => {
                        left_index += 1;
                        value
                    }
                    (None, Some(&value)) => {
                        right_index += 1;
                        value
                    }
                    (None, None) => unreachable!("the merge loop has no remaining value"),
                };
                members.push(value);
            }
            offsets.push(members.len());
        }
        let union = Self { offsets, members };
        assert!(union.is_well_formed(edge_count));
        union
    }

    fn transported(&self, edge_renaming: &[usize]) -> Self {
        let edge_count = edge_renaming.len();
        let mut slots = vec![Vec::new(); edge_count];
        for (old_edge, &new_edge) in edge_renaming.iter().enumerate() {
            slots[new_edge] = self.slot(old_edge).to_vec();
        }
        Self::from_slots(edge_count, &slots)
    }
}

#[derive(Clone, Debug)]
struct Reachability {
    root: usize,
    discovered: Vec<bool>,
    parent_edge: Vec<Option<usize>>,
    edge_scans: usize,
}

impl Reachability {
    fn search(graph: &CanonicalCsr, root: usize) -> Self {
        let mut discovered = vec![false; graph.vertex_count];
        let mut parent_edge = vec![None; graph.vertex_count];
        let mut queue = Vec::with_capacity(graph.vertex_count);
        let mut head = 0usize;
        let mut edge_scans = 0usize;
        discovered[root] = true;
        queue.push(root);
        while head < queue.len() {
            let source = queue[head];
            head += 1;
            for edge in graph.row_range(source) {
                edge_scans += 1;
                let target = graph.targets[edge];
                if !discovered[target] {
                    discovered[target] = true;
                    parent_edge[target] = Some(edge);
                    queue.push(target);
                }
            }
        }
        Self {
            root,
            discovered,
            parent_edge,
            edge_scans,
        }
    }

    fn path_to(&self, graph: &CanonicalCsr, target: usize) -> Option<Vec<usize>> {
        if !self.discovered[target] {
            return None;
        }
        let mut reverse = Vec::with_capacity(graph.vertex_count);
        let mut current = target;
        while current != self.root {
            let edge = self.parent_edge[current]
                .expect("every discovered non-root vertex must have a parent edge");
            assert_eq!(graph.edges[edge].1, current);
            reverse.push(edge);
            current = graph.edges[edge].0;
        }
        reverse.reverse();
        Some(reverse)
    }
}

fn replay(graph: &CanonicalCsr, source: usize, path: &[usize]) -> Option<usize> {
    let mut current = source;
    for &edge in path {
        let &(edge_source, edge_target) = graph.edges.get(edge)?;
        if edge_source != current {
            return None;
        }
        current = edge_target;
    }
    Some(current)
}

fn closure_and_distances(graph: &CanonicalCsr) -> (Vec<Vec<bool>>, Vec<Vec<usize>>) {
    let count = graph.vertex_count;
    let mut reachable = vec![vec![false; count]; count];
    let mut distance = vec![vec![usize::MAX; count]; count];
    for vertex in 0..count {
        reachable[vertex][vertex] = true;
        distance[vertex][vertex] = 0;
    }
    for &(source, target) in &graph.edges {
        reachable[source][target] = true;
        distance[source][target] = distance[source][target].min(1);
    }
    for middle in 0..count {
        for source in 0..count {
            for target in 0..count {
                reachable[source][target] |= reachable[source][middle] && reachable[middle][target];
                if distance[source][middle] != usize::MAX && distance[middle][target] != usize::MAX
                {
                    distance[source][target] = distance[source][target]
                        .min(distance[source][middle] + distance[middle][target]);
                }
            }
        }
    }
    (reachable, distance)
}

fn strongly_connected_components(reachable: &[Vec<bool>]) -> (Vec<Vec<usize>>, Vec<usize>) {
    let count = reachable.len();
    let mut assigned = vec![false; count];
    let mut components = Vec::new();
    let mut component_of = vec![usize::MAX; count];
    for source in 0..count {
        if assigned[source] {
            continue;
        }
        let component = components.len();
        let mut members = Vec::new();
        for target in 0..count {
            if reachable[source][target] && reachable[target][source] {
                assigned[target] = true;
                component_of[target] = component;
                members.push(target);
            }
        }
        components.push(members);
    }
    assert!(component_of
        .iter()
        .all(|&component| component != usize::MAX));
    (components, component_of)
}

fn condensation_witness_fibers(
    graph: &CanonicalCsr,
    component_of: &[usize],
) -> BTreeMap<(usize, usize), Vec<usize>> {
    let mut fibers: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (edge, &(source, target)) in graph.edges.iter().enumerate() {
        let pair = (component_of[source], component_of[target]);
        if pair.0 != pair.1 {
            fibers.entry(pair).or_default().push(edge);
        }
    }
    fibers
}

fn reachable_avoiding(
    graph: &CanonicalCsr,
    root: usize,
    target: usize,
    blocked: Option<usize>,
) -> bool {
    if blocked == Some(root) || blocked == Some(target) {
        return false;
    }
    let mut discovered = vec![false; graph.vertex_count];
    let mut queue = VecDeque::with_capacity(graph.vertex_count);
    discovered[root] = true;
    queue.push_back(root);
    while let Some(source) = queue.pop_front() {
        if source == target {
            return true;
        }
        for &successor in graph.row(source) {
            if blocked != Some(successor) && !discovered[successor] {
                discovered[successor] = true;
                queue.push_back(successor);
            }
        }
    }
    false
}

fn dominators_oracle(graph: &CanonicalCsr, root: usize) -> Vec<BTreeSet<usize>> {
    let search = Reachability::search(graph, root);
    let mut result = vec![BTreeSet::new(); graph.vertex_count];
    for target in 0..graph.vertex_count {
        if !search.discovered[target] {
            continue;
        }
        for candidate in 0..graph.vertex_count {
            let dominates = candidate == root
                || candidate == target
                || !reachable_avoiding(graph, root, target, Some(candidate));
            if dominates {
                result[target].insert(candidate);
            }
        }
    }
    result
}

#[derive(Clone, Debug)]
struct DominatorTree {
    root: usize,
    reachable: Vec<bool>,
    immediate: Vec<Option<usize>>,
    eval_steps: usize,
}

impl DominatorTree {
    fn lengauer_tarjan(graph: &CanonicalCsr, root: usize) -> Self {
        let count = graph.vertex_count;
        let unvisited = usize::MAX;
        let mut dfs_number = vec![unvisited; count];
        let mut vertex = vec![0usize];
        let mut parent = vec![0usize];
        let mut frames: Vec<(usize, usize)> = Vec::with_capacity(count);

        dfs_number[root] = 1;
        vertex.push(root);
        parent.push(0);
        frames.push((root, 0));
        while let Some((node, next_offset)) = frames.last_mut() {
            let row = graph.row_range(*node);
            if *next_offset < row.len() {
                let edge = row.start + *next_offset;
                *next_offset += 1;
                let successor = graph.targets[edge];
                if dfs_number[successor] == unvisited {
                    let number = vertex.len();
                    dfs_number[successor] = number;
                    vertex.push(successor);
                    parent.push(dfs_number[*node]);
                    frames.push((successor, 0));
                }
            } else {
                frames.pop();
            }
        }

        let reachable_count = vertex.len() - 1;
        let mut predecessors = vec![Vec::new(); reachable_count + 1];
        for &(source, target) in &graph.edges {
            if dfs_number[source] != unvisited && dfs_number[target] != unvisited {
                predecessors[dfs_number[target]].push(dfs_number[source]);
            }
        }

        let mut semi: Vec<usize> = (0..=reachable_count).collect();
        let mut label: Vec<usize> = (0..=reachable_count).collect();
        let mut ancestor = vec![0usize; reachable_count + 1];
        let mut immediate_number = vec![0usize; reachable_count + 1];
        let mut buckets = vec![Vec::new(); reachable_count + 1];
        let mut compression_path = Vec::with_capacity(reachable_count);
        let mut eval_steps = 0usize;

        fn evaluate(
            vertex_number: usize,
            ancestor: &mut [usize],
            label: &mut [usize],
            semi: &[usize],
            path: &mut Vec<usize>,
            steps: &mut usize,
        ) -> usize {
            if ancestor[vertex_number] == 0 {
                return label[vertex_number];
            }
            path.clear();
            let mut cursor = vertex_number;
            while ancestor[cursor] != 0 && ancestor[ancestor[cursor]] != 0 {
                path.push(cursor);
                cursor = ancestor[cursor];
                *steps += 1;
            }
            for &node in path.iter().rev() {
                let parent_node = ancestor[node];
                if semi[label[parent_node]] < semi[label[node]] {
                    label[node] = label[parent_node];
                }
                ancestor[node] = ancestor[parent_node];
                *steps += 1;
            }
            label[vertex_number]
        }

        for number in (2..=reachable_count).rev() {
            for &predecessor in &predecessors[number] {
                let evaluated = evaluate(
                    predecessor,
                    &mut ancestor,
                    &mut label,
                    &semi,
                    &mut compression_path,
                    &mut eval_steps,
                );
                semi[number] = semi[number].min(semi[evaluated]);
            }
            buckets[semi[number]].push(number);
            ancestor[number] = parent[number];
            let parent_number = parent[number];
            let pending = std::mem::take(&mut buckets[parent_number]);
            for bucket_vertex in pending {
                let evaluated = evaluate(
                    bucket_vertex,
                    &mut ancestor,
                    &mut label,
                    &semi,
                    &mut compression_path,
                    &mut eval_steps,
                );
                immediate_number[bucket_vertex] = if semi[evaluated] < semi[bucket_vertex] {
                    evaluated
                } else {
                    parent_number
                };
            }
        }
        for number in 2..=reachable_count {
            if immediate_number[number] != semi[number] {
                immediate_number[number] = immediate_number[immediate_number[number]];
            }
        }
        if reachable_count != 0 {
            immediate_number[1] = 1;
        }

        let mut reachable = vec![false; count];
        let mut immediate = vec![None; count];
        for number in 1..=reachable_count {
            let node = vertex[number];
            reachable[node] = true;
            immediate[node] = if number == 1 {
                None
            } else {
                Some(vertex[immediate_number[number]])
            };
        }
        Self {
            root,
            reachable,
            immediate,
            eval_steps,
        }
    }

    fn dominator_sets(&self) -> Vec<BTreeSet<usize>> {
        let count = self.reachable.len();
        let mut sets = vec![BTreeSet::new(); count];
        for target in 0..count {
            if !self.reachable[target] {
                continue;
            }
            let mut current = target;
            sets[target].insert(current);
            while current != self.root {
                current = self.immediate[current]
                    .expect("every reachable non-root vertex has an immediate dominator");
                assert!(
                    sets[target].insert(current),
                    "the dominator tree must be acyclic"
                );
            }
        }
        sets
    }

    fn frontiers(&self, graph: &CanonicalCsr) -> Vec<BTreeSet<usize>> {
        let count = graph.vertex_count;
        let mut children = vec![Vec::new(); count];
        for node in 0..count {
            if let Some(parent) = self.immediate[node] {
                children[parent].push(node);
            }
        }
        let mut preorder = Vec::with_capacity(count);
        let mut stack = vec![self.root];
        while let Some(node) = stack.pop() {
            preorder.push(node);
            for &child in children[node].iter().rev() {
                stack.push(child);
            }
        }

        let mut frontiers = vec![BTreeSet::new(); count];
        for &node in preorder.iter().rev() {
            for &successor in graph.row(node) {
                if self.reachable[successor] && self.immediate[successor] != Some(node) {
                    frontiers[node].insert(successor);
                }
            }
            for &child in &children[node] {
                let child_frontier: Vec<usize> = frontiers[child].iter().copied().collect();
                for member in child_frontier {
                    if self.immediate[member] != Some(node) {
                        frontiers[node].insert(member);
                    }
                }
            }
        }
        frontiers
    }
}

fn frontiers_from_definition(
    graph: &CanonicalCsr,
    dominators: &[BTreeSet<usize>],
) -> Vec<BTreeSet<usize>> {
    let count = graph.vertex_count;
    let mut predecessors = vec![Vec::new(); count];
    for &(source, target) in &graph.edges {
        predecessors[target].push(source);
    }
    let mut frontiers = vec![BTreeSet::new(); count];
    for owner in 0..count {
        for target in 0..count {
            if dominators[target].is_empty() {
                continue;
            }
            let strictly_dominates_target = owner != target && dominators[target].contains(&owner);
            if !strictly_dominates_target
                && predecessors[target]
                    .iter()
                    .any(|&predecessor| dominators[predecessor].contains(&owner))
            {
                frontiers[owner].insert(target);
            }
        }
    }
    frontiers
}

fn next_permutation(values: &mut [usize]) -> bool {
    let Some(pivot) = (1..values.len())
        .rev()
        .find(|&index| values[index - 1] < values[index])
        .map(|index| index - 1)
    else {
        return false;
    };
    let successor = (pivot + 1..values.len())
        .rev()
        .find(|&index| values[pivot] < values[index])
        .expect("a permutation pivot must have a successor");
    values.swap(pivot, successor);
    values[pivot + 1..].reverse();
    true
}

fn permutations(count: usize) -> Vec<Vec<usize>> {
    let mut current: Vec<usize> = (0..count).collect();
    let mut output = Vec::new();
    loop {
        output.push(current.clone());
        if !next_permutation(&mut current) {
            break;
        }
    }
    output
}

fn graph_edges(vertex_count: usize, mask: usize) -> Vec<(usize, usize)> {
    let mut edges = Vec::with_capacity(vertex_count * vertex_count);
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

fn sidecars_for(graph: &CanonicalCsr) -> (FlatSidecar, FlatSidecar) {
    let mut left = Vec::with_capacity(graph.edges.len());
    let mut right = Vec::with_capacity(graph.edges.len());
    for (edge, &(source, target)) in graph.edges.iter().enumerate() {
        left.push(vec![
            (edge % 3) as u32,
            (17 * source + target + 11) as u32,
            (edge % 3) as u32,
        ]);
        right.push(vec![(19 * target + source + 7) as u32, (edge % 5) as u32]);
    }
    (
        FlatSidecar::from_slots(graph.edges.len(), &left),
        FlatSidecar::from_slots(graph.edges.len(), &right),
    )
}

fn verify_graph(
    vertex_count: usize,
    input_edges: &[(usize, usize)],
    root_cases: &mut u64,
    renaming_cases: &mut u64,
) {
    let graph = CanonicalCsr::from_edges(vertex_count, input_edges);
    let mut adversarial = input_edges.to_vec();
    adversarial.reverse();
    adversarial.extend(input_edges.iter().rev().copied());
    assert_eq!(graph, CanonicalCsr::from_edges(vertex_count, &adversarial));

    let (reachable, distances) = closure_and_distances(&graph);
    let (_components, component_of) = strongly_connected_components(&reachable);
    let fibers = condensation_witness_fibers(&graph, &component_of);
    let (left_sidecar, right_sidecar) = sidecars_for(&graph);
    let union = left_sidecar.union(&right_sidecar);
    assert_eq!(union, right_sidecar.union(&left_sidecar));
    assert_eq!(left_sidecar, left_sidecar.union(&left_sidecar));
    assert_eq!(
        left_sidecar.union(&right_sidecar).union(&left_sidecar),
        left_sidecar.union(&right_sidecar.union(&left_sidecar))
    );
    assert!(union.members.len() <= left_sidecar.members.len() + right_sidecar.members.len());

    let mut dominators_by_root = Vec::with_capacity(vertex_count);
    let mut frontiers_by_root = Vec::with_capacity(vertex_count);
    for root in 0..vertex_count {
        let search = Reachability::search(&graph, root);
        assert!(search.edge_scans <= graph.edges.len());
        for target in 0..vertex_count {
            let path = search.path_to(&graph, target);
            assert_eq!(path.is_some(), reachable[root][target]);
            if let Some(path) = path {
                assert_eq!(replay(&graph, root, &path), Some(target));
                assert_eq!(path.len(), distances[root][target]);
                assert!(path.len() <= vertex_count.saturating_sub(1));
            }
        }

        let oracle = dominators_oracle(&graph, root);
        let tree = DominatorTree::lengauer_tarjan(&graph, root);
        let actual = tree.dominator_sets();
        assert_eq!(actual, oracle);
        assert!(
            tree.eval_steps <= 4 * (vertex_count + graph.edges.len() + 1),
            "the bounded corpus must remain within the preregistered link/eval charge"
        );
        let definition_frontiers = frontiers_from_definition(&graph, &oracle);
        let tree_frontiers = tree.frontiers(&graph);
        assert_eq!(tree_frontiers, definition_frontiers);
        dominators_by_root.push(oracle);
        frontiers_by_root.push(definition_frontiers);
        *root_cases += 1;
    }

    for permutation in permutations(vertex_count) {
        let renamed_edges: Vec<(usize, usize)> = graph
            .edges
            .iter()
            .map(|&(source, target)| (permutation[source], permutation[target]))
            .collect();
        let renamed = CanonicalCsr::from_edges(vertex_count, &renamed_edges);
        let (renamed_reachable, renamed_distances) = closure_and_distances(&renamed);
        let (_renamed_components, renamed_component_of) =
            strongly_connected_components(&renamed_reachable);
        let renamed_fibers = condensation_witness_fibers(&renamed, &renamed_component_of);

        let mut edge_renaming = vec![usize::MAX; graph.edges.len()];
        for (old_edge, &(source, target)) in graph.edges.iter().enumerate() {
            edge_renaming[old_edge] = renamed
                .edge_index(permutation[source], permutation[target])
                .expect("a vertex bijection must induce an edge bijection");
        }
        let mut component_renaming = vec![
            usize::MAX;
            component_of
                .iter()
                .copied()
                .max()
                .map_or(0, |value| value + 1)
        ];
        for vertex in 0..vertex_count {
            let old_component = component_of[vertex];
            let new_component = renamed_component_of[permutation[vertex]];
            if component_renaming[old_component] == usize::MAX {
                component_renaming[old_component] = new_component;
            } else {
                assert_eq!(component_renaming[old_component], new_component);
            }
        }

        for (&(source_component, target_component), old_fiber) in &fibers {
            let renamed_pair = (
                component_renaming[source_component],
                component_renaming[target_component],
            );
            let mut expected: Vec<usize> =
                old_fiber.iter().map(|&edge| edge_renaming[edge]).collect();
            expected.sort_unstable();
            assert_eq!(renamed_fibers.get(&renamed_pair), Some(&expected));

            let old_choice = *old_fiber
                .iter()
                .min()
                .expect("a quotient witness fiber is nonempty");
            let renamed_choice = *expected
                .iter()
                .min_by_key(|&&edge| {
                    edge_renaming
                        .iter()
                        .position(|&renamed_edge| renamed_edge == edge)
                        .expect("the induced edge map is bijective")
                })
                .expect("a transported witness fiber is nonempty");
            assert_eq!(renamed_choice, edge_renaming[old_choice]);
        }

        let transported_union = union.transported(&edge_renaming);
        assert_eq!(
            transported_union,
            left_sidecar
                .transported(&edge_renaming)
                .union(&right_sidecar.transported(&edge_renaming))
        );

        for root in 0..vertex_count {
            let renamed_root = permutation[root];
            let renamed_search = Reachability::search(&renamed, renamed_root);
            let renamed_dominators = dominators_oracle(&renamed, renamed_root);
            let renamed_frontiers = frontiers_from_definition(&renamed, &renamed_dominators);
            for target in 0..vertex_count {
                let renamed_target = permutation[target];
                assert_eq!(
                    reachable[root][target],
                    renamed_reachable[renamed_root][renamed_target]
                );
                assert_eq!(
                    distances[root][target],
                    renamed_distances[renamed_root][renamed_target]
                );
                if let Some(path) = Reachability::search(&graph, root).path_to(&graph, target) {
                    let mapped_path: Vec<usize> =
                        path.iter().map(|&edge| edge_renaming[edge]).collect();
                    assert_eq!(
                        replay(&renamed, renamed_root, &mapped_path),
                        Some(renamed_target)
                    );
                    assert_eq!(
                        mapped_path.len(),
                        renamed_search
                            .path_to(&renamed, renamed_target)
                            .expect("renamed reachability must be preserved")
                            .len()
                    );
                }

                let expected_dominators: BTreeSet<usize> = dominators_by_root[root][target]
                    .iter()
                    .map(|&vertex| permutation[vertex])
                    .collect();
                assert_eq!(expected_dominators, renamed_dominators[renamed_target]);
            }
            for owner in 0..vertex_count {
                let expected_frontier: BTreeSet<usize> = frontiers_by_root[root][owner]
                    .iter()
                    .map(|&vertex| permutation[vertex])
                    .collect();
                assert_eq!(expected_frontier, renamed_frontiers[permutation[owner]]);
            }
        }
        *renaming_cases += 1;
    }
}

fn verify_negative_controls() {
    let duplicate = FlatSidecar {
        offsets: vec![0, 2],
        members: vec![7, 7],
    };
    assert!(!duplicate.is_well_formed(1));
    for malformed in [
        FlatSidecar {
            offsets: vec![],
            members: vec![],
        },
        FlatSidecar {
            offsets: vec![1, 1],
            members: vec![],
        },
        FlatSidecar {
            offsets: vec![0, 2, 1],
            members: vec![1],
        },
        FlatSidecar {
            offsets: vec![0, 2],
            members: vec![1],
        },
        FlatSidecar {
            offsets: vec![0, 2],
            members: vec![2, 1],
        },
    ] {
        assert!(!malformed.is_well_formed(1));
    }

    let path_graph = CanonicalCsr::from_edges(3, &[(0, 1), (1, 2)]);
    assert_eq!(replay(&path_graph, 0, &[0, 1]), Some(2));
    assert_eq!(replay(&path_graph, 0, &[1]), None);
    assert_eq!(replay(&path_graph, 0, &[usize::MAX]), None);
    assert_ne!(replay(&path_graph, 0, &[0]), Some(2));

    let disconnected = CanonicalCsr::from_edges(3, &[(0, 1)]);
    let dominators = dominators_oracle(&disconnected, 0);
    assert!(dominators[2].is_empty());
    assert!(
        !dominators[2].contains(&2),
        "the unreachable-dominates-itself mutant must be rejected"
    );

    let chain_dominators = dominators_oracle(&path_graph, 0);
    let chain_tree = DominatorTree::lengauer_tarjan(&path_graph, 0);
    assert_eq!(chain_tree.immediate[2], Some(1));
    assert_ne!(
        chain_tree.immediate[2],
        Some(0),
        "the root-is-every-idom mutant must be rejected"
    );
    assert_eq!(chain_tree.dominator_sets(), chain_dominators);

    let self_loop = CanonicalCsr::from_edges(1, &[(0, 0)]);
    let self_dominators = dominators_oracle(&self_loop, 0);
    let self_frontier = frontiers_from_definition(&self_loop, &self_dominators);
    assert!(
        self_frontier[0].contains(&0),
        "the remove-owner-from-frontier mutant must be rejected"
    );
    assert_eq!(
        DominatorTree::lengauer_tarjan(&self_loop, 0).frontiers(&self_loop),
        self_frontier
    );

    let symmetric = CanonicalCsr::from_edges(4, &[(0, 1), (1, 0), (2, 3), (3, 2), (0, 2), (1, 3)]);
    let permutation = [1usize, 0, 3, 2];
    let renamed_edges: Vec<(usize, usize)> = symmetric
        .edges
        .iter()
        .map(|&(source, target)| (permutation[source], permutation[target]))
        .collect();
    let renamed = CanonicalCsr::from_edges(4, &renamed_edges);
    assert_eq!(
        symmetric, renamed,
        "the selected graph has the required automorphism"
    );
    let (_, components) = strongly_connected_components(&closure_and_distances(&symmetric).0);
    let fibers = condensation_witness_fibers(&symmetric, &components);
    let fiber = fibers
        .values()
        .find(|fiber| fiber.len() == 2)
        .expect("the symmetric quotient edge has two source witnesses");
    let numeric_choice = *fiber.iter().min().expect("the fiber is nonempty");
    let mapped_edge = symmetric
        .edge_index(
            permutation[symmetric.edges[numeric_choice].0],
            permutation[symmetric.edges[numeric_choice].1],
        )
        .expect("the automorphism maps the chosen edge");
    assert_ne!(
        numeric_choice, mapped_edge,
        "an unqualified numeric-index selector must fail equivariance"
    );
}

fn verify_deep_small_stack() {
    let worker = thread::Builder::new()
        .name("libvgraph-witness-formal-model".into())
        .stack_size(SMALL_STACK_BYTES)
        .spawn(|| {
            let edges: Vec<(usize, usize)> = (0..DEEP_CHAIN_VERTICES - 1)
                .map(|vertex| (vertex, vertex + 1))
                .collect();
            let graph = CanonicalCsr::from_edges(DEEP_CHAIN_VERTICES, &edges);
            let search = Reachability::search(&graph, 0);
            let path = search
                .path_to(&graph, DEEP_CHAIN_VERTICES - 1)
                .expect("the end of a chain must be reachable");
            assert_eq!(path.len(), DEEP_CHAIN_VERTICES - 1);
            assert_eq!(replay(&graph, 0, &path), Some(DEEP_CHAIN_VERTICES - 1));

            let dominators = DominatorTree::lengauer_tarjan(&graph, 0);
            for vertex in 1..DEEP_CHAIN_VERTICES {
                assert_eq!(dominators.immediate[vertex], Some(vertex - 1));
            }
            let frontiers = dominators.frontiers(&graph);
            assert!(frontiers.iter().all(BTreeSet::is_empty));

            let empty_slots = vec![Vec::<u32>::new(); graph.edges.len()];
            let empty = FlatSidecar::from_slots(graph.edges.len(), &empty_slots);
            assert_eq!(empty, empty.union(&empty));
        })
        .expect("the 256 KiB witness-model thread must spawn");
    worker
        .join()
        .expect("all witness algorithms must complete on the small native stack");
}

fn main() {
    verify_negative_controls();
    let mut graph_cases = 0u64;
    let mut root_cases = 0u64;
    let mut renaming_cases = 0u64;
    for vertex_count in 1..=EXHAUSTIVE_VERTEX_LIMIT {
        let graph_count = 1usize << (vertex_count * vertex_count);
        for mask in 0..graph_count {
            let edges = graph_edges(vertex_count, mask);
            verify_graph(vertex_count, &edges, &mut root_cases, &mut renaming_cases);
            graph_cases += 1;
        }
    }
    verify_deep_small_stack();
    println!(
        "verified {graph_cases} witness graphs, {root_cases} rooted dominator/frontier cases, \
         {renaming_cases} lawful renamings, all registered mutants/malformed inputs, and a \
         {DEEP_CHAIN_VERTICES}-vertex 256 KiB-stack chain"
    );
}
