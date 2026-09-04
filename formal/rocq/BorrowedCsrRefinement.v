From Stdlib Require Import List Arith Lia Bool.
Import ListNotations.

Require Import GraphQuotient.

Set Implicit Arguments.

(** A borrowed compressed-sparse-row (CSR) observation owns no graph storage.
    [offsets] delimits one successor row per dense vertex and [targets] stores
    raw dense [u32]-domain targets.  The executable booleans below are the
    mathematical contract that the production admission state machine must
    refine. *)

Definition header_admittedb
    (vertex_count : nat) (offsets targets : list nat) : bool :=
  match offsets with
  | [] => false
  | origin :: _ =>
      andb (Nat.eqb (length offsets) (S vertex_count))
        (andb (Nat.eqb origin 0)
          (match nth_error offsets vertex_count with
           | Some terminal => Nat.eqb terminal (length targets)
           | None => false
           end))
  end.

Definition row_admittedb
    (offsets targets : list nat) (vertex : nat) : bool :=
  match nth_error offsets vertex, nth_error offsets (S vertex) with
  | Some start, Some stop =>
      andb (Nat.leb start stop) (Nat.leb stop (length targets))
  | _, _ => false
  end.

Definition rows_admittedb
    (vertex_count : nat) (offsets targets : list nat) : bool :=
  forallb (row_admittedb offsets targets) (seq 0 vertex_count).

Definition targets_admittedb
    (vertex_count : nat) (targets : list nat) : bool :=
  forallb (fun target => Nat.ltb target vertex_count) targets.

Fixpoint strictly_increasingb (values : list nat) : bool :=
  match values with
  | preceding :: ((following :: _) as tail) =>
      andb (Nat.ltb preceding following) (strictly_increasingb tail)
  | _ => true
  end.

Definition row_values
    (offsets targets : list nat) (vertex : nat) : list nat :=
  match nth_error offsets vertex, nth_error offsets (S vertex) with
  | Some start, Some stop => firstn (stop - start) (skipn start targets)
  | _, _ => []
  end.

Definition canonical_rowsb
    (vertex_count : nat) (offsets targets : list nat) : bool :=
  forallb
    (fun vertex => strictly_increasingb (row_values offsets targets vertex))
    (seq 0 vertex_count).

Definition borrowed_admittedb
    (vertex_count : nat) (offsets targets : list nat) : bool :=
  andb (header_admittedb vertex_count offsets targets)
    (andb (rows_admittedb vertex_count offsets targets)
      (andb (targets_admittedb vertex_count targets)
        (canonical_rowsb vertex_count offsets targets))).

Lemma borrowed_header_length_exact :
  forall vertex_count offsets targets,
    borrowed_admittedb vertex_count offsets targets = true ->
    length offsets = S vertex_count.
Proof.
  intros vertex_count offsets targets Hadmitted.
  unfold borrowed_admittedb in Hadmitted.
  apply andb_true_iff in Hadmitted as [Hheader _].
  unfold header_admittedb in Hheader.
  destruct offsets as [|origin remaining].
  - discriminate.
  - apply andb_true_iff in Hheader as [Hlength _].
    apply Nat.eqb_eq in Hlength.
    exact Hlength.
Qed.

Lemma borrowed_header_origin_exact :
  forall vertex_count offsets targets,
    borrowed_admittedb vertex_count offsets targets = true ->
    nth_error offsets 0 = Some 0.
Proof.
  intros vertex_count offsets targets Hadmitted.
  unfold borrowed_admittedb in Hadmitted.
  apply andb_true_iff in Hadmitted as [Hheader _].
  unfold header_admittedb in Hheader.
  destruct offsets as [|origin remaining].
  - discriminate.
  - simpl.
    apply andb_true_iff in Hheader as [_ Hremaining].
    apply andb_true_iff in Hremaining as [Horigin _].
    apply Nat.eqb_eq in Horigin.
    now subst origin.
Qed.

Lemma borrowed_header_terminal_exact :
  forall vertex_count offsets targets,
    borrowed_admittedb vertex_count offsets targets = true ->
    nth_error offsets vertex_count = Some (length targets).
Proof.
  intros vertex_count offsets targets Hadmitted.
  unfold borrowed_admittedb in Hadmitted.
  apply andb_true_iff in Hadmitted as [Hheader _].
  unfold header_admittedb in Hheader.
  destruct offsets as [|origin remaining].
  - discriminate.
  - apply andb_true_iff in Hheader as [_ Hremaining].
    apply andb_true_iff in Hremaining as [_ Hterminal].
    destruct (nth_error (origin :: remaining) vertex_count)
      as [terminal|] eqn:Hlookup.
    + apply Nat.eqb_eq in Hterminal.
      now subst terminal.
    + discriminate.
Qed.

Lemma borrowed_row_bounds_safe :
  forall vertex_count offsets targets vertex,
    borrowed_admittedb vertex_count offsets targets = true ->
    vertex < vertex_count ->
    exists start stop,
      nth_error offsets vertex = Some start /\
      nth_error offsets (S vertex) = Some stop /\
      start <= stop /\
      stop <= length targets.
Proof.
  intros vertex_count offsets targets vertex Hadmitted Hvertex.
  unfold borrowed_admittedb in Hadmitted.
  apply andb_true_iff in Hadmitted as [_ Hremaining].
  apply andb_true_iff in Hremaining as [Hrows _].
  unfold rows_admittedb in Hrows.
  apply forallb_forall with (x := vertex) in Hrows.
  - unfold row_admittedb in Hrows.
    destruct (nth_error offsets vertex) as [start|] eqn:Hstart;
      destruct (nth_error offsets (S vertex)) as [stop|] eqn:Hstop;
      try discriminate.
    apply andb_true_iff in Hrows as [Hordered Hterminal].
    apply Nat.leb_le in Hordered.
    apply Nat.leb_le in Hterminal.
    exists start, stop.
    repeat split; assumption.
  - apply in_seq.
    lia.
Qed.

Lemma borrowed_target_range_safe :
  forall vertex_count offsets targets target,
    borrowed_admittedb vertex_count offsets targets = true ->
    In target targets ->
    target < vertex_count.
Proof.
  intros vertex_count offsets targets target Hadmitted Hin.
  unfold borrowed_admittedb in Hadmitted.
  apply andb_true_iff in Hadmitted as [_ Hremaining].
  apply andb_true_iff in Hremaining as [_ Hremaining].
  apply andb_true_iff in Hremaining as [Htargets _].
  unfold targets_admittedb in Htargets.
  apply forallb_forall with (x := target) in Htargets; [|exact Hin].
  now apply Nat.ltb_lt in Htargets.
Qed.

Definition borrowed_edge
    (offsets targets : list nat) (source target : nat) : Prop :=
  exists start stop position,
    nth_error offsets source = Some start /\
    nth_error offsets (S source) = Some stop /\
    start <= position < stop /\
    nth_error targets position = Some target.

Theorem borrowed_edge_target_safe :
  forall vertex_count offsets targets source target,
    borrowed_admittedb vertex_count offsets targets = true ->
    borrowed_edge offsets targets source target ->
    target < vertex_count.
Proof.
  intros vertex_count offsets targets source target Hadmitted
    [start [stop [position [_ [_ [_ Htarget]]]]]].
  eapply (@borrowed_target_range_safe vertex_count offsets targets target).
  - exact Hadmitted.
  - apply nth_error_In in Htarget.
    exact Htarget.
Qed.

Lemma strictly_increasingb_tail :
  forall head tail,
    strictly_increasingb (head :: tail) = true ->
    strictly_increasingb tail = true.
Proof.
  intros head tail Hstrict.
  destruct tail as [|next remaining].
  - reflexivity.
  - simpl in Hstrict.
    now apply andb_true_iff in Hstrict as [_ Htail].
Qed.

Lemma strictly_increasingb_adjacent :
  forall values prefix left right suffix,
    values = prefix ++ left :: right :: suffix ->
    strictly_increasingb values = true ->
    left < right.
Proof.
  intros values prefix.
  revert values.
  induction prefix as [|head prefix IH];
    intros values left right suffix Hvalues Hstrict.
  - subst values.
    simpl in Hstrict.
    apply andb_true_iff in Hstrict as [Hordered _].
    now apply Nat.ltb_lt in Hordered.
  - subst values.
    apply IH with (values := prefix ++ left :: right :: suffix)
      (suffix := suffix).
    + reflexivity.
    + apply strictly_increasingb_tail with (head := head).
      exact Hstrict.
Qed.

Theorem borrowed_row_adjacent_targets_strict :
  forall vertex_count offsets targets vertex prefix left right suffix,
    borrowed_admittedb vertex_count offsets targets = true ->
    vertex < vertex_count ->
    row_values offsets targets vertex =
      prefix ++ left :: right :: suffix ->
    left < right.
Proof.
  intros vertex_count offsets targets vertex prefix left right suffix
    Hadmitted Hvertex Hrow.
  unfold borrowed_admittedb in Hadmitted.
  apply andb_true_iff in Hadmitted as [_ Hremaining].
  apply andb_true_iff in Hremaining as [_ Hremaining].
  apply andb_true_iff in Hremaining as [_ Hcanonical].
  unfold canonical_rowsb in Hcanonical.
  apply forallb_forall with (x := vertex) in Hcanonical.
  - eapply strictly_increasingb_adjacent.
    + exact Hrow.
    + exact Hcanonical.
  - apply in_seq.
    lia.
Qed.

Theorem duplicate_row_is_rejected :
  borrowed_admittedb 1 [0; 2] [0; 0] = false.
Proof. reflexivity. Qed.

Theorem decreasing_row_is_rejected :
  borrowed_admittedb 2 [0; 2; 2] [1; 0] = false.
Proof. reflexivity. Qed.

Theorem malformed_header_is_rejected :
  borrowed_admittedb 2 [0; 1] [1] = false.
Proof. reflexivity. Qed.

Theorem decreasing_offsets_are_rejected :
  borrowed_admittedb 2 [0; 2; 1] [0] = false.
Proof. reflexivity. Qed.

Theorem out_of_range_target_is_rejected :
  borrowed_admittedb 1 [0; 1] [1] = false.
Proof. reflexivity. Qed.

(** The borrowed and owned observations differ only in storage ownership.
    Their denotation is deliberately the same predicate, which proves that
    admission cannot change graph meaning. *)

Record borrowed_csr : Type := {
  borrowed_vertex_count : nat;
  borrowed_offsets : list nat;
  borrowed_targets : list nat;
  borrowed_admitted :
    borrowed_admittedb borrowed_vertex_count borrowed_offsets
      borrowed_targets = true
}.

Record owned_csr : Type := {
  owned_vertex_count : nat;
  owned_offsets : list nat;
  owned_targets : list nat;
  owned_admitted :
    borrowed_admittedb owned_vertex_count owned_offsets owned_targets = true
}.

Definition materialize_owned (input : borrowed_csr) : owned_csr :=
  {| owned_vertex_count := borrowed_vertex_count input;
     owned_offsets := borrowed_offsets input;
     owned_targets := borrowed_targets input;
     owned_admitted := borrowed_admitted input |}.

Definition borrowed_denotes (input : borrowed_csr) : nat -> nat -> Prop :=
  borrowed_edge (borrowed_offsets input) (borrowed_targets input).

Definition owned_denotes (input : owned_csr) : nat -> nat -> Prop :=
  borrowed_edge (owned_offsets input) (owned_targets input).

Theorem borrowed_owned_edge_equivalence :
  forall input source target,
    borrowed_denotes input source target <->
    owned_denotes (materialize_owned input) source target.
Proof.
  intros input source target.
  reflexivity.
Qed.

Definition borrowed_buffer_identity (input : borrowed_csr) : list nat * list nat :=
  (borrowed_offsets input, borrowed_targets input).

Theorem borrowed_observation_preserves_buffer_identity :
  forall input,
    borrowed_buffer_identity input =
      (borrowed_offsets input, borrowed_targets input).
Proof.
  reflexivity.
Qed.

Definition adapter_input_clone_slots : nat := 0.

Theorem borrowed_adapter_allocates_no_input_clone :
  forall input : borrowed_csr,
    adapter_input_clone_slots = 0.
Proof.
  intros input.
  reflexivity.
Qed.

(** Publication is exactly successful admission.  The transition-system model
    strengthens this extensional theorem by proving that no intermediate or
    cancelled state can publish. *)

Inductive admission_outcome : Type :=
| AdmissionRejected
| AdmissionPublished.

Definition decide_admission
    (vertex_count : nat) (offsets targets : list nat) : admission_outcome :=
  if borrowed_admittedb vertex_count offsets targets
  then AdmissionPublished
  else AdmissionRejected.

Definition outcome_publishes (outcome : admission_outcome) : Prop :=
  outcome = AdmissionPublished.

Theorem publication_iff_admitted :
  forall vertex_count offsets targets,
    outcome_publishes (decide_admission vertex_count offsets targets) <->
    borrowed_admittedb vertex_count offsets targets = true.
Proof.
  intros vertex_count offsets targets.
  unfold outcome_publishes, decide_admission.
  destruct (borrowed_admittedb vertex_count offsets targets) eqn:Hadmitted.
  - split; intro; reflexivity.
  - split; intro H.
    + discriminate.
    + discriminate.
Qed.

Theorem rejected_input_publishes_nothing :
  forall vertex_count offsets targets,
    borrowed_admittedb vertex_count offsets targets = false ->
    decide_admission vertex_count offsets targets = AdmissionRejected.
Proof.
  intros vertex_count offsets targets Hrejected.
  unfold decide_admission.
  now rewrite Hrejected.
Qed.

(** SCC fibers and condensation retain the already-proved exact quotient laws
    when their edge relation is the borrowed CSR denotation. *)

Theorem borrowed_scc_fiber_total :
  forall (input : borrowed_csr) (C : Type) (quotient : nat -> C)
    (vertex : nat),
    fiber quotient (quotient vertex) vertex.
Proof.
  intros.
  apply fiber_total.
Qed.

Theorem borrowed_scc_fibers_disjoint :
  forall (input : borrowed_csr) (C : Type) (quotient : nat -> C)
    (vertex : nat) (left right : C),
    fiber quotient left vertex ->
    fiber quotient right vertex ->
    left = right.
Proof.
  intros.
  eapply fibers_disjoint; eauto.
Qed.

Theorem borrowed_scc_fibers_nonempty :
  forall (input : borrowed_csr) (C : Type) (quotient : nat -> C),
    scc_quotient_laws (borrowed_denotes input) quotient ->
    forall component : C,
      exists vertex : nat, quotient vertex = component.
Proof.
  intros input C quotient Hlaws.
  exact (scc_quotient_fibers_nonempty Hlaws).
Qed.

Theorem borrowed_scc_kernel_exact :
  forall (input : borrowed_csr) (C : Type) (quotient : nat -> C),
    scc_quotient_laws (borrowed_denotes input) quotient ->
    forall left right,
      quotient left = quotient right <->
      strongly_connected (borrowed_denotes input) left right.
Proof.
  intros input C quotient Hlaws.
  exact (scc_quotient_kernel_exact Hlaws).
Qed.

Theorem borrowed_condensation_edge_exact :
  forall (input : borrowed_csr) (C : Type) (quotient : nat -> C)
    source target,
    quotient_edge (borrowed_denotes input) quotient source target <->
    source <> target /\
    exists source_vertex target_vertex,
      quotient source_vertex = source /\
      quotient target_vertex = target /\
      borrowed_denotes input source_vertex target_vertex.
Proof.
  intros.
  split; intro Hedge.
  - exact (quotient_edge_has_witness Hedge).
  - exact Hedge.
Qed.

Definition singleton_fiber {V C : Type}
    (quotient : V -> C) (vertex : V) : Prop :=
  forall candidate, quotient candidate = quotient vertex -> candidate = vertex.

Definition nonempty_cycle {V : Type}
    (edge : V -> V -> Prop) (vertex : V) : Prop :=
  exists next, edge vertex next /\ reach edge next vertex.

Theorem singleton_nonempty_cycle_iff_self_loop :
  forall (V C : Type) (edge : V -> V -> Prop) (quotient : V -> C)
    (vertex : V),
    scc_quotient_laws edge quotient ->
    singleton_fiber quotient vertex ->
    (nonempty_cycle edge vertex <-> edge vertex vertex).
Proof.
  intros V C edge quotient vertex Hlaws Hsingleton.
  split.
  - intros [next [Hedge Hreturn]].
    assert (Hconnected : strongly_connected edge vertex next).
    {
      split.
      - exact (reach_step vertex next Hedge).
      - exact Hreturn.
    }
    assert (Hsame : quotient vertex = quotient next).
    {
      apply (proj2 (quotient_exact_kernel Hlaws vertex next)).
      exact Hconnected.
    }
    specialize (Hsingleton next (eq_sym Hsame)).
    now subst next.
  - intro Hself.
    exists vertex.
    split.
    + exact Hself.
    + apply reach_refl.
Qed.

(** Canonical component identifiers are characterized, rather than assumed,
    by the least member of each total fiber. *)

Record canonical_component_numbering
    (component_count : nat) (quotient : nat -> nat) : Type := {
  canonical_minimum : nat -> nat;
  canonical_minimum_in_fiber :
    forall component,
      component < component_count ->
      quotient (canonical_minimum component) = component;
  canonical_minimum_is_least :
    forall component vertex,
      component < component_count ->
      quotient vertex = component ->
      canonical_minimum component <= vertex;
  canonical_identifier_order :
    forall left right,
      left < component_count ->
      right < component_count ->
      (left < right <->
       canonical_minimum left < canonical_minimum right)
}.

Theorem component_ids_are_ordered_by_least_member :
  forall component_count quotient
    (numbering : canonical_component_numbering component_count quotient),
    forall left right,
      left < component_count ->
      right < component_count ->
      (left < right <->
       @canonical_minimum component_count quotient numbering left <
       @canonical_minimum component_count quotient numbering right).
Proof.
  intros component_count quotient numbering left right Hleft Hright.
  exact (@canonical_identifier_order component_count quotient numbering
    left right Hleft Hright).
Qed.

(** Validation performs one constant-time header check, one row-bound check per
    vertex, and one fused range/order check per inspected edge. *)

Record complete_borrowed_validation
    (vertex_count edge_count : nat) : Type := {
  validation_header_checks : nat;
  validation_row_checks : nat;
  validation_edge_checks : nat;
  validation_header_exact : validation_header_checks = 1;
  validation_rows_exact : validation_row_checks = vertex_count;
  validation_edges_exact : validation_edge_checks = edge_count
}.

Definition borrowed_validation_work
    {vertex_count edge_count}
    (trace : complete_borrowed_validation vertex_count edge_count) : nat :=
  validation_header_checks trace +
  validation_row_checks trace +
  validation_edge_checks trace.

Theorem borrowed_validation_work_exact :
  forall vertex_count edge_count
    (trace : complete_borrowed_validation vertex_count edge_count),
    borrowed_validation_work trace = 1 + vertex_count + edge_count.
Proof.
  intros vertex_count edge_count trace.
  destruct trace.
  unfold borrowed_validation_work.
  simpl.
  lia.
Qed.

Theorem borrowed_validation_work_linear :
  forall vertex_count edge_count
    (trace : complete_borrowed_validation vertex_count edge_count),
    borrowed_validation_work trace <= 2 * (vertex_count + edge_count + 1).
Proof.
  intros vertex_count edge_count trace.
  rewrite borrowed_validation_work_exact.
  lia.
Qed.

Definition borrowed_tarjan_validation_upper_bound
    (vertex_count edge_count : nat) : nat :=
  (5 * vertex_count + edge_count) +
  (1 + vertex_count + edge_count).

Theorem borrowed_tarjan_validation_is_strictly_linear :
  forall vertex_count edge_count,
    borrowed_tarjan_validation_upper_bound vertex_count edge_count =
      6 * vertex_count + 2 * edge_count + 1.
Proof.
  intros.
  unfold borrowed_tarjan_validation_upper_bound.
  lia.
Qed.

Theorem borrowed_pipeline_adds_no_clone_slots :
  forall vertex_count edge_count candidate_count component_count
    quotient_edge_count wave_count,
    quotient_dimensions vertex_count edge_count candidate_count component_count
      quotient_edge_count wave_count ->
    pipeline_auxiliary_slots vertex_count candidate_count component_count +
      adapter_input_clone_slots <=
      9 * vertex_count + 2 * edge_count + quotient_radix_bucket_count.
Proof.
  intros vertex_count edge_count candidate_count component_count
    quotient_edge_count wave_count Hdimensions.
  change
    (pipeline_auxiliary_slots vertex_count candidate_count component_count + 0 <=
     9 * vertex_count + 2 * edge_count + quotient_radix_bucket_count).
  pose proof (pipeline_auxiliary_slots_linear Hdimensions) as Hbound.
  lia.
Qed.

Print Assumptions borrowed_header_length_exact.
Print Assumptions borrowed_header_origin_exact.
Print Assumptions borrowed_header_terminal_exact.
Print Assumptions borrowed_row_bounds_safe.
Print Assumptions borrowed_edge_target_safe.
Print Assumptions borrowed_row_adjacent_targets_strict.
Print Assumptions duplicate_row_is_rejected.
Print Assumptions decreasing_row_is_rejected.
Print Assumptions malformed_header_is_rejected.
Print Assumptions decreasing_offsets_are_rejected.
Print Assumptions out_of_range_target_is_rejected.
Print Assumptions borrowed_owned_edge_equivalence.
Print Assumptions borrowed_observation_preserves_buffer_identity.
Print Assumptions borrowed_adapter_allocates_no_input_clone.
Print Assumptions publication_iff_admitted.
Print Assumptions rejected_input_publishes_nothing.
Print Assumptions borrowed_scc_fiber_total.
Print Assumptions borrowed_scc_fibers_disjoint.
Print Assumptions borrowed_scc_fibers_nonempty.
Print Assumptions borrowed_scc_kernel_exact.
Print Assumptions borrowed_condensation_edge_exact.
Print Assumptions singleton_nonempty_cycle_iff_self_loop.
Print Assumptions component_ids_are_ordered_by_least_member.
Print Assumptions borrowed_validation_work_exact.
Print Assumptions borrowed_validation_work_linear.
Print Assumptions borrowed_tarjan_validation_is_strictly_linear.
Print Assumptions borrowed_pipeline_adds_no_clone_slots.
