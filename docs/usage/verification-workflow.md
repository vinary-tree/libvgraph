# Verification Workflow

## Prerequisites

The formal gate requires Rocq, TLA+/TLC, Rust and Cargo, Kani/CBMC, Verus, ripgrep, systemd user
scopes, PlantUML, `jq`, and `vinary-doc-lint`. The repository does not download tools or
dependencies during verification.

## Formal gate

From the repository root, run:

```bash
scripts/verify-formal.sh all
```

The command must finish with 27 closed-context messages from Rocq, a no-error TLC completion, the
exact exhaustive-oracle summary including exact linear Tarjan work and the 256 KiB stack gate,
six Verus proofs with zero errors, and four successful Kani harnesses. A nonzero exit, resource-cap
termination, or missing success marker fails the gate.

Every individual layer self-launches through `systemd-run --user --scope`. The wrapper sets
`MemorySwapMax=0`, `CARGO_BUILD_JOBS=1`, a 400% CPU quota, and a finite task limit. Kani is capped
at 2 GiB; every other formal layer is capped at 4 GiB. Do not invoke Kani or CBMC directly for this
repository.

Individual layers are available for proof development:

```bash
scripts/verify-formal.sh rocq
scripts/verify-formal.sh tla
scripts/verify-formal.sh model
scripts/verify-formal.sh verus
scripts/verify-formal.sh kani
```

An individual pass helps localize a defect but does not replace the final `all` run.

## Documentation gate

Run:

```bash
scripts/verify-docs.sh
```

This renders every PlantUML diagram headlessly and runs `vinary-doc-lint` with diagram-tool checks.
The documentation wrapper also enforces a 4 GiB no-swap systemd scope. The accepted JSON report
has no diagnostics and no proposed changes.

## Rust quality gate

Build and test commands are also heavy operations. Run them in a no-swap systemd scope:

```bash
systemd-run --user --scope \
  -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=400% \
  env CARGO_BUILD_JOBS=1 cargo test --all-targets --all-features

systemd-run --user --scope \
  -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=400% \
  env CARGO_BUILD_JOBS=1 cargo clippy --all-targets --all-features -- -D warnings
```

## Headless allocation evidence

`heaptrack` opens its graphical analyzer unless recording is explicitly constrained. Capture with
`--record-only`, then analyze with `heaptrack_print`:

```bash
systemd-run --user --scope \
  -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=400% \
  heaptrack --record-only -o /tmp/libvgraph.heaptrack \
  target/debug/examples/schedule_allocation_probe 100000

heaptrack_print -f /tmp/libvgraph.heaptrack.zst \
  --filter-bt-function schedule_impl -a -p -T -n 20
```

Never use `heaptrack -a` in this workflow: that option launches `heaptrack_gui` when installed.
Hash and record the raw capture and analysis in pgmcp, then remove the temporary capture.

## Failure discipline

A proof, model, oracle, diagram, or lint failure is a rejected contract state. Correct the source
of the discrepancy and rerun the affected layer, followed by both complete gates. Do not suppress
diagnostics, weaken invariants, accept partial output, or edit generated SVG independently of its
PlantUML source.
