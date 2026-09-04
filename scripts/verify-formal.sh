#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
evidence_directory="$repository_root/target/verification"
repository_tmp="$repository_root/target/tmp"
mkdir -p "$evidence_directory" "$repository_tmp"

verification_target="${1:-all}"
if [[ "$verification_target" != all && "${LIBVGRAPH_FORMAL_SCOPED:-0}" != 1 ]]; then
  memory_high=1536M
  memory_max=2G
  if [[ "$verification_target" == z3 || "$verification_target" == invariants ]]; then
    memory_high=384M
    memory_max=512M
  fi
  java_tmp_option="-Djava.io.tmpdir=$repository_tmp"
  java_tool_options="${JAVA_TOOL_OPTIONS:-} $java_tmp_option"
  tla_java_options="${TLA_JAVA_OPTS:-} $java_tmp_option"
  exec systemd-run --user --scope \
    -p MemoryHigh="$memory_high" -p MemoryMax="$memory_max" \
    -p MemorySwapMax=0 -p CPUQuota=100% -p TasksMax=32 \
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
  coqc -q BorrowedCsrRefinement.v 2>&1 \
    | tee "$evidence_directory/rocq-borrowed-csr.log"
  rg -q 'Closed under the global context' "$evidence_directory/rocq.log"
  rg -q 'Closed under the global context' \
    "$evidence_directory/rocq-borrowed-csr.log"
  if rg -n '\b(Admitted|admit|Axiom)\b' \
      GraphQuotient.v BorrowedCsrRefinement.v; then
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

  tla2sany BorrowedCsrMachine.tla 2>&1 \
    | tee "$evidence_directory/tla-borrowed-csr-syntax.log"
  tlc -workers 1 -config BorrowedCsrMachine.cfg BorrowedCsrMachine.tla 2>&1 \
    | tee "$evidence_directory/tlc-borrowed-csr.log"
  rg -q 'Model checking completed. No error has been found' \
    "$evidence_directory/tlc-borrowed-csr.log"

  local negative_controls=(
    "BorrowedCsrHeaderMutant:PublishedInputIsCanonical"
    "BorrowedCsrOffsetMutant:PublishedInputIsCanonical"
    "BorrowedCsrTargetMutant:CheckedBeforeIndexed"
    "BorrowedCsrOrderMutant:CheckedBeforeIndexed"
    "BorrowedCsrDuplicateMutant:CheckedBeforeIndexed"
    "BorrowedCsrPublicationMutant:NoPartialPublication"
  )
  local entry model invariant log status
  for entry in "${negative_controls[@]}"; do
    model="${entry%%:*}"
    invariant="${entry#*:}"
    log="$evidence_directory/tlc-${model,,}-required-red.log"
    set +e
    tlc -workers 1 -config "$model.cfg" BorrowedCsrMachine.tla 2>&1 | tee "$log"
    status="${PIPESTATUS[0]}"
    set -e
    if [[ "$status" -eq 0 ]]; then
      printf 'required-red model unexpectedly passed: %s\n' "$model" >&2
      return 1
    fi
    rg -q "Invariant $invariant is violated" "$log"
  done
}

verify_exhaustive_model() {
  rustc --edition=2021 -D warnings -O "$repository_root/formal/model/exhaustive_graphs.rs" \
    -o "$evidence_directory/exhaustive-graphs"
  "$evidence_directory/exhaustive-graphs" 2>&1 | tee "$evidence_directory/exhaustive.log"
  rg -q '^verified 66067 directed graphs,' "$evidence_directory/exhaustive.log"
  rg -q '48776 raw borrowed representations, fail-atomic cancellation' \
    "$evidence_directory/exhaustive.log"
}

verify_z3() {
  z3 "$repository_root/formal/z3/BorrowedCsrRefinement.smt2" 2>&1 \
    | tee "$evidence_directory/z3-borrowed-csr.log"
  [[ "$(rg -c '^unsat$' "$evidence_directory/z3-borrowed-csr.log")" -eq 8 ]]
  if rg -q '^(sat|unknown)$' "$evidence_directory/z3-borrowed-csr.log"; then
    printf '%s\n' 'borrowed-CSR Z3 proof did not discharge every obligation' >&2
    return 1
  fi

  z3 "$repository_root/formal/z3/BorrowedCsrRequiredRed.smt2" 2>&1 \
    | tee "$evidence_directory/z3-borrowed-csr-required-red.log"
  [[ "$(rg -c '^sat$' "$evidence_directory/z3-borrowed-csr-required-red.log")" -eq 6 ]]
  if rg -q '^(unsat|unknown)$' \
      "$evidence_directory/z3-borrowed-csr-required-red.log"; then
    printf '%s\n' 'a borrowed-CSR Z3 required-red counterexample disappeared' >&2
    return 1
  fi
}

verify_invariant_ledger() {
  local ledger="$repository_root/formal/invariants/borrowed-csr.json"
  jq -e '
    .schema_version == 1 and
    .baseline_commit == "1f5df96651b61e88fe86e84f27c1635a2971c29e" and
    (.invariants | length) == 15 and
    ([.invariants[].id] | unique | length) == 15 and
    all(.invariants[];
      (.id | test("^BCSR-[0-9]{3}$")) and
      (.statement | length > 0) and
      (.rocq | length > 0) and
      (.tla | length > 0) and
      (.exhaustive | length > 0) and
      (.property_test | length > 0)
    )
  ' "$ledger" >/dev/null
}

verify_verus() {
  verus "$repository_root/formal/verus/flat_wave_refinement.rs" 2>&1 \
    | tee "$evidence_directory/verus.log"
  rg -q '^verification results:: [1-9][0-9]* verified, 0 errors$' \
    "$evidence_directory/verus.log"
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
  z3)
    verify_z3
    ;;
  invariants)
    verify_invariant_ledger
    ;;
  all)
    "$repository_root/scripts/verify-formal.sh" boundary
    "$repository_root/scripts/verify-formal.sh" rocq
    "$repository_root/scripts/verify-formal.sh" tla
    "$repository_root/scripts/verify-formal.sh" model
    "$repository_root/scripts/verify-formal.sh" verus
    "$repository_root/scripts/verify-formal.sh" z3
    "$repository_root/scripts/verify-formal.sh" invariants
    "$repository_root/scripts/verify-formal.sh" kani
    ;;
  *)
    printf '%s\n' \
      'usage: scripts/verify-formal.sh [all|boundary|rocq|tla|model|verus|z3|invariants|kani]' \
      >&2
    exit 2
    ;;
esac
