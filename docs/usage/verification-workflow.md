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

The command must finish with a repository-only lockfile pass, 28 closed-context messages from
Rocq, a no-error TLC completion, the exact exhaustive-oracle summary including exact linear Tarjan
work and the 256 KiB stack gate, six Verus proofs with zero errors, four successful Kani harnesses,
and a successful neutral-core dependency-boundary check. A nonzero exit, resource-cap termination,
or missing success marker fails the gate.

Before any concrete formal target can execute a resolver-affecting Cargo command,
`scripts/check-lockfile-portability.sh` enumerates every tracked `Cargo.lock` file. The check is
fail-closed: the set must be nonempty, each entry must be a regular repository file rather than a
symbolic link, and no entry may contain a `[[patch.unused]]` table. Cargo writes that table when a
dependency patch was visible but unused; retaining it can encode a developer's ambient Cargo
configuration rather than repository-owned resolution. The check copies each tracked lockfile to
a repository-backed test directory, injects an unused-patch table, and requires the mutant to be
rejected. It also requires missing-file and symbolic-link mutants to be rejected.

Every individual layer self-launches through `systemd-run --user --scope`. The wrapper sets
`MemorySwapMax=0`, `CARGO_BUILD_JOBS=1`, a 100% CPU quota, a finite task limit, and a repository
temporary directory. Java's temporary directory is pinned to that same repository location so
TLA+ cannot spill parser artifacts into memory-backed `/tmp`. Kani is capped at 2 GiB; every other
formal layer is capped at 4 GiB. Do not invoke Kani or CBMC directly for this repository.

The installed `cargo-kani` interface does not accept Cargo's `--locked` or `--offline` flags.
The wrapper therefore sets `CARGO_NET_OFFLINE=true`, hashes `Cargo.lock` before verification, and
rejects the evidence if any harness changes the lockfile. This is the Kani-specific refinement of
the repository's locked, offline verification policy.

Individual layers are available for proof development:

```bash
scripts/verify-formal.sh rocq
scripts/verify-formal.sh tla
scripts/verify-formal.sh model
scripts/verify-formal.sh verus
scripts/verify-formal.sh kani
scripts/verify-formal.sh boundary
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
  -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=100% -p TasksMax=64 \
  env CARGO_BUILD_JOBS=1 TMPDIR="$PWD/target/tmp" \
  cargo test --locked --offline --all-targets --all-features

systemd-run --user --scope \
  -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=100% -p TasksMax=64 \
  env CARGO_BUILD_JOBS=1 TMPDIR="$PWD/target/tmp" \
  cargo clippy --locked --offline --all-targets --all-features -- -D warnings
```

## Headless allocation evidence

`heaptrack` opens its graphical analyzer unless recording is explicitly constrained. Capture with
`--record-only`, then analyze with `heaptrack_print`:

```bash
systemd-run --user --scope \
  -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=100% -p TasksMax=64 \
  env TMPDIR="$PWD/target/tmp" \
  heaptrack --record-only -o target/verification/profiles/libvgraph.heaptrack \
  target/debug/examples/schedule_allocation_probe 100000

heaptrack_print -f target/verification/profiles/libvgraph.heaptrack.zst \
  --filter-bt-function schedule_impl -a -p -T -n 20
```

Never use `heaptrack -a` in this workflow: that option launches `heaptrack_gui` when installed.
Hash and record the raw capture and analysis in pgmcp, then remove the temporary capture.

## Failure discipline

A proof, model, oracle, diagram, or lint failure is a rejected contract state. Correct the source
of the discrepancy and rerun the affected layer, followed by both complete gates. Do not suppress
diagnostics, weaken invariants, accept partial output, or edit generated SVG independently of its
PlantUML source.
