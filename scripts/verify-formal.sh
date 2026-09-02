#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_directory="$repository_root/target/verification"
repository_tmp="$repository_root/target/tmp"
mkdir -p "$evidence_directory" "$repository_tmp"

verification_target="${1:-all}"
if [[ "$verification_target" != all && "${LIBVGRAPH_FORMAL_SCOPED:-0}" != 1 ]]; then
  memory_max=4G
  if [[ "$verification_target" == kani ]]; then
    memory_max=2G
  fi
  java_tmp_option="-Djava.io.tmpdir=$repository_tmp"
  java_tool_options="${JAVA_TOOL_OPTIONS:-} $java_tmp_option"
  tla_java_options="${TLA_JAVA_OPTS:-} $java_tmp_option"
  exec systemd-run --user --scope \
    -p MemoryMax="$memory_max" -p MemorySwapMax=0 -p CPUQuota=100% -p TasksMax=64 \
    env LIBVGRAPH_FORMAL_SCOPED=1 CARGO_BUILD_JOBS=1 TMPDIR="$repository_tmp" \
    JAVA_TOOL_OPTIONS="$java_tool_options" TLA_JAVA_OPTS="$tla_java_options" \
    "$repository_root/scripts/verify-formal.sh" "$verification_target"
fi

verify_boundary() {
  LIBVGRAPH_BOUNDARY_SCOPED=1 "$repository_root/scripts/check-core-boundary.sh"
}

verify_rocq_core() {
  cd "$repository_root/formal/rocq"
  coqc -q GraphQuotient.v 2>&1 | tee "$evidence_directory/rocq.log"
  rg -q 'Closed under the global context' "$evidence_directory/rocq.log"
  if rg -n '\b(Admitted|admit|Axiom)\b' GraphQuotient.v; then
    printf '%s\n' 'forbidden Rocq admission or axiom found' >&2
    return 1
  fi
}

verify_rocq_witness() {
  cd "$repository_root/formal/rocq"
  coqc -q GraphQuotient.v >/dev/null
  coqc -q GraphWitnesses.v 2>&1 | tee "$evidence_directory/witness-rocq.log"
  rg -q 'Closed under the global context' "$evidence_directory/witness-rocq.log"
  if rg -n '\b(Admitted|admit|Axiom)\b' GraphWitnesses.v; then
    printf '%s\n' 'forbidden witness Rocq admission or axiom found' >&2
    return 1
  fi
}

verify_rocq() {
  verify_rocq_core
  verify_rocq_witness
}

verify_tla_core() {
  cd "$repository_root/formal/tla"
  tla2sany IterativeGraphMachine.tla 2>&1 | tee "$evidence_directory/tla-syntax.log"
  tlc -metadir "$evidence_directory/tlc-core-state" \
    -config IterativeGraphMachine.cfg IterativeGraphMachine.tla 2>&1 \
    | tee "$evidence_directory/tlc.log"
  rg -q 'Model checking completed. No error has been found' "$evidence_directory/tlc.log"
}

verify_tla_witness() {
  cd "$repository_root/formal/tla"
  tla2sany WitnessMachine.tla 2>&1 | tee "$evidence_directory/witness-tla-syntax.log"
  tlc -metadir "$evidence_directory/witness-tlc-reachable-state" \
    -config WitnessMachine.cfg WitnessMachine.tla 2>&1 \
    | tee "$evidence_directory/witness-tlc-reachable.log"
  tlc -metadir "$evidence_directory/witness-tlc-unreachable-state" \
    -config WitnessMachineUnreachable.cfg WitnessMachine.tla 2>&1 \
    | tee "$evidence_directory/witness-tlc-unreachable.log"
  rg -q 'Model checking completed. No error has been found' \
    "$evidence_directory/witness-tlc-reachable.log"
  rg -q 'Model checking completed. No error has been found' \
    "$evidence_directory/witness-tlc-unreachable.log"
}

verify_tla() {
  verify_tla_core
  verify_tla_witness
}

verify_exhaustive_core() {
  rustc --edition=2021 -D warnings -O "$repository_root/formal/model/exhaustive_graphs.rs" \
    -o "$evidence_directory/exhaustive-graphs"
  "$evidence_directory/exhaustive-graphs" 2>&1 | tee "$evidence_directory/exhaustive.log"
  rg -q '^verified 66067 directed graphs,' "$evidence_directory/exhaustive.log"
}

verify_exhaustive_witness() {
  rustc --edition=2021 -D warnings -O \
    "$repository_root/formal/model/exhaustive_witnesses.rs" \
    -o "$evidence_directory/exhaustive-witnesses"
  "$evidence_directory/exhaustive-witnesses" 2>&1 \
    | tee "$evidence_directory/witness-model.log"
  rg -q '^verified 530 witness graphs, 1570 rooted dominator/frontier cases, 3106 lawful renamings,' \
    "$evidence_directory/witness-model.log"
}

verify_exhaustive_model() {
  verify_exhaustive_core
  verify_exhaustive_witness
}

verify_verus_core() {
  verus "$repository_root/formal/verus/flat_wave_refinement.rs" 2>&1 \
    | tee "$evidence_directory/verus.log"
  rg -q '^verification results:: [1-9][0-9]* verified, 0 errors$' \
    "$evidence_directory/verus.log"
}

verify_verus_witness() {
  verus "$repository_root/formal/verus/witness_refinement.rs" 2>&1 \
    | tee "$evidence_directory/witness-verus.log"
  rg -q '^verification results:: [1-9][0-9]* verified, 0 errors$' \
    "$evidence_directory/witness-verus.log"
}

verify_verus() {
  verify_verus_core
  verify_verus_witness
}

verify_kani() {
  local kani_log="$evidence_directory/kani.log"
  local lockfile="$repository_root/Cargo.lock"
  local expected_lock_hash
  expected_lock_hash="$(sha256sum "$lockfile" | cut -d ' ' -f 1)"
  local harnesses=(
    encoded_pairs_round_trip_without_aliasing
    radix_work_charge_is_exact_and_overflow_free_in_the_graph_domain
    bounded_work_admission_is_fail_atomic_and_overflow_safe
    flat_wave_buffers_are_exact_sorted_rank_fibers
  )
  : > "$kani_log"
  for harness in "${harnesses[@]}"; do
    CARGO_NET_OFFLINE=true cargo kani --harness "$harness" 2>&1 | tee -a "$kani_log"
    if [[ "$(sha256sum "$lockfile" | cut -d ' ' -f 1)" != "$expected_lock_hash" ]]; then
      printf '%s\n' 'cargo-kani modified Cargo.lock; refusing non-locked proof evidence' >&2
      return 1
    fi
  done
  [[ "$(rg -c '^VERIFICATION:- SUCCESSFUL$' "$kani_log")" -eq "${#harnesses[@]}" ]]
}

case "${1:-all}" in
  boundary)
    verify_boundary
    ;;
  rocq)
    verify_rocq
    ;;
  witness-rocq)
    verify_rocq_witness
    ;;
  tla)
    verify_tla
    ;;
  witness-tla)
    verify_tla_witness
    ;;
  model)
    verify_exhaustive_model
    ;;
  witness-model)
    verify_exhaustive_witness
    ;;
  verus)
    verify_verus
    ;;
  witness-verus)
    verify_verus_witness
    ;;
  kani)
    verify_kani
    ;;
  witness)
    verify_rocq_witness
    verify_tla_witness
    verify_exhaustive_witness
    verify_verus_witness
    ;;
  all)
    "$repository_root/scripts/verify-formal.sh" boundary
    "$repository_root/scripts/verify-formal.sh" rocq
    "$repository_root/scripts/verify-formal.sh" tla
    "$repository_root/scripts/verify-formal.sh" model
    "$repository_root/scripts/verify-formal.sh" verus
    "$repository_root/scripts/verify-formal.sh" kani
    ;;
  *)
    printf '%s\n' \
      'usage: scripts/verify-formal.sh [all|boundary|rocq|witness-rocq|tla|witness-tla|model|witness-model|verus|witness-verus|kani|witness]' >&2
    exit 2
    ;;
esac
