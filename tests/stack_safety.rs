use core::fmt::Debug;
use std::thread;

use libvgraph::{ComponentId, CsrGraph, DenseId, SccWorkspace};

const SMALL_STACK_BYTES: usize = 256 * 1024;
const DEEP_OR_WIDE_VERTICES: u32 = 100_000;
const DENSE_VERTICES: u32 = 384;

fn must<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("stack-safety fixture failed: {error:?}"),
    }
}

fn chain_graph(vertex_count: u32) -> CsrGraph<u32> {
    let last_edge = vertex_count.saturating_sub(1);
    let forward_offsets = (0..=vertex_count)
        .map(|offset| offset.min(last_edge))
        .collect();
    let forward_targets = (1..vertex_count).map(DenseId::from_raw).collect();
    let reverse_offsets = (0..=vertex_count)
        .map(|offset| offset.saturating_sub(1))
        .collect();
    let reverse_targets = (0..last_edge).map(DenseId::from_raw).collect();
    must(CsrGraph::try_from_parts(
        (0..vertex_count).collect(),
        forward_offsets,
        forward_targets,
        Some((reverse_offsets, reverse_targets)),
    ))
}

fn wide_star_graph(vertex_count: u32) -> CsrGraph<u32> {
    let edge_count = vertex_count.saturating_sub(1);
    let mut forward_offsets = Vec::with_capacity(vertex_count as usize + 1);
    forward_offsets.push(0);
    for _ in 0..vertex_count {
        forward_offsets.push(edge_count);
    }
    let forward_targets = (1..vertex_count).map(DenseId::from_raw).collect();
    let reverse_offsets = (0..=vertex_count)
        .map(|offset| offset.saturating_sub(1))
        .collect();
    let reverse_targets = vec![DenseId::from_raw(0); edge_count as usize];
    must(CsrGraph::try_from_parts(
        (0..vertex_count).collect(),
        forward_offsets,
        forward_targets,
        Some((reverse_offsets, reverse_targets)),
    ))
}

fn dense_graph(vertex_count: u32) -> CsrGraph<u32> {
    let offsets = (0..=vertex_count)
        .map(|source| source * vertex_count)
        .collect::<Vec<_>>();
    let targets = (0..vertex_count)
        .flat_map(|_| (0..vertex_count).map(DenseId::from_raw))
        .collect::<Vec<_>>();
    must(CsrGraph::try_from_parts(
        (0..vertex_count).collect(),
        offsets.clone(),
        targets.clone(),
        Some((offsets, targets)),
    ))
}

#[test]
fn every_depth_sensitive_public_lifecycle_is_stack_safe() {
    let worker = thread::Builder::new()
        .name("libvgraph-production-small-stack".into())
        .stack_size(SMALL_STACK_BYTES)
        .spawn(|| {
            let graph = chain_graph(DEEP_OR_WIDE_VERTICES);
            assert_eq!(graph.validate(), Ok(()));

            let breadth_first = must(graph.breadth_first(DenseId::from_raw(0)));
            assert_eq!(breadth_first.len(), DEEP_OR_WIDE_VERTICES as usize);
            drop(breadth_first);
            let depth_first = must(graph.depth_first_preorder(DenseId::from_raw(0)));
            assert_eq!(depth_first.len(), DEEP_OR_WIDE_VERTICES as usize);
            drop(depth_first);
            let forest = must(graph.depth_first_forest());
            assert_eq!(forest.len(), DEEP_OR_WIDE_VERTICES as usize);
            drop(forest);

            let mut workspace = SccWorkspace::new();
            let decomposition = must(workspace.compute(&graph));
            assert_eq!(decomposition.validate(&graph), Ok(()));
            assert_eq!(
                decomposition.component_count(),
                DEEP_OR_WIDE_VERTICES as usize
            );
            let schedule = must(decomposition.condensation().schedule());
            assert_eq!(schedule.wave_count(), DEEP_OR_WIDE_VERTICES as usize);
            assert_eq!(
                schedule.wave_offsets().len(),
                DEEP_OR_WIDE_VERTICES as usize + 1
            );
            assert_eq!(
                schedule.wave_members().len(),
                DEEP_OR_WIDE_VERTICES as usize
            );
            assert!(schedule.waves().all(|wave| wave.len() == 1));
            assert_eq!(schedule.wave(0), Some(&[ComponentId::from_raw(0)][..]));
            assert_eq!(
                schedule.logical_work(),
                6 * u64::from(DEEP_OR_WIDE_VERTICES)
                    + u64::from(DEEP_OR_WIDE_VERTICES - 1)
                    + 3 * u64::from(DEEP_OR_WIDE_VERTICES)
                    + 1
            );

            let graph_clone = graph.clone();
            let decomposition_clone = decomposition.clone();
            let schedule_clone = schedule.clone();
            assert_eq!(graph_clone, graph);
            assert_eq!(decomposition_clone, decomposition);
            assert_eq!(schedule_clone, schedule);
            assert!(!format!("{graph:?}").is_empty());
            assert!(!format!("{decomposition:?}").is_empty());
            assert!(!format!("{schedule:?}").is_empty());
            drop(schedule_clone);
            drop(decomposition_clone);
            drop(graph_clone);
            drop(schedule);
            drop(decomposition);
            drop(graph);

            let wide = wide_star_graph(DEEP_OR_WIDE_VERTICES);
            let wide_decomposition = must(workspace.compute(&wide));
            assert_eq!(wide_decomposition.validate(&wide), Ok(()));
            let wide_schedule = must(wide_decomposition.condensation().schedule());
            assert_eq!(wide_schedule.wave_count(), 2);
            assert_eq!(wide_schedule.wave(0), Some(&[ComponentId::from_raw(0)][..]));
            assert_eq!(
                wide_schedule.wave(1).map(<[ComponentId]>::len),
                Some(DEEP_OR_WIDE_VERTICES as usize - 1)
            );
            drop(wide_schedule);
            drop(wide_decomposition);
            drop(wide);

            let dense = dense_graph(DENSE_VERTICES);
            let dense_decomposition = must(workspace.compute(&dense));
            assert_eq!(dense_decomposition.validate(&dense), Ok(()));
            assert_eq!(dense_decomposition.component_count(), 1);
            assert!(!dense_decomposition.is_acyclic());
            let dense_schedule = must(dense_decomposition.condensation().schedule());
            assert_eq!(dense_schedule.wave_count(), 1);
            assert_eq!(
                dense_schedule.wave(0),
                Some(&[ComponentId::from_raw(0)][..])
            );
            drop(dense_schedule);
            drop(dense_decomposition);
            drop(dense);
            drop(workspace);
        });
    let worker = match worker {
        Ok(worker) => worker,
        Err(error) => panic!("small-stack worker failed to spawn: {error}"),
    };
    if let Err(payload) = worker.join() {
        std::panic::resume_unwind(payload);
    }
}
