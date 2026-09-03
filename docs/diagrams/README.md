# Diagram Catalog

| Diagram | Editable source | Purpose |
|---|---|---|
| SCC fibers and quotient | [scc-quotient.puml](scc-quotient.puml) | Shows source vertices, quotient fibers, and condensation edges |
| Formal-first flow | [formal-first-flow.puml](formal-first-flow.puml) | Shows proof/model/oracle gates before implementation |
| Linear work bound | [linear-work-bound.puml](linear-work-bound.puml) | Shows exact SCC events, phase-complete work, and reusable-workspace bounds |
| Dependency-wave execution | [dependency-wave-execution.puml](dependency-wave-execution.puml) | Shows deterministic parallel evaluation with per-wave barriers and ordered commit |
| Snapshot wire layout | [interop-wire-layout.puml](interop-wire-layout.puml) | Fixes every v1.0 byte range, payload array, and exclusion |
| Snapshot admission machine | [interop-admission-machine.puml](interop-admission-machine.puml) | Shows identity, resource, structural, digest, publication, rejection, and release steps |
| Interop ownership boundaries | [interop-boundaries.puml](interop-boundaries.puml) | Shows dependency direction across adapters, the neutral core, interop, storage, and validators |

Run `scripts/render-diagrams.sh` and commit all PlantUML sources with their current SVG renderings.
