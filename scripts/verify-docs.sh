#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_directory="$repository_root/target/verification"
repository_tmp="$repository_root/target/tmp"
mkdir -p "$evidence_directory" "$repository_tmp"

if [[ "${LIBVGRAPH_DOCS_SCOPED:-0}" != 1 ]]; then
  exec systemd-run --user --scope \
    -p MemoryHigh=768M -p MemoryMax=1G -p MemorySwapMax=0 \
    -p CPUQuota=100% -p TasksMax=32 \
    env LIBVGRAPH_DOCS_SCOPED=1 TMPDIR="$repository_tmp" \
    JAVA_TOOL_OPTIONS="${JAVA_TOOL_OPTIONS:-} -Djava.io.tmpdir=$repository_tmp" \
    "$repository_root/scripts/verify-docs.sh"
fi

"$repository_root/scripts/render-diagrams.sh"
vinary-doc-lint check "$repository_root" --diagram-tools --format json 2>&1 \
  | tee "$evidence_directory/vinary-doc-lint.json"
jq -e '
  all(.files[];
    ((.diagnostics // []) | length) == 0 and
    ((.changes // []) | length) == 0
  )
' "$evidence_directory/vinary-doc-lint.json" >/dev/null
