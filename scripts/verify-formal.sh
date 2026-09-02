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

verify_rocq() {
  cd "$repository_root/formal/rocq"
  coqc -q GraphQuotient.v 2>&1 | tee "$evidence_directory/rocq.log"
  rg -q 'Closed under the global context' "$evidence_directory/rocq.log"
  coqc -q GraphSnapshot.v 2>&1 | tee "$evidence_directory/interop-rocq.log"
  [[ "$(rg -c '^Closed under the global context$' \
    "$evidence_directory/interop-rocq.log")" -eq 19 ]]
  if rg -n '\b(Admitted|admit|Axiom|Parameter|Hypothesis)\b' \
      GraphQuotient.v GraphSnapshot.v; then
    printf '%s\n' 'forbidden Rocq admission or axiom found' >&2
    return 1
  fi
}

verify_tla_core() {
  cd "$repository_root/formal/tla"
  tla2sany IterativeGraphMachine.tla 2>&1 | tee "$evidence_directory/tla-syntax.log"
  tlc -workers 1 -deadlock -metadir "$evidence_directory/tlc-core-state" \
    -config IterativeGraphMachine.cfg IterativeGraphMachine.tla 2>&1 \
    | tee "$evidence_directory/tlc.log"
  rg -q 'Model checking completed. No error has been found' "$evidence_directory/tlc.log"
}

verify_tla_interop() {
  cd "$repository_root/formal/tla"
  tla2sany InteropCodecMachine.tla 2>&1 \
    | tee "$evidence_directory/interop-tla-syntax.log"
  tlc -workers 1 -deadlock \
    -metadir "$evidence_directory/interop-tlc-positive-state" \
    -config InteropCodecMachine.cfg InteropCodecMachine.tla 2>&1 \
    | tee "$evidence_directory/interop-tlc-positive.log"
  rg -q 'Model checking completed. No error has been found' \
    "$evidence_directory/interop-tlc-positive.log"
  rg -q '16900 distinct states found' \
    "$evidence_directory/interop-tlc-positive.log"

  local configurations=(
    InteropCodecSkipSchema.cfg
    InteropCodecSkipCanonical.cfg
    InteropCodecIgnoreCancellation.cfg
    InteropCodecGrowNativeDepth.cfg
  )
  local labels=(schema canonical cancellation native-depth)
  local invariants=(PublicationSound PublicationSound PublicationSound NativeControlDepthBound)
  local index
  for index in "${!configurations[@]}"; do
    local log="$evidence_directory/interop-tlc-mutant-${labels[$index]}.log"
    set +e
    tlc -workers 1 -deadlock \
      -metadir "$evidence_directory/interop-tlc-mutant-${labels[$index]}-state" \
      -config "${configurations[$index]}" InteropCodecMachine.tla 2>&1 | tee "$log"
    local status="${PIPESTATUS[0]}"
    set -e
    if [[ "$status" -eq 0 ]]; then
      printf 'interop %s mutant unexpectedly satisfied the model\n' \
        "${labels[$index]}" >&2
      return 1
    fi
    rg -q "Invariant ${invariants[$index]} is violated" "$log"
  done
}

verify_tla_release() {
  cd "$repository_root/formal/tla"
  tla2sany ReleaseMachine.tla 2>&1 \
    | tee "$evidence_directory/release-tla-syntax.log"
  tlc -workers 1 -deadlock \
    -metadir "$evidence_directory/release-tlc-positive-state" \
    -config ReleaseMachine.cfg ReleaseMachine.tla 2>&1 \
    | tee "$evidence_directory/release-tlc-positive.log"
  rg -q 'Model checking completed. No error has been found' \
    "$evidence_directory/release-tlc-positive.log"
  rg -q '176 states generated, 128 distinct states found' \
    "$evidence_directory/release-tlc-positive.log"

  local configurations=(
    ReleaseMachineCandidatePolicy.cfg
    ReleaseMachineSkipProtectedHead.cfg
    ReleaseMachineSkipGates.cfg
    ReleaseMachinePublishEarly.cfg
    ReleaseMachineSkipEvidence.cfg
    ReleaseMachineSkipRegistryChecksum.cfg
    ReleaseMachineRepublish.cfg
  )
  local labels=(
    candidate-policy protected-head gates early-publication evidence registry-checksum republish
  )
  local invariants=(
    PublishedUsesProtectedTrust
    PublishedUsesProtectedHead
    PublishedHasPassedGates
    PublishedFromDraft
    PublishedHasCompleteAssets
    PublishedRegistryMatches
    AtMostOnePublication
  )
  local index
  for index in "${!configurations[@]}"; do
    local log="$evidence_directory/release-tlc-mutant-${labels[$index]}.log"
    set +e
    tlc -workers 1 -deadlock \
      -metadir "$evidence_directory/release-tlc-mutant-${labels[$index]}-state" \
      -config "${configurations[$index]}" ReleaseMachine.tla 2>&1 | tee "$log"
    local status="${PIPESTATUS[0]}"
    set -e
    if [[ "$status" -eq 0 ]]; then
      printf 'release %s mutant unexpectedly satisfied the model\n' \
        "${labels[$index]}" >&2
      return 1
    fi
    rg -q "Invariant ${invariants[$index]} is violated" "$log"
  done
}

verify_tla() {
  verify_tla_core
  verify_tla_interop
  verify_tla_release
}

verify_exhaustive_model() {
  rustc --edition=2021 -D warnings -O "$repository_root/formal/model/exhaustive_graphs.rs" \
    -o "$evidence_directory/exhaustive-graphs"
  "$evidence_directory/exhaustive-graphs" 2>&1 | tee "$evidence_directory/exhaustive.log"
  rg -q '^verified 66067 directed graphs,' "$evidence_directory/exhaustive.log"
  rustc --edition=2021 -D warnings -O \
    "$repository_root/formal/model/exhaustive_interop.rs" \
    -o "$evidence_directory/exhaustive-interop"
  "$evidence_directory/exhaustive-interop" 2>&1 \
    | tee "$evidence_directory/interop-model.log"
  rg -q '^verified 531 directed graphs, 1593 profile-bound encodings, 9321 lawful renamings, 180696 strict-prefix rejections,' \
    "$evidence_directory/interop-model.log"
}

verify_verus() {
  verus "$repository_root/formal/verus/flat_wave_refinement.rs" 2>&1 \
    | tee "$evidence_directory/verus.log"
  rg -q '^verification results:: [1-9][0-9]* verified, 0 errors$' \
    "$evidence_directory/verus.log"
  verus "$repository_root/formal/verus/interop_refinement.rs" 2>&1 \
    | tee "$evidence_directory/interop-verus.log"
  rg -q '^verification results:: 6 verified, 0 errors$' \
    "$evidence_directory/interop-verus.log"
}

verify_smt_interop() {
  z3 -smt2 "$repository_root/formal/smt/interop_snapshot.smt2" 2>&1 \
    | tee "$evidence_directory/interop-smt.log"
  mapfile -t results < <(rg '^(unsat|sat)$' "$evidence_directory/interop-smt.log")
  [[ "${#results[@]}" -eq 11 ]]
  local index
  for index in {0..8}; do
    [[ "${results[$index]}" == unsat ]]
  done
  [[ "${results[9]}" == sat ]]
  [[ "${results[10]}" == sat ]]
}

verify_invariants_interop() {
  "$repository_root/scripts/check-interop-invariants.sh" 2>&1 \
    | tee "$evidence_directory/interop-invariants.log"
  rg -q '^verified 74 interop and release invariants ' \
    "$evidence_directory/interop-invariants.log"
}

verify_required_red_interop() {
  local log="$evidence_directory/interop-required-red.log"
  cd "$repository_root"
  set +e
  CARGO_NET_OFFLINE=true cargo test --locked --offline --no-run \
    --test interop_contract_properties 2>&1 | tee "$log"
  local status="${PIPESTATUS[0]}"
  set -e
  [[ "$status" -eq 101 ]]
  [[ "$(rg -c '^error\[E[0-9]+\]:' "$log")" -eq 1 ]]
  rg -q '^error\[E0432\]: unresolved import `libvgraph_interop`$' "$log"
  if rg '^error\[E[0-9]+\]:' "$log" \
      | rg -v '^error\[E0432\]: unresolved import `libvgraph_interop`$'; then
    printf '%s\n' 'required-red failed for an unexpected compiler reason' >&2
    return 1
  fi
}

verify_interop() {
  verify_boundary
  verify_rocq
  verify_tla_interop
  verify_tla_release
  verify_exhaustive_model
  verify_verus
  verify_smt_interop
  verify_invariants_interop
  verify_required_red_interop
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
  interop-rocq)
    verify_rocq
    ;;
  tla)
    verify_tla
    ;;
  interop-tla)
    verify_tla_interop
    ;;
  release-tla)
    verify_tla_release
    ;;
  model)
    verify_exhaustive_model
    ;;
  interop-model)
    verify_exhaustive_model
    ;;
  verus)
    verify_verus
    ;;
  interop-verus)
    verify_verus
    ;;
  interop-smt)
    verify_smt_interop
    ;;
  interop-invariants)
    verify_invariants_interop
    ;;
  interop-required-red)
    verify_required_red_interop
    ;;
  interop)
    verify_interop
    ;;
  kani)
    verify_kani
    ;;
  all)
    "$repository_root/scripts/verify-formal.sh" boundary
    "$repository_root/scripts/verify-formal.sh" rocq
    "$repository_root/scripts/verify-formal.sh" tla
    "$repository_root/scripts/verify-formal.sh" model
    "$repository_root/scripts/verify-formal.sh" verus
    "$repository_root/scripts/verify-formal.sh" kani
    "$repository_root/scripts/verify-formal.sh" interop-smt
    "$repository_root/scripts/verify-formal.sh" interop-invariants
    "$repository_root/scripts/verify-formal.sh" interop-required-red
    ;;
  *)
    printf '%s\n' \
      'usage: scripts/verify-formal.sh [all|boundary|rocq|tla|model|verus|kani|interop|interop-rocq|interop-tla|release-tla|interop-model|interop-verus|interop-smt|interop-invariants|interop-required-red]' \
      >&2
    exit 2
    ;;
esac
