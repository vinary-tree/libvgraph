#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
vinary_graph_java_options="${JAVA_TOOL_OPTIONS:-} -Djava.awt.headless=true"
JAVA_TOOL_OPTIONS="$vinary_graph_java_options" plantuml -tsvg \
  "$repository_root/docs/diagrams/scc-quotient.puml" \
  "$repository_root/docs/diagrams/formal-first-flow.puml"
