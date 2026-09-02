#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ledger="$repository_root/formal/doc/interop-invariants.tsv"
properties="$repository_root/tests/interop_contract_properties.rs"
evidence_directory="$repository_root/target/verification"
mkdir -p "$evidence_directory"

expected_header=$'invariant_id\tobligation\tlayer\tartifact\tsymbol\tproperty'
actual_header="$(head -n 1 "$ledger")"
if [[ "$actual_header" != "$expected_header" ]]; then
  printf '%s\n' 'interop invariant ledger header is not canonical' >&2
  exit 1
fi

row_count="$(awk 'NR > 1 { count += 1 } END { print count + 0 }' "$ledger")"
if [[ "$row_count" -ne 66 ]]; then
  printf 'expected 66 interop invariant rows, found %s\n' "$row_count" >&2
  exit 1
fi

duplicate_ids="$(awk -F '\t' 'NR > 1 { print $1 }' "$ledger" | sort | uniq -d)"
if [[ -n "$duplicate_ids" ]]; then
  printf 'duplicate interop invariant ids:\n%s\n' "$duplicate_ids" >&2
  exit 1
fi

for layer in Rocq TLC Z3 Model Verus RequiredRed; do
  if ! awk -F '\t' -v expected="$layer" \
      'NR > 1 && $3 == expected { found = 1 } END { exit !found }' \
      "$ledger"; then
    printf 'interop invariant ledger has no %s obligations\n' "$layer" >&2
    exit 1
  fi
done

while IFS=$'\t' read -r invariant_id obligation layer artifact symbol property; do
  if [[ "$invariant_id" == invariant_id ]]; then
    continue
  fi
  if [[ ! -f "$repository_root/$artifact" ]]; then
    printf '%s references missing artifact %s\n' "$invariant_id" "$artifact" >&2
    exit 1
  fi
  if ! rg -Fq -- "$symbol" "$repository_root/$artifact"; then
    printf '%s references missing symbol %s in %s\n' \
      "$invariant_id" "$symbol" "$artifact" >&2
    exit 1
  fi
  if ! rg -Fq -- "fn $property" "$properties"; then
    printf '%s references missing required-red property %s\n' \
      "$invariant_id" "$property" >&2
    exit 1
  fi
  if [[ -z "$obligation" || -z "$layer" ]]; then
    printf '%s has an empty obligation or layer\n' "$invariant_id" >&2
    exit 1
  fi
done < "$ledger"

rg -o 'fn contract_[a-z0-9_]+' "$properties" \
  | awk '{ print $2 }' | sort -u > "$evidence_directory/interop-properties.actual"
awk -F '\t' 'NR > 1 { print $6 }' "$ledger" \
  | sort -u > "$evidence_directory/interop-properties.mapped"
if ! diff -u \
  "$evidence_directory/interop-properties.actual" \
  "$evidence_directory/interop-properties.mapped"; then
  printf '%s\n' 'required-red properties and invariant mappings differ' >&2
  exit 1
fi

if rg -n '\b(TODO|FIXME|HACK|XXX|PENDING)\b' \
    "$ledger" "$properties" \
    "$repository_root/formal/rocq/GraphSnapshot.v" \
    "$repository_root/formal/tla/InteropCodecMachine.tla" \
    "$repository_root/formal/smt/interop_snapshot.smt2" \
    "$repository_root/formal/model/exhaustive_interop.rs" \
    "$repository_root/formal/verus/interop_refinement.rs"; then
  printf '%s\n' 'interop contract contains a forbidden incompletion marker' >&2
  exit 1
fi

printf 'verified %s interop invariants across Rocq, TLC, Z3, model, Verus, and required-red layers\n' \
  "$row_count"
