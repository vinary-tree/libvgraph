# Rust API and Usage

## Construct a canonical graph

A **stable identifier** is the caller's ordered vertex key. A **dense identifier** is libvgraph's
zero-based index into canonical compressed sparse row (CSR) storage. Construction sorts and
deduplicates both vertices and edges, then maps stable identifiers to dense identifiers.

```rust
use libvgraph::{CsrGraph, SccDecomposition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph = CsrGraph::from_edges(
        ["emit", "lower", "parse", "typecheck"],
        [
            ("parse", "lower"),
            ("lower", "typecheck"),
            ("typecheck", "emit"),
        ],
    )?;

    let parse = graph.dense_id(&"parse").expect("parse is in the stable domain");
    let reachable = graph.breadth_first(parse)?;
    assert_eq!(reachable.len(), 4);

    let decomposition = SccDecomposition::compute(&graph)?;
    assert!(decomposition.is_acyclic());
    let schedule = decomposition.condensation().schedule()?;
    assert_eq!(schedule.wave_count(), 4);
    Ok(())
}
```

`CsrGraph::from_dense_edges` avoids comparison-based stable-label canonicalization when the caller
already owns a zero-based domain. `BuildOptions` can omit reverse CSR to reduce retained output;
`with_reverse` materializes it later in strict linear work.

## Consume SCC fibers and dependency waves

An SCC **fiber** is the sorted slice of source vertices mapped to one component. A dependency
**wave** is the sorted slice of components with one longest-predecessor rank. Components in the
same wave have no direct condensation edge between them.

```rust
# use libvgraph::{CsrGraph, SccDecomposition};
# fn run() -> Result<(), Box<dyn std::error::Error>> {
# let graph = CsrGraph::from_edges([0, 1, 2], [(0, 1), (1, 0), (1, 2)])?;
let decomposition = SccDecomposition::compute(&graph)?;

for (component, members) in decomposition.fibers() {
    println!("component {} contains {members:?}", component.id());
}

let schedule = decomposition.condensation().schedule()?;
for (rank, wave) in schedule.waves().enumerate() {
    println!("rank {rank}: {wave:?}");
}
# Ok(())
# }
```

The schedule owns exactly two wave buffers: offsets and members. `wave(rank)` returns a borrowed
slice, while `waves()` is an exact-size, double-ended iterator. No call allocates one vector per
wave.

## Bound work and request cancellation

`ExecutionControl` counts deterministic logical operations, not elapsed time. A batch that would
exceed its limit is rejected before the batch mutates the meter. Cancellation uses a caller-owned
`AtomicBool`; its relaxed load communicates cancellation only and does not publish caller data.

```rust
use libvgraph::{ComputeError, CsrGraph, ExecutionControl, SccDecomposition};

# fn run() -> Result<(), Box<dyn std::error::Error>> {
let graph = CsrGraph::from_edges([0, 1], [(0, 1)])?;
let result = SccDecomposition::compute_with_control(
    &graph,
    ExecutionControl::with_work_limit(0),
);
assert!(matches!(result, Err(ComputeError::Incomplete(_))));
# Ok(())
# }
```

An incomplete result never contains a partial decomposition or schedule. Callers that need exact
answers must retry with a sufficient budget and an unset cancellation flag.

## Reuse analysis storage

`SccWorkspace` retains temporary vectors across calls. Reuse one workspace serially for a stream
of graphs to amortize allocations. The returned decomposition owns its partition and condensation,
so a later workspace call cannot mutate an earlier result.

```rust
# use libvgraph::{CsrGraph, SccWorkspace};
# fn run() -> Result<(), Box<dyn std::error::Error>> {
let first = CsrGraph::from_edges([0, 1], [(0, 1)])?;
let second = CsrGraph::from_edges([0, 1], [(0, 1), (1, 0)])?;
let mut workspace = SccWorkspace::new();

let acyclic = workspace.compute(&first)?;
let cyclic = workspace.compute(&second)?;
assert!(acyclic.is_acyclic());
assert!(!cyclic.is_acyclic());
# Ok(())
# }
```

Use one workspace per concurrent worker. Sharing a mutable workspace would require external
synchronization and would erase its allocation-reuse advantage.

## Serialize validated source graphs

Enable the `serde` feature to serialize `CsrGraph<K>`. The wire representation is explicitly
versioned. Deserialization validates all CSR invariants before returning a graph.

```toml
[dependencies]
libvgraph = { version = "0.1.0", features = ["serde"] }
serde_json = "1"
```

```rust
# use libvgraph::CsrGraph;
# fn run() -> Result<(), Box<dyn std::error::Error>> {
let graph = CsrGraph::from_edges(["a", "b"], [("a", "b")])?;
let encoded = serde_json::to_vec(&graph)?;
let decoded: CsrGraph<&str> = serde_json::from_slice(&encoded)?;
assert_eq!(decoded, graph);
# Ok(())
# }
```

Derived SCC and schedule values are intentionally recomputed rather than deserialized. This keeps
the validated source graph as the only persisted trust boundary.
