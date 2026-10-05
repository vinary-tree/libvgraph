#!/usr/bin/env bash

# Scan Cargo's flat lockfile table headers without recursion or a parser dependency.
lockfile_has_only_repository_resolution() {
  local lockfile="$1"
  if [[ ! -f "$lockfile" || -L "$lockfile" ]]; then
    return 1
  fi
  local header_pattern="^[[:space:]]*\\[\\[[[:space:]]*(patch|\"patch\"|'patch')[[:space:]]*\\.[[:space:]]*(unused|\"unused\"|'unused')[[:space:]]*\\]\\]"
  if LC_ALL=C grep -E -q -- "$header_pattern" "$lockfile"; then
    return 1
  else
    case "$?" in
      1) return 0 ;;
      *) return 1 ;;
    esac
  fi
}
