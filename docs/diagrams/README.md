# Diagram Catalog

| Diagram | Editable source | Purpose |
|---|---|---|
| SCC fibers and quotient | [scc-quotient.puml](scc-quotient.puml) | Shows source vertices, quotient fibers, and condensation edges |
| Formal-first flow | [formal-first-flow.puml](formal-first-flow.puml) | Shows proof/model/oracle gates before implementation |
| Linear work bound | [linear-work-bound.puml](linear-work-bound.puml) | Shows exact SCC events, phase-complete work, and reusable-workspace bounds |
| Dependency-wave execution | [dependency-wave-execution.puml](dependency-wave-execution.puml) | Shows deterministic parallel evaluation with per-wave barriers and ordered commit |
| Witness evidence flow | [witness-evidence-flow.puml](witness-evidence-flow.puml) | Separates payload-free CSR, opaque provenance, replayable paths, quotient witness fibers, selection policy, and rooted dominance |
| Selector naturality | [selector-naturality.puml](selector-naturality.puml) | Gives the symmetry counterexample and the transported-order correction |
| Dominator frontier | [dominator-frontier.puml](dominator-frontier.puml) | Shows rooted dominance and the predecessor-based frontier definition at a join |

Run `scripts/render-diagrams.sh` and commit all PlantUML sources with their current SVG renderings.
