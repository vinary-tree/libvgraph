#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_directory="$repository_root/target/verification"
mkdir -p "$evidence_directory"

if [[ "${LIBVGRAPH_DOCS_SCOPED:-0}" != 1 ]]; then
  exec systemd-run --user --scope \
    -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=400% -p TasksMax=64 \
    env LIBVGRAPH_DOCS_SCOPED=1 "$repository_root/scripts/verify-docs.sh"
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
