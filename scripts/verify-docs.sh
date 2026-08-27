#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_directory="$repository_root/target/verification"
mkdir -p "$evidence_directory"

"$repository_root/scripts/render-diagrams.sh"
vinary-doc-lint check "$repository_root" --diagram-tools --format json 2>&1 \
  | tee "$evidence_directory/vinary-doc-lint.json"
jq -e '
  all(.files[];
    ((.diagnostics // []) | length) == 0 and
    ((.changes // []) | length) == 0
  )
' "$evidence_directory/vinary-doc-lint.json" >/dev/null
