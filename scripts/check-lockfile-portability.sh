#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 0 ]]; then
  printf 'usage: %s\n' "$0" >&2
  exit 2
fi

script_directory="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(CDPATH='' cd -- "$script_directory/.." && pwd)"
# shellcheck source=scripts/lockfile-portability-lib.sh
source "$script_directory/lockfile-portability-lib.sh"

scratch_directory="$repository_root/target/tmp"
mkdir -p "$scratch_directory"
paths_file="$(mktemp "$scratch_directory/lockfile-paths.XXXXXXXX")"
trap 'rm -f -- "$paths_file"' EXIT
git -c core.fsmonitor=false -C "$repository_root" ls-files -z -- '*Cargo.lock' \
  > "$paths_file"
mapfile -d '' -t lockfiles < "$paths_file"
if [[ "${#lockfiles[@]}" -eq 0 ]]; then
  printf '%s\n' 'no tracked Cargo lockfiles found' >&2
  exit 1
fi

for relative_path in "${lockfiles[@]}"; do
  if ! lockfile_has_only_repository_resolution "$repository_root/$relative_path"; then
    printf 'nonportable or unreadable tracked lockfile: %s\n' "$relative_path" >&2
    exit 1
  fi
done

printf 'verified %s tracked Cargo lockfile(s) without ambient patch metadata\n' \
  "${#lockfiles[@]}"
