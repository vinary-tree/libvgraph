use core::fmt::Debug;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use libvgraph::{CsrGraph, SccDecomposition};

const SIZES: [u32; 3] = [1_000, 10_000, 100_000];

fn must<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("benchmark fixture failed: {error:?}"),
    }
}

fn chain_inputs(size: u32) -> (Vec<u32>, Vec<(u32, u32)>) {
    let nodes = (0..size).collect();
    let edges = (0..size.saturating_sub(1))
        .map(|source| (source, source + 1))
        .collect();
    (nodes, edges)
}

fn benchmark_kernel(criterion: &mut Criterion) {
    let mut construction = criterion.benchmark_group("canonical-csr-chain");
    for size in SIZES {
        let (nodes, edges) = chain_inputs(size);
        construction.bench_with_input(BenchmarkId::from_parameter(size), &size, |bencher, _| {
            bencher.iter(|| {
                must(CsrGraph::from_edges(
                    black_box(nodes.clone()),
                    black_box(edges.clone()),
                ))
            });
        });
    }
    construction.finish();

    let mut scc = criterion.benchmark_group("iterative-scc-chain");
    for size in SIZES {
        let (nodes, edges) = chain_inputs(size);
        let graph = must(CsrGraph::from_edges(nodes, edges));
        scc.bench_with_input(
            BenchmarkId::from_parameter(size),
            &graph,
            |bencher, graph| {
                bencher.iter(|| must(SccDecomposition::compute(black_box(graph))));
            },
        );
    }
    scc.finish();

    let (nodes, edges) = chain_inputs(100_000);
    let graph = must(CsrGraph::from_edges(nodes, edges));
    let decomposition = must(SccDecomposition::compute(&graph));
    criterion.bench_function("wavefront-schedule-chain/100000", |bencher| {
        bencher.iter(|| must(black_box(decomposition.condensation()).schedule()));
    });
}

criterion_group!(benches, benchmark_kernel);
criterion_main!(benches);
