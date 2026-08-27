#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${LIBVGRAPH_DOCS_SCOPED:-0}" != 1 ]]; then
  exec systemd-run --user --scope \
    -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=400% -p TasksMax=64 \
    env LIBVGRAPH_DOCS_SCOPED=1 "$repository_root/scripts/render-diagrams.sh"
fi
vinary_graph_java_options="${JAVA_TOOL_OPTIONS:-} -Djava.awt.headless=true"
JAVA_TOOL_OPTIONS="$vinary_graph_java_options" plantuml -tsvg \
  "$repository_root/docs/diagrams/scc-quotient.puml" \
  "$repository_root/docs/diagrams/formal-first-flow.puml" \
  "$repository_root/docs/diagrams/linear-work-bound.puml" \
  "$repository_root/docs/diagrams/dependency-wave-execution.puml"
