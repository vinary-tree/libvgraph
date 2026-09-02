From Stdlib Require Import List Arith Lia Sorting.Permutation.
Import ListNotations.
Set Implicit Arguments.

Require Import GraphQuotient.

(** This file specifies evidence which is stored beside the payload-free CSR.
    The graph remains structural: provenance values are opaque and no theorem
    assigns them source-domain meaning. *)

Record canonicalizer_laws {P : Type} (canonicalize : list P -> list P) : Prop := {
  canonicalizer_membership :
    forall values value,
      In value (canonicalize values) <-> In value values;
  canonicalizer_nodup :
    forall values, NoDup (canonicalize values);
  canonicalizer_extensional :
    forall first second,
      (forall value, In value first <-> In value second) ->
      canonicalize first = canonicalize second
}.

Definition slot_union {P : Type}
    (canonicalize : list P -> list P) (left right : list P) : list P :=
  canonicalize (left ++ right).

Theorem slot_union_membership :
  forall (P : Type) (canonicalize : list P -> list P),
    canonicalizer_laws canonicalize ->
    forall left right value,
      In value (slot_union canonicalize left right) <->
      In value left \/ In value right.
Proof.
  intros P canonicalize Hcanonical left right value.
  unfold slot_union.
  rewrite (canonicalizer_membership Hcanonical).
  apply in_app_iff.
Qed.

Theorem slot_union_commutative :
  forall (P : Type) (canonicalize : list P -> list P),
    canonicalizer_laws canonicalize ->
    forall left right,
      slot_union canonicalize left right =
      slot_union canonicalize right left.
Proof.
  intros P canonicalize Hcanonical left right.
  unfold slot_union.
  apply (canonicalizer_extensional Hcanonical).
  intro value.
  rewrite !in_app_iff.
  tauto.
Qed.

Theorem slot_union_associative :
  forall (P : Type) (canonicalize : list P -> list P),
    canonicalizer_laws canonicalize ->
    forall first second third,
      slot_union canonicalize (slot_union canonicalize first second) third =
      slot_union canonicalize first (slot_union canonicalize second third).
Proof.
  intros P canonicalize Hcanonical first second third.
  unfold slot_union.
  apply (canonicalizer_extensional Hcanonical).
  intro value.
  rewrite !in_app_iff.
  rewrite !(canonicalizer_membership Hcanonical).
  rewrite !in_app_iff.
  tauto.
Qed.

Theorem slot_union_idempotent :
  forall (P : Type) (canonicalize : list P -> list P),
    canonicalizer_laws canonicalize ->
    forall values,
      slot_union canonicalize values values = canonicalize values.
Proof.
  intros P canonicalize Hcanonical values.
  unfold slot_union.
  apply (canonicalizer_extensional Hcanonical).
  intro value.
  rewrite in_app_iff.
  tauto.
Qed.

Theorem slot_union_duplicate_free :
  forall (P : Type) (canonicalize : list P -> list P),
    canonicalizer_laws canonicalize ->
    forall left right,
      NoDup (slot_union canonicalize left right).
Proof.
  intros P canonicalize Hcanonical left right.
  unfold slot_union.
  apply (canonicalizer_nodup Hcanonical).
Qed.

(** A logical sidecar is total over canonical edge indices and may have an
    empty provenance fiber. A flat implementation refines it with offsets and
    one member array. *)
Definition sidecar {P : Type} := nat -> P -> Prop.

Definition sidecar_union {P : Type}
    (left right : @sidecar P) : @sidecar P :=
  fun edge_index value => left edge_index value \/ right edge_index value.

Definition sidecar_empty {P : Type} : @sidecar P :=
  fun _ _ => False.

Definition sidecar_bounded {P : Type}
    (edge_count : nat) (values : @sidecar P) : Prop :=
  forall edge_index value, values edge_index value -> edge_index < edge_count.

Theorem sidecar_union_membership :
  forall (P : Type) (left right : @sidecar P) edge_index value,
    sidecar_union left right edge_index value <->
    left edge_index value \/ right edge_index value.
Proof.
  intros P left right edge_index value.
  reflexivity.
Qed.

Theorem sidecar_union_associative :
  forall (P : Type) (first second third : @sidecar P) edge_index value,
    sidecar_union (sidecar_union first second) third edge_index value <->
    sidecar_union first (sidecar_union second third) edge_index value.
Proof.
  intros P first second third edge_index value.
  unfold sidecar_union.
  tauto.
Qed.

Theorem sidecar_union_commutative :
  forall (P : Type) (left right : @sidecar P) edge_index value,
    sidecar_union left right edge_index value <->
    sidecar_union right left edge_index value.
Proof.
  intros P left right edge_index value.
  unfold sidecar_union.
  tauto.
Qed.

Theorem sidecar_union_idempotent :
  forall (P : Type) (values : @sidecar P) edge_index value,
    sidecar_union values values edge_index value <-> values edge_index value.
Proof.
  intros P values edge_index value.
  unfold sidecar_union.
  tauto.
Qed.

Theorem sidecar_union_preserves_bounds :
  forall (P : Type) edge_count (left right : @sidecar P),
    sidecar_bounded edge_count left ->
    sidecar_bounded edge_count right ->
    sidecar_bounded edge_count (sidecar_union left right).
Proof.
  intros P edge_count left right Hleft Hright edge_index value [Hin | Hin].
  - exact (Hleft edge_index value Hin).
  - exact (Hright edge_index value Hin).
Qed.

Definition in_flat_sidecar_slot {P : Type}
    (offsets : list nat) (members : list P)
    (edge_index position : nat) (value : P) : Prop :=
  exists start stop,
    nth_error offsets edge_index = Some start /\
    nth_error offsets (S edge_index) = Some stop /\
    start <= position < stop /\
    nth_error members position = Some value.

Record flat_sidecar_shape {P : Type}
    (edge_count : nat) (offsets : list nat) (members : list P) : Prop := {
  sidecar_offsets_length : length offsets = S edge_count;
  sidecar_offsets_origin : nth_error offsets 0 = Some 0;
  sidecar_offsets_terminal :
    nth_error offsets edge_count = Some (length members);
  sidecar_offsets_monotone :
    forall index start stop,
      nth_error offsets index = Some start ->
      nth_error offsets (S index) = Some stop ->
      start <= stop
}.

Definition flat_sidecar_returned_slots {P : Type}
    (offsets : list nat) (members : list P) : nat :=
  length offsets + length members.

Theorem flat_sidecar_returned_slots_exact :
  forall (P : Type) edge_count offsets (members : list P),
    flat_sidecar_shape edge_count offsets members ->
    flat_sidecar_returned_slots offsets members =
      edge_count + length members + 1.
Proof.
  intros P edge_count offsets members Hshape.
  destruct Hshape.
  unfold flat_sidecar_returned_slots.
  lia.
Qed.

(** Edge-index paths replay without searching for an edge source. At each
    step, the current vertex determines the CSR row and the edge index
    determines the next vertex. *)
Inductive replay {V : Type} (edge_at : nat -> option (V * V))
    : V -> list nat -> V -> Prop :=
| replay_empty : forall vertex, replay edge_at vertex [] vertex
| replay_cons : forall source middle target edge_index suffix,
    edge_at edge_index = Some (source, middle) ->
    replay edge_at middle suffix target ->
    replay edge_at source (edge_index :: suffix) target.

Arguments replay_empty {V edge_at} vertex.
Arguments replay_cons {V edge_at} source middle target edge_index suffix _ _.

Inductive visits {V : Type} (edge_at : nat -> option (V * V))
    : V -> list nat -> V -> Prop :=
| visits_start : forall source path, visits edge_at source path source
| visits_tail : forall source middle path value edge_index,
    edge_at edge_index = Some (source, middle) ->
    visits edge_at middle path value ->
    visits edge_at source (edge_index :: path) value.

Arguments visits_start {V edge_at} source path.
Arguments visits_tail {V edge_at} source middle path value edge_index _ _.

Lemma replay_append :
  forall (V : Type) (edge_at : nat -> option (V * V))
    source middle target prefix suffix,
    replay edge_at source prefix middle ->
    replay edge_at middle suffix target ->
    replay edge_at source (prefix ++ suffix) target.
Proof.
  intros V edge_at source middle target prefix suffix Hprefix Hsuffix.
  induction Hprefix.
  - exact Hsuffix.
  - simpl.
    eapply replay_cons.
    + exact H.
    + apply IHHprefix. exact Hsuffix.
Qed.

Theorem replay_target_is_visited :
  forall (V : Type) (edge_at : nat -> option (V * V))
    source path target,
    replay edge_at source path target ->
    visits edge_at source path target.
Proof.
  intros V edge_at source path target Hreplay.
  induction Hreplay.
  - apply visits_start.
  - eapply visits_tail.
    + exact H.
    + exact IHHreplay.
Qed.

Theorem replay_implies_reachability :
  forall (V : Type) (edge : V -> V -> Prop)
    (edge_at : nat -> option (V * V)),
    (forall edge_index source target,
      edge_at edge_index = Some (source, target) -> edge source target) ->
    forall source path target,
      replay edge_at source path target ->
      reach edge source target.
Proof.
  intros V edge edge_at Hedge source path target Hreplay.
  induction Hreplay.
  - apply reach_refl.
  - eapply reach_trans.
    + apply reach_step. eapply Hedge. exact H.
    + exact IHHreplay.
Qed.

Theorem reachability_has_replay :
  forall (V : Type) (edge : V -> V -> Prop)
    (edge_at : nat -> option (V * V)),
    (forall source target, edge source target ->
      exists edge_index, edge_at edge_index = Some (source, target)) ->
    forall source target,
      reach edge source target ->
      exists path, replay edge_at source path target.
Proof.
  intros V edge edge_at Hcomplete source target Hreach.
  induction Hreach.
  - exists []. apply replay_empty.
  - destruct (Hcomplete source target H) as [edge_index Hedge].
    exists [edge_index].
    eapply replay_cons.
    + exact Hedge.
    + apply replay_empty.
  - destruct IHHreach1 as [prefix Hprefix].
    destruct IHHreach2 as [suffix Hsuffix].
    exists (prefix ++ suffix).
    eapply replay_append; eassumption.
Qed.

Theorem replay_renaming_natural :
  forall (V V2 : Type)
    (edge_at : nat -> option (V * V))
    (edge_at2 : nat -> option (V2 * V2))
    (rename_vertex : V -> V2) (rename_index : nat -> nat),
    (forall edge_index source target,
      edge_at edge_index = Some (source, target) ->
      edge_at2 (rename_index edge_index) =
        Some (rename_vertex source, rename_vertex target)) ->
    forall source path target,
      replay edge_at source path target ->
      replay edge_at2 (rename_vertex source)
        (map rename_index path) (rename_vertex target).
Proof.
  intros V V2 edge_at edge_at2 rename_vertex rename_index Hrename
    source path target Hreplay.
  induction Hreplay.
  - simpl. apply replay_empty.
  - simpl. eapply replay_cons.
    + apply Hrename. exact H.
    + exact IHHreplay.
Qed.

Theorem visits_renaming_natural :
  forall (V V2 : Type)
    (edge_at : nat -> option (V * V))
    (edge_at2 : nat -> option (V2 * V2))
    (rename_vertex : V -> V2) (rename_index : nat -> nat),
    (forall edge_index source target,
      edge_at edge_index = Some (source, target) ->
      edge_at2 (rename_index edge_index) =
        Some (rename_vertex source, rename_vertex target)) ->
    forall source path value,
      visits edge_at source path value ->
      visits edge_at2 (rename_vertex source)
        (map rename_index path) (rename_vertex value).
Proof.
  intros V V2 edge_at edge_at2 rename_vertex rename_index Hrename
    source path value Hvisit.
  induction Hvisit.
  - simpl. apply visits_start.
  - simpl. eapply visits_tail.
    + apply Hrename. exact H.
    + exact IHHvisit.
Qed.

Definition reaches_by {V : Type} (edge_at : nat -> option (V * V))
    (source target : V) : Prop :=
  exists path, replay edge_at source path target.

Definition dominates {V : Type} (edge_at : nat -> option (V * V))
    (root dominator target : V) : Prop :=
  reaches_by edge_at root target /\
  forall path, replay edge_at root path target ->
    visits edge_at root path dominator.

Definition strict_dominates {V : Type}
    (edge_at : nat -> option (V * V))
    (root dominator target : V) : Prop :=
  dominates edge_at root dominator target /\ dominator <> target.

Definition immediate_dominator {V : Type}
    (edge_at : nat -> option (V * V))
    (root parent target : V) : Prop :=
  strict_dominates edge_at root parent target /\
  forall other,
    strict_dominates edge_at root other target ->
    dominates edge_at root other parent.

Theorem root_dominates_every_reachable_vertex :
  forall (V : Type) (edge_at : nat -> option (V * V)) root target,
    reaches_by edge_at root target ->
    dominates edge_at root root target.
Proof.
  intros V edge_at root target Hreachable.
  split.
  - exact Hreachable.
  - intros path Hreplay. apply visits_start.
Qed.

Theorem every_reachable_vertex_dominates_itself :
  forall (V : Type) (edge_at : nat -> option (V * V)) root target,
    reaches_by edge_at root target ->
    dominates edge_at root target target.
Proof.
  intros V edge_at root target Hreachable.
  split.
  - exact Hreachable.
  - intros path Hreplay.
    exact (replay_target_is_visited Hreplay).
Qed.

Theorem immediate_dominator_unique :
  forall (V : Type) (edge_at : nat -> option (V * V)) root,
    (forall left right,
      dominates edge_at root left right ->
      dominates edge_at root right left ->
      left = right) ->
    forall first second target,
      immediate_dominator edge_at root first target ->
      immediate_dominator edge_at root second target ->
      first = second.
Proof.
  intros V edge_at root Hantisymmetric first second target
    [Hfirst Hfirst_closest] [Hsecond Hsecond_closest].
  apply Hantisymmetric.
  - apply Hsecond_closest. exact Hfirst.
  - apply Hfirst_closest. exact Hsecond.
Qed.

Definition dominance_frontier {V : Type}
    (edge_at : nat -> option (V * V))
    (root owner frontier_vertex : V) : Prop :=
  reaches_by edge_at root frontier_vertex /\
  exists predecessor edge_index,
    edge_at edge_index = Some (predecessor, frontier_vertex) /\
    dominates edge_at root owner predecessor /\
    ~ strict_dominates edge_at root owner frontier_vertex.

Theorem dominance_frontier_has_predecessor_witness :
  forall (V : Type) (edge_at : nat -> option (V * V))
    root owner frontier_vertex,
    dominance_frontier edge_at root owner frontier_vertex ->
    reaches_by edge_at root frontier_vertex /\
    exists predecessor edge_index,
      edge_at edge_index = Some (predecessor, frontier_vertex) /\
      dominates edge_at root owner predecessor /\
      ~ strict_dominates edge_at root owner frontier_vertex.
Proof.
  intros V edge_at root owner frontier_vertex Hfrontier.
  exact Hfrontier.
Qed.

Theorem dominance_frontier_natural :
  forall (V V2 : Type)
    (edge_at : nat -> option (V * V))
    (edge_at2 : nat -> option (V2 * V2))
    (rename_vertex : V -> V2) (rename_index : nat -> nat)
    root,
    (forall edge_index source target,
      edge_at edge_index = Some (source, target) ->
      edge_at2 (rename_index edge_index) =
        Some (rename_vertex source, rename_vertex target)) ->
    (forall source target,
      reaches_by edge_at source target ->
      reaches_by edge_at2 (rename_vertex source) (rename_vertex target)) ->
    (forall owner target,
      dominates edge_at root owner target <->
      dominates edge_at2 (rename_vertex root)
        (rename_vertex owner) (rename_vertex target)) ->
    (forall owner target,
      strict_dominates edge_at root owner target <->
      strict_dominates edge_at2 (rename_vertex root)
        (rename_vertex owner) (rename_vertex target)) ->
    forall owner frontier_vertex,
      dominance_frontier edge_at root owner frontier_vertex ->
      dominance_frontier edge_at2 (rename_vertex root)
        (rename_vertex owner) (rename_vertex frontier_vertex).
Proof.
  intros V V2 edge_at edge_at2 rename_vertex rename_index root
    Hedge Hreach Hdominates Hstrict owner frontier_vertex
    [Hreachable [predecessor [edge_index
      [Hedge_at [Howner Hnot_strict]]]]].
  split.
  - apply Hreach. exact Hreachable.
  - exists (rename_vertex predecessor), (rename_index edge_index).
    split.
    + apply Hedge. exact Hedge_at.
    + split.
      * apply (proj1 (Hdominates owner predecessor)). exact Howner.
      * intro Hrenamed_strict.
        apply Hnot_strict.
        apply (proj2 (Hstrict owner frontier_vertex)).
        exact Hrenamed_strict.
Qed.

(** A condensation witness fiber contains every source edge whose endpoints
    map to one distinct component pair. It is natural under any transported
    vertex/component/edge-index bijection. *)
Definition condensation_witness {V C : Type}
    (edge_at : nat -> option (V * V)) (quotient : V -> C)
    (source_component target_component : C) (edge_index : nat) : Prop :=
  exists source target,
    edge_at edge_index = Some (source, target) /\
    quotient source = source_component /\
    quotient target = target_component /\
    source_component <> target_component.

Theorem condensation_witness_is_source_edge :
  forall (V C : Type) (edge : V -> V -> Prop)
    (edge_at : nat -> option (V * V)) (quotient : V -> C),
    (forall edge_index source target,
      edge_at edge_index = Some (source, target) -> edge source target) ->
    forall source_component target_component edge_index,
      condensation_witness edge_at quotient
        source_component target_component edge_index ->
      quotient_edge edge quotient source_component target_component.
Proof.
  intros V C edge edge_at quotient Hedge source_component target_component
    edge_index [source [target [Hedge_at
      [Hsource [Htarget Hdistinct]]]]].
  split.
  - exact Hdistinct.
  - exists source, target.
    repeat split; try assumption.
    apply Hedge with edge_index. exact Hedge_at.
Qed.

Theorem condensation_witness_natural :
  forall (V C V2 C2 : Type)
    (edge_at : nat -> option (V * V))
    (edge_at2 : nat -> option (V2 * V2))
    (quotient : V -> C) (quotient2 : V2 -> C2)
    (rename_vertex : V -> V2) (rename_component : C -> C2)
    (rename_index : nat -> nat),
    (forall edge_index source target,
      edge_at edge_index = Some (source, target) ->
      edge_at2 (rename_index edge_index) =
        Some (rename_vertex source, rename_vertex target)) ->
    (forall vertex,
      quotient2 (rename_vertex vertex) =
        rename_component (quotient vertex)) ->
    (forall left right,
      rename_component left = rename_component right -> left = right) ->
    forall source_component target_component edge_index,
      condensation_witness edge_at quotient
        source_component target_component edge_index ->
      condensation_witness edge_at2 quotient2
        (rename_component source_component)
        (rename_component target_component)
        (rename_index edge_index).
Proof.
  intros V C V2 C2 edge_at edge_at2 quotient quotient2
    rename_vertex rename_component rename_index Hedge Hquotient Hinjective
    source_component target_component edge_index
    [source [target [Hedge_at [Hsource [Htarget Hdistinct]]]]].
  exists (rename_vertex source), (rename_vertex target).
  repeat split.
  - apply Hedge. exact Hedge_at.
  - rewrite Hquotient, Hsource. reflexivity.
  - rewrite Hquotient, Htarget. reflexivity.
  - intro Hequal. apply Hdistinct. apply Hinjective. exact Hequal.
Qed.

Record strict_total_order {A : Type} (before : A -> A -> Prop) : Prop := {
  order_irreflexive : forall value, ~ before value value;
  order_transitive : forall first second third,
    before first second -> before second third -> before first third;
  order_trichotomy : forall left right,
    left = right \/ before left right \/ before right left
}.

Definition least_witness {A : Type}
    (before : A -> A -> Prop) (witness : A -> Prop) (choice : A) : Prop :=
  witness choice /\
  forall alternative, witness alternative -> ~ before alternative choice.

Theorem least_witness_unique :
  forall (A : Type) (before : A -> A -> Prop),
    strict_total_order before ->
    forall witness first second,
      least_witness before witness first ->
      least_witness before witness second ->
      first = second.
Proof.
  intros A before Horder witness first second
    [Hfirst Hfirst_least] [Hsecond Hsecond_least].
  destruct Horder as [Hirreflexive Htransitive Htrichotomy].
  destruct (Htrichotomy first second) as
      [Hequal | [Hfirst_before | Hsecond_before]].
  - exact Hequal.
  - exfalso. exact (Hsecond_least first Hfirst Hfirst_before).
  - exfalso. exact (Hfirst_least second Hsecond Hsecond_before).
Qed.

Theorem least_witness_natural :
  forall (A B : Type)
    (before : A -> A -> Prop) (before2 : B -> B -> Prop)
    (witness : A -> Prop) (witness2 : B -> Prop) (rename : A -> B),
    (forall value, witness value -> witness2 (rename value)) ->
    (forall value2, witness2 value2 ->
      exists value, witness value /\ value2 = rename value) ->
    (forall left right,
      before2 (rename left) (rename right) <-> before left right) ->
    forall choice,
      least_witness before witness choice ->
      least_witness before2 witness2 (rename choice).
Proof.
  intros A B before before2 witness witness2 rename
    Hwitness_forward Hwitness_backward Horder choice
    [Hchoice Hleast].
  split.
  - apply Hwitness_forward. exact Hchoice.
  - intros alternative Halternative Hbefore.
    destruct (Hwitness_backward alternative Halternative) as
      [source [Hsource Hrenamed]].
    subst alternative.
    apply (Hleast source Hsource).
    apply (proj1 (Horder source choice)).
    exact Hbefore.
Qed.

Definition swap_zero_one (value : nat) : nat :=
  match value with
  | 0 => 1
  | 1 => 0
  | S (S rest) => S (S rest)
  end.

(** A two-witness fiber with a symmetry swapping both witnesses has no
    deterministic equivariant selector. Exact single-choice APIs therefore
    require an order policy which is transported by lawful renaming. *)
Theorem unqualified_equivariant_selector_impossible :
  forall choice,
    (choice = 0 \/ choice = 1) ->
    swap_zero_one choice = choice ->
    False.
Proof.
  intros choice [Hchoice | Hchoice] Hfixed;
    subst choice; simpl in Hfixed; discriminate.
Qed.

Inductive witness_outcome (A : Type) : Type :=
| WitnessExact : A -> witness_outcome A
| WitnessUnreachable : witness_outcome A
| WitnessInvalidEdge : nat -> witness_outcome A
| WitnessLimitExceeded : nat -> witness_outcome A
| WitnessCancelled : witness_outcome A.

Arguments WitnessExact {A} _.
Arguments WitnessUnreachable {A}.
Arguments WitnessInvalidEdge {A} _.
Arguments WitnessLimitExceeded {A} _.
Arguments WitnessCancelled {A}.

Theorem incomplete_outcomes_are_not_exact :
  forall (A : Type) (value : A),
    WitnessUnreachable <> WitnessExact value /\
    (forall edge_index, WitnessInvalidEdge edge_index <> WitnessExact value) /\
    (forall limit, WitnessLimitExceeded limit <> WitnessExact value) /\
    WitnessCancelled <> WitnessExact value.
Proof.
  intros A value.
  repeat split; intros; discriminate.
Qed.

(** Resource contracts count logical operations, not machine instructions.
    Returned sidecars, paths, and frontier members are output rather than
    auxiliary workspace. *)
Definition sidecar_union_work
    (edge_count left_members right_members output_members : nat) : nat :=
  2 * edge_count + left_members + right_members + output_members + 1.

Theorem sidecar_union_work_linear :
  forall edge_count left_members right_members output_members,
    output_members <= left_members + right_members ->
    sidecar_union_work edge_count left_members right_members output_members <=
      2 * edge_count + 2 * left_members + 2 * right_members + 1.
Proof.
  intros edge_count left_members right_members output_members Houtput.
  unfold sidecar_union_work.
  lia.
Qed.

Definition reachability_work
    (vertex_count reachable_vertices reachable_edges path_length : nat) : nat :=
  vertex_count + 2 * reachable_vertices + reachable_edges + path_length + 1.

Record reachability_dimensions
    (vertex_count edge_count reachable_vertices reachable_edges
      path_length : nat) : Prop := {
  reachable_vertices_bounded : reachable_vertices <= vertex_count;
  reachable_edges_bounded : reachable_edges <= edge_count;
  simple_path_length_bounded : path_length <= vertex_count
}.

Theorem reachability_work_linear :
  forall vertex_count edge_count reachable_vertices reachable_edges path_length,
    reachability_dimensions vertex_count edge_count reachable_vertices
      reachable_edges path_length ->
    reachability_work vertex_count reachable_vertices reachable_edges path_length <=
      4 * vertex_count + edge_count + 1.
Proof.
  intros vertex_count edge_count reachable_vertices reachable_edges path_length
    Hdimensions.
  destruct Hdimensions.
  unfold reachability_work.
  lia.
Qed.

Definition lengauer_tarjan_work
    (vertex_count edge_count link_eval_work : nat) : nat :=
  8 * vertex_count + 2 * edge_count + link_eval_work + 1.

Theorem lengauer_tarjan_near_linear :
  forall vertex_count edge_count link_eval_work inverse_ackermann_factor,
    link_eval_work <=
      inverse_ackermann_factor * (vertex_count + edge_count) ->
    lengauer_tarjan_work vertex_count edge_count link_eval_work <=
      (8 + inverse_ackermann_factor) * vertex_count +
      (2 + inverse_ackermann_factor) * edge_count + 1.
Proof.
  intros vertex_count edge_count link_eval_work inverse_ackermann_factor Hlink.
  unfold lengauer_tarjan_work.
  nia.
Qed.

Definition dominance_frontier_work
    (vertex_count edge_count candidate_count output_count : nat) : nat :=
  4 * vertex_count + 2 * edge_count + candidate_count + output_count + 1.

Theorem dominance_frontier_output_sensitive :
  forall vertex_count edge_count candidate_count output_count,
    candidate_count <= edge_count ->
    dominance_frontier_work vertex_count edge_count candidate_count output_count <=
      4 * vertex_count + 3 * edge_count + output_count + 1.
Proof.
  intros vertex_count edge_count candidate_count output_count Hcandidate.
  unfold dominance_frontier_work.
  lia.
Qed.

Record witness_control_shape := {
  witness_recursive_control_edges : nat;
  witness_resident_native_frames : nat;
  witness_heap_frames : nat
}.

Definition witness_stack_safe
    (vertex_count : nat) (shape : witness_control_shape) : Prop :=
  witness_recursive_control_edges shape = 0 /\
  witness_resident_native_frames shape <= 1 /\
  witness_heap_frames shape <= vertex_count.

Theorem witness_native_stack_constant :
  forall vertex_count shape,
    witness_stack_safe vertex_count shape ->
    witness_resident_native_frames shape <= 1.
Proof.
  intros vertex_count shape [_ [Hframes _]].
  exact Hframes.
Qed.

Print Assumptions slot_union_commutative.
Print Assumptions slot_union_associative.
Print Assumptions slot_union_idempotent.
Print Assumptions sidecar_union_preserves_bounds.
Print Assumptions flat_sidecar_returned_slots_exact.
Print Assumptions replay_append.
Print Assumptions replay_target_is_visited.
Print Assumptions replay_implies_reachability.
Print Assumptions reachability_has_replay.
Print Assumptions replay_renaming_natural.
Print Assumptions root_dominates_every_reachable_vertex.
Print Assumptions every_reachable_vertex_dominates_itself.
Print Assumptions immediate_dominator_unique.
Print Assumptions dominance_frontier_natural.
Print Assumptions condensation_witness_is_source_edge.
Print Assumptions condensation_witness_natural.
Print Assumptions least_witness_unique.
Print Assumptions least_witness_natural.
Print Assumptions unqualified_equivariant_selector_impossible.
Print Assumptions incomplete_outcomes_are_not_exact.
Print Assumptions sidecar_union_work_linear.
Print Assumptions reachability_work_linear.
Print Assumptions lengauer_tarjan_near_linear.
Print Assumptions dominance_frontier_output_sensitive.
Print Assumptions witness_native_stack_constant.
