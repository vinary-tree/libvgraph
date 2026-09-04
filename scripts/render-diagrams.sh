#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repository_tmp="$repository_root/target/tmp"
mkdir -p "$repository_tmp"
if [[ "${LIBVGRAPH_DOCS_SCOPED:-0}" != 1 ]]; then
  exec systemd-run --user --scope \
    -p MemoryHigh=768M -p MemoryMax=1G -p MemorySwapMax=0 \
    -p CPUQuota=100% -p TasksMax=32 \
    env LIBVGRAPH_DOCS_SCOPED=1 TMPDIR="$repository_tmp" \
    JAVA_TOOL_OPTIONS="${JAVA_TOOL_OPTIONS:-} -Djava.io.tmpdir=$repository_tmp" \
    "$repository_root/scripts/render-diagrams.sh"
fi
vinary_graph_java_options="${JAVA_TOOL_OPTIONS:-} -Djava.awt.headless=true -Djava.io.tmpdir=$repository_tmp"
JAVA_TOOL_OPTIONS="$vinary_graph_java_options" plantuml -tsvg \
  "$repository_root/docs/diagrams/scc-quotient.puml" \
  "$repository_root/docs/diagrams/formal-first-flow.puml" \
  "$repository_root/docs/diagrams/linear-work-bound.puml" \
  "$repository_root/docs/diagrams/dependency-wave-execution.puml" \
  "$repository_root/docs/diagrams/interop-wire-layout.puml" \
  "$repository_root/docs/diagrams/interop-admission-machine.puml" \
  "$repository_root/docs/diagrams/interop-boundaries.puml" \
  "$repository_root/docs/diagrams/borrowed-csr-refinement.puml"
