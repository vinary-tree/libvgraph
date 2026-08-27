use core::fmt::Debug;
use std::hint::black_box;

use libvgraph::{CsrGraph, DenseId, SccDecomposition};

fn must<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("allocation probe failed: {error:?}"),
    }
}

fn main() {
    let vertex_count = match std::env::args().nth(1) {
        Some(argument) => match argument.parse::<u32>() {
            Ok(value) if value > 0 => value,
            Ok(_) => panic!("vertex count must be positive"),
            Err(error) => panic!("vertex count is not a u32: {error}"),
        },
        None => 100_000,
    };
    let edges = (0..vertex_count.saturating_sub(1))
        .map(|source| (DenseId::from_raw(source), DenseId::from_raw(source + 1)));
    let graph = must(CsrGraph::from_dense_edges(vertex_count, edges));
    let decomposition = must(SccDecomposition::compute(&graph));
    let schedule = must(black_box(decomposition.condensation()).schedule());
    println!(
        "vertices={vertex_count} waves={} members={} logical_work={}",
        schedule.wave_count(),
        schedule.wave_members().len(),
        schedule.logical_work()
    );
    black_box(schedule);
}
