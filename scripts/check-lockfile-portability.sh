#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
test_root="$repository_root/target/lockfile-portability-tests"
mkdir -p "$test_root"
run_directory="$(mktemp -d "$test_root/run.XXXXXXXX")"
trap 'rm -rf -- "$run_directory"' EXIT

lockfile_has_repository_only_resolution() {
  local lockfile="$1"
  [[ -f "$lockfile" && ! -L "$lockfile" ]] \
    && ! LC_ALL=C grep -E -q \
      '^[[:space:]]*\[\[patch[.]unused\]\][[:space:]]*(#.*)?$' \
      "$lockfile"
}

if lockfile_has_repository_only_resolution "$run_directory/missing-Cargo.lock"; then
  printf '%s\n' 'missing lockfile unexpectedly passed portability verification' >&2
  exit 1
fi

mapfile -d '' -t tracked_lockfiles < <(
  git -C "$repository_root" ls-files -z -- '*Cargo.lock'
)
if [[ "${#tracked_lockfiles[@]}" -eq 0 ]]; then
  printf '%s\n' 'lockfile portability verification found no tracked Cargo lockfiles' >&2
  exit 1
fi

index=0
for tracked_lockfile in "${tracked_lockfiles[@]}"; do
  lockfile="$repository_root/$tracked_lockfile"
  if ! lockfile_has_repository_only_resolution "$lockfile"; then
    printf 'tracked lockfile contains ambient unused-patch state: %s\n' \
      "$tracked_lockfile" >&2
    exit 1
  fi

  symlink_mutant="$run_directory/symlink-$index-Cargo.lock"
  ln -s -- "$lockfile" "$symlink_mutant"
  if lockfile_has_repository_only_resolution "$symlink_mutant"; then
    printf 'symlink lockfile mutant unexpectedly passed: %s\n' \
      "$tracked_lockfile" >&2
    exit 1
  fi

  unused_patch_mutant="$run_directory/unused-patch-$index-Cargo.lock"
  cp -- "$lockfile" "$unused_patch_mutant"
  printf '\n[[patch.unused]]\nname = "ambient-patch-mutant"\nversion = "0.0.0"\n' \
    >> "$unused_patch_mutant"
  if lockfile_has_repository_only_resolution "$unused_patch_mutant"; then
    printf 'ambient unused-patch mutant unexpectedly passed: %s\n' \
      "$tracked_lockfile" >&2
    exit 1
  fi

  index="$((index + 1))"
done

printf 'verified repository-only resolution for %s tracked Cargo lockfile(s) and rejected missing, symlink, and ambient unused-patch mutants\n' \
  "${#tracked_lockfiles[@]}"
