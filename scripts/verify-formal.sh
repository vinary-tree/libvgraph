#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_directory="$repository_root/target/verification"
mkdir -p "$evidence_directory"

verification_target="${1:-all}"
if [[ "$verification_target" != all && "${LIBVGRAPH_FORMAL_SCOPED:-0}" != 1 ]]; then
  memory_max=4G
  if [[ "$verification_target" == kani ]]; then
    memory_max=2G
  fi
  exec systemd-run --user --scope \
    -p MemoryMax="$memory_max" -p MemorySwapMax=0 -p CPUQuota=400% -p TasksMax=64 \
    env LIBVGRAPH_FORMAL_SCOPED=1 CARGO_BUILD_JOBS=1 \
    "$repository_root/scripts/verify-formal.sh" "$verification_target"
fi

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

verify_verus() {
  verus "$repository_root/formal/verus/flat_wave_refinement.rs" 2>&1 \
    | tee "$evidence_directory/verus.log"
  rg -q '^verification results:: [1-9][0-9]* verified, 0 errors$' \
    "$evidence_directory/verus.log"
}

verify_kani() {
  local kani_log="$evidence_directory/kani.log"
  local harnesses=(
    encoded_pairs_round_trip_without_aliasing
    radix_work_charge_is_exact_and_overflow_free_in_the_graph_domain
    bounded_work_admission_is_fail_atomic_and_overflow_safe
    flat_wave_buffers_are_exact_sorted_rank_fibers
  )
  : > "$kani_log"
  for harness in "${harnesses[@]}"; do
    cargo kani --harness "$harness" 2>&1 | tee -a "$kani_log"
  done
  [[ "$(rg -c '^VERIFICATION:- SUCCESSFUL$' "$kani_log")" -eq "${#harnesses[@]}" ]]
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
  verus)
    verify_verus
    ;;
  kani)
    verify_kani
    ;;
  all)
    "$repository_root/scripts/verify-formal.sh" rocq
    "$repository_root/scripts/verify-formal.sh" tla
    "$repository_root/scripts/verify-formal.sh" model
    "$repository_root/scripts/verify-formal.sh" verus
    "$repository_root/scripts/verify-formal.sh" kani
    ;;
  *)
    printf '%s\n' 'usage: scripts/verify-formal.sh [all|rocq|tla|model|verus|kani]' >&2
    exit 2
    ;;
esac
