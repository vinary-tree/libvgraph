#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_directory="$repository_root/target/verification"
mkdir -p "$evidence_directory"

verify_rocq() {
  cd "$repository_root/formal/rocq"
  coqc -q GraphQuotient.v 2>&1 | tee "$evidence_directory/rocq.log"
  rg -q 'Closed under the global context' "$evidence_directory/rocq.log"
  if rg -n '\b(Admitted|admit|Axiom)\b' GraphQuotient.v; then
    printf '%s\n' 'forbidden Rocq admission or axiom found' >&2
    return 1
  fi
}

verify_tla() {
  cd "$repository_root/formal/tla"
  tla2sany IterativeGraphMachine.tla 2>&1 | tee "$evidence_directory/tla-syntax.log"
  tlc -config IterativeGraphMachine.cfg IterativeGraphMachine.tla 2>&1 \
    | tee "$evidence_directory/tlc.log"
  rg -q 'Model checking completed. No error has been found' "$evidence_directory/tlc.log"
}

verify_exhaustive_model() {
  rustc --edition=2021 -D warnings -O "$repository_root/formal/model/exhaustive_graphs.rs" \
    -o "$evidence_directory/exhaustive-graphs"
  "$evidence_directory/exhaustive-graphs" 2>&1 | tee "$evidence_directory/exhaustive.log"
  rg -q '^verified 66067 directed graphs,' "$evidence_directory/exhaustive.log"
}

case "${1:-all}" in
  rocq)
    verify_rocq
    ;;
  tla)
    verify_tla
    ;;
  model)
    verify_exhaustive_model
    ;;
  all)
    systemd-run --user --scope \
      -p MemoryMax=8G -p CPUQuota=800% -p IOWeight=30 -p TasksMax=128 \
      "$repository_root/scripts/verify-formal.sh" rocq
    "$repository_root/scripts/verify-formal.sh" tla
    "$repository_root/scripts/verify-formal.sh" model
    ;;
  *)
    printf '%s\n' 'usage: scripts/verify-formal.sh [all|rocq|tla|model]' >&2
    exit 2
    ;;
esac
