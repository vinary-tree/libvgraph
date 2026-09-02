#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_directory="$repository_root/target/verification"
repository_tmp="$repository_root/target/tmp"
mkdir -p "$evidence_directory" "$repository_tmp"

if [[ "${LIBVGRAPH_BOUNDARY_SCOPED:-0}" != 1 ]]; then
  exec systemd-run --user --scope \
    -p MemoryMax=4G -p MemorySwapMax=0 -p CPUQuota=100% -p TasksMax=64 \
    env LIBVGRAPH_BOUNDARY_SCOPED=1 CARGO_BUILD_JOBS=1 TMPDIR="$repository_tmp" \
    "$repository_root/scripts/check-core-boundary.sh"
fi

metadata="$evidence_directory/core-boundary-metadata.json"
cargo metadata --locked --offline --no-deps --format-version 1 >"$metadata"

if ! jq -e '
  (.packages | length) == 1 and
  .packages[0].name == "libvgraph" and
  (.packages[0].dependencies | all(.name != "serde" and .name != "serde_json")) and
  (.packages[0].features == {"default": []})
' "$metadata" >/dev/null; then
  printf '%s\n' 'serialization dependencies or features remain in the graph kernel manifest' >&2
  exit 1
fi

if rg -n '\b(serde|serde_json|Serialize|Deserialize)\b|format_version' \
    "$repository_root/src" "$repository_root/tests" "$repository_root/Cargo.toml"; then
  printf '%s\n' 'serialization belongs in libvgraph-interop, not the graph kernel' >&2
  exit 1
fi

printf '%s\n' 'verified neutral libvgraph core dependency boundary'
