#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
repository_tmp="$repository_root/target/tmp"
isolated_cargo_home="$repository_tmp/hermetic-cargo-home"
standard_target_directory="$repository_root/target/hermetic-cargo"
kani_target_directory="$repository_root/target/kani-hermetic"

if [[ "$#" -eq 0 ]]; then
  printf '%s\n' \
    'usage: scripts/run-cargo-hermetic.sh {bench|build|check|clippy|doc|fetch|metadata|run|rustc|test|kani} [arguments...]' \
    >&2
  exit 64
fi

cargo_subcommand="$1"
shift

case "$cargo_subcommand" in
  bench|build|check|clippy|doc|fetch|metadata|run|rustc|test|kani)
    ;;
  *)
    printf 'unsupported hermetic Cargo subcommand: %s\n' "$cargo_subcommand" >&2
    exit 64
    ;;
esac

source_cargo_home="${CARGO_HOME:-${HOME:?HOME is required to locate the Cargo cache}/.cargo}"
if [[ ! -d "$source_cargo_home" ]]; then
  printf 'Cargo cache home is not a directory: %s\n' "$source_cargo_home" >&2
  exit 66
fi
source_cargo_home="$(cd "$source_cargo_home" && pwd -P)"

mkdir -p "$repository_tmp" "$isolated_cargo_home" "$standard_target_directory" \
  "$kani_target_directory"
isolated_cargo_home="$(cd "$isolated_cargo_home" && pwd -P)"

if [[ "$source_cargo_home" == "$isolated_cargo_home" ]]; then
  printf '%s\n' 'source and isolated Cargo homes must be distinct' >&2
  exit 78
fi

for configuration in config config.toml; do
  if [[ -e "$isolated_cargo_home/$configuration" || \
        -L "$isolated_cargo_home/$configuration" ]]; then
    printf 'refusing ambient Cargo configuration in isolated home: %s\n' \
      "$isolated_cargo_home/$configuration" >&2
    exit 78
  fi
done

for cache in registry git; do
  source_cache="$source_cargo_home/$cache"
  isolated_cache="$isolated_cargo_home/$cache"
  if [[ ! -e "$source_cache" ]]; then
    continue
  fi
  if [[ -L "$isolated_cache" ]]; then
    if [[ "$(readlink -f "$isolated_cache")" != "$(readlink -f "$source_cache")" ]]; then
      printf 'refusing mismatched Cargo cache link: %s\n' "$isolated_cache" >&2
      exit 78
    fi
  elif [[ -e "$isolated_cache" ]]; then
    printf 'refusing non-symlink Cargo cache path: %s\n' "$isolated_cache" >&2
    exit 78
  else
    ln -s "$source_cache" "$isolated_cache"
  fi
done

export CARGO_HOME="$isolated_cargo_home"
export CARGO_NET_OFFLINE=true
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export TMPDIR="$repository_tmp"

cd /
if [[ "$cargo_subcommand" == kani ]]; then
  exec cargo kani --manifest-path "$repository_root/Cargo.toml" \
    --target-dir "$kani_target_directory" "$@"
fi

export CARGO_TARGET_DIR="$standard_target_directory"
exec cargo "$cargo_subcommand" --manifest-path "$repository_root/Cargo.toml" \
  --locked --offline "$@"
