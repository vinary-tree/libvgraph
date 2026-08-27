# Verification Workflow

## Prerequisites

The formal gate requires Rocq, TLA+/TLC, a Rust compiler, ripgrep, systemd user scopes, PlantUML,
and `vinary-doc-lint`. The repository does not download tools or dependencies during verification.

## Formal gate

From the repository root, run:

```bash
scripts/verify-formal.sh all
```

The command must finish with closed-context messages from Rocq, a no-error TLC completion, and the
exact exhaustive-oracle summary. A nonzero exit or missing success marker fails the gate.

Individual layers are available for proof development:

```bash
scripts/verify-formal.sh rocq
scripts/verify-formal.sh tla
scripts/verify-formal.sh model
```

An individual pass helps localize a defect but does not replace the final `all` run.

## Documentation gate

Run:

```bash
scripts/verify-docs.sh
```

This renders both PlantUML diagrams headlessly and runs online `vinary-doc-lint` with diagram-tool
checks. The accepted JSON report has no diagnostics and no proposed changes.

## Failure discipline

A proof, model, oracle, diagram, or lint failure is a rejected contract state. Correct the source
of the discrepancy and rerun the affected layer, followed by both complete gates. Do not suppress
diagnostics, weaken invariants, accept partial output, or edit generated SVG independently of its
PlantUML source.
