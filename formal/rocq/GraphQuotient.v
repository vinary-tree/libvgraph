From Stdlib Require Import List Arith Lia Ring Sorting.Permutation.
Import ListNotations.
Set Implicit Arguments.

Inductive reach {A : Type} (relation : A -> A -> Prop) : A -> A -> Prop :=
| reach_refl : forall value, reach relation value value
| reach_step : forall source target, relation source target -> reach relation source target
| reach_trans : forall source middle target,
    reach relation source middle ->
    reach relation middle target ->
    reach relation source target.

Arguments reach_refl {A relation} value.
Arguments reach_step {A relation} source target _.
Arguments reach_trans {A relation} source middle target _ _.

Definition strongly_connected {V : Type} (edge : V -> V -> Prop) (left right : V) : Prop :=
  reach edge left right /\ reach edge right left.

Lemma strongly_connected_reflexive :
  forall (V : Type) (edge : V -> V -> Prop) (value : V),
    strongly_connected edge value value.
Proof.
  intros V edge value.
  split; apply reach_refl.
Qed.

Lemma strongly_connected_symmetric :
  forall (V : Type) (edge : V -> V -> Prop) (left right : V),
    strongly_connected edge left right -> strongly_connected edge right left.
Proof.
  intros V edge left right [Hleft_right Hright_left].
  split; assumption.
Qed.

Lemma strongly_connected_transitive :
  forall (V : Type) (edge : V -> V -> Prop) (left middle right : V),
    strongly_connected edge left middle ->
    strongly_connected edge middle right ->
    strongly_connected edge left right.
Proof.
  intros V edge left middle right [Hleft_middle Hmiddle_left]
    [Hmiddle_right Hright_middle].
  split.
  - exact (reach_trans left middle right Hleft_middle Hmiddle_right).
  - exact (reach_trans right middle left Hright_middle Hmiddle_left).
Qed.

Record scc_quotient_laws {V C : Type}
    (edge : V -> V -> Prop) (quotient : V -> C) : Prop := {
  quotient_surjective :
    forall component : C, exists vertex : V, quotient vertex = component;
  quotient_exact_kernel :
    forall left right : V,
      quotient left = quotient right <-> strongly_connected edge left right
}.

Theorem scc_quotient_kernel_exact :
  forall (V C : Type) (edge : V -> V -> Prop) (quotient : V -> C),
    scc_quotient_laws edge quotient ->
    forall left right : V,
      quotient left = quotient right <-> strongly_connected edge left right.
Proof.
  intros V C edge quotient Hlaws.
  exact (quotient_exact_kernel Hlaws).
Qed.

Theorem scc_quotient_fibers_nonempty :
  forall (V C : Type) (edge : V -> V -> Prop) (quotient : V -> C),
    scc_quotient_laws edge quotient ->
    forall component : C, exists vertex : V, quotient vertex = component.
Proof.
  intros V C edge quotient Hlaws.
  exact (quotient_surjective Hlaws).
Qed.

Definition fiber {V C : Type} (quotient : V -> C) (component : C) (vertex : V) : Prop :=
  quotient vertex = component.

Theorem fiber_total :
  forall (V C : Type) (quotient : V -> C) (vertex : V),
    fiber quotient (quotient vertex) vertex.
Proof.
  intros V C quotient vertex.
  reflexivity.
Qed.

Theorem fibers_disjoint :
  forall (V C : Type) (quotient : V -> C) (vertex : V) (left right : C),
    fiber quotient left vertex -> fiber quotient right vertex -> left = right.
Proof.
  intros V C quotient vertex left right Hleft Hright.
  unfold fiber in Hleft, Hright.
  congruence.
Qed.

Definition quotient_edge {V C : Type}
    (edge : V -> V -> Prop) (quotient : V -> C) (source target : C) : Prop :=
  source <> target /\
  exists source_vertex target_vertex,
    quotient source_vertex = source /\
    quotient target_vertex = target /\
    edge source_vertex target_vertex.

Definition quotient_reach {V C : Type}
    (edge : V -> V -> Prop) (quotient : V -> C) : C -> C -> Prop :=
  reach (quotient_edge edge quotient).

Theorem quotient_edge_complete :
  forall (V C : Type) (edge : V -> V -> Prop) (quotient : V -> C)
    (source_vertex target_vertex : V),
    edge source_vertex target_vertex ->
    quotient source_vertex <> quotient target_vertex ->
    quotient_edge edge quotient
      (quotient source_vertex) (quotient target_vertex).
Proof.
  intros V C edge quotient source_vertex target_vertex Hedge Hdistinct.
  split.
  - exact Hdistinct.
  - exists source_vertex, target_vertex.
    repeat split; try reflexivity; exact Hedge.
Qed.

Theorem quotient_edge_has_witness :
  forall (V C : Type) (edge : V -> V -> Prop) (quotient : V -> C)
    (source target : C),
    quotient_edge edge quotient source target ->
    source <> target /\
    exists source_vertex target_vertex,
      quotient source_vertex = source /\
      quotient target_vertex = target /\
      edge source_vertex target_vertex.
Proof.
  intros V C edge quotient source target Hedge.
  exact Hedge.
Qed.

Theorem quotient_edge_has_no_self_loop :
  forall (V C : Type) (edge : V -> V -> Prop) (quotient : V -> C) (component : C),
    ~ quotient_edge edge quotient component component.
Proof.
  intros V C edge quotient component [Hdistinct _].
  apply Hdistinct.
  reflexivity.
Qed.

Theorem quotient_reach_lifts :
  forall (V C : Type) (edge : V -> V -> Prop) (quotient : V -> C),
    (forall component : C, exists vertex : V, quotient vertex = component) ->
    (forall left right : V,
      quotient left = quotient right -> strongly_connected edge left right) ->
    forall source target : C,
      quotient_reach edge quotient source target ->
      exists source_vertex target_vertex,
        quotient source_vertex = source /\
        quotient target_vertex = target /\
        reach edge source_vertex target_vertex.
Proof.
  intros V C edge quotient Hsurjective Hkernel source target Hpath.
  induction Hpath as
      [component
      | source_component target_component Hedge
      | source_component middle_component target_component
          Hsource_middle IHsource_middle Hmiddle_target IHmiddle_target].
  - destruct (Hsurjective component) as [vertex Hvertex].
    exists vertex, vertex.
    repeat split; try assumption.
    apply reach_refl.
  - destruct Hedge as
        [_ [source_vertex [target_vertex [Hsource [Htarget Hedge]]]]].
    exists source_vertex, target_vertex.
    repeat split; try assumption.
    exact (reach_step source_vertex target_vertex Hedge).
  - destruct IHsource_middle as
        [source_vertex [left_middle [Hsource [Hleft_middle Hreach_left]]]].
    destruct IHmiddle_target as
        [right_middle [target_vertex [Hright_middle [Htarget Hreach_right]]]].
    assert (Hsame_middle : quotient left_middle = quotient right_middle) by congruence.
    destruct (Hkernel left_middle right_middle Hsame_middle) as [Hbridge _].
    exists source_vertex, target_vertex.
    repeat split; try assumption.
    exact (reach_trans source_vertex left_middle target_vertex Hreach_left
      (reach_trans left_middle right_middle target_vertex Hbridge Hreach_right)).
Qed.

Theorem quotient_reach_antisymmetric :
  forall (V C : Type) (edge : V -> V -> Prop) (quotient : V -> C),
    scc_quotient_laws edge quotient ->
    forall left right : C,
      quotient_reach edge quotient left right ->
      quotient_reach edge quotient right left ->
      left = right.
Proof.
  intros V C edge quotient Hlaws left right Hleft_right Hright_left.
  destruct Hlaws as [Hsurjective Hkernel].
  assert (Hkernel_forward : forall x y : V,
      quotient x = quotient y -> strongly_connected edge x y).
  { intros x y Hequal. apply (proj1 (Hkernel x y)). exact Hequal. }
  destruct (quotient_reach_lifts
      (edge := edge) (quotient := quotient)
      Hsurjective Hkernel_forward Hleft_right) as
    [left_vertex [right_vertex [Hleft [Hright Hreach_left_right]]]].
  destruct (quotient_reach_lifts
      (edge := edge) (quotient := quotient)
      Hsurjective Hkernel_forward Hright_left) as
    [right_start [left_end [Hright_start [Hleft_end Hreach_right_left]]]].
  assert (Hright_bridge : strongly_connected edge right_vertex right_start).
  { apply Hkernel_forward. congruence. }
  assert (Hleft_bridge : strongly_connected edge left_end left_vertex).
  { apply Hkernel_forward. congruence. }
  destruct Hright_bridge as [Hright_to_start _].
  destruct Hleft_bridge as [Hend_to_left _].
  assert (Hreach_back : reach edge right_vertex left_vertex).
  {
    exact (reach_trans right_vertex right_start left_vertex Hright_to_start
      (reach_trans right_start left_end left_vertex Hreach_right_left Hend_to_left)).
  }
  assert (Hsame_component : quotient left_vertex = quotient right_vertex).
  {
    apply (proj2 (Hkernel left_vertex right_vertex)).
    split; assumption.
  }
  congruence.
Qed.

Theorem quotient_edge_natural :
  forall (V C V2 C2 : Type)
    (edge : V -> V -> Prop) (edge2 : V2 -> V2 -> Prop)
    (quotient : V -> C) (quotient2 : V2 -> C2)
    (rename_vertex : V -> V2) (rename_component : C -> C2),
    (forall source target, edge source target ->
      edge2 (rename_vertex source) (rename_vertex target)) ->
    (forall vertex, quotient2 (rename_vertex vertex) =
      rename_component (quotient vertex)) ->
    (forall left right, rename_component left = rename_component right -> left = right) ->
    forall source target,
      quotient_edge edge quotient source target ->
      quotient_edge edge2 quotient2
        (rename_component source) (rename_component target).
Proof.
  intros V C V2 C2 edge edge2 quotient quotient2 rename_vertex rename_component
    Hedge Hcommutes Hinjective source target
    [Hdistinct [source_vertex [target_vertex [Hsource [Htarget Hsource_edge]]]]].
  split.
  - intro Hrenamed_equal.
    apply Hdistinct.
    exact (Hinjective source target Hrenamed_equal).
  - exists (rename_vertex source_vertex), (rename_vertex target_vertex).
    repeat split.
    + rewrite Hcommutes, Hsource. reflexivity.
    + rewrite Hcommutes, Htarget. reflexivity.
    + apply Hedge. exact Hsource_edge.
Qed.

Theorem quotient_edge_rename_equivalent :
  forall (V C V2 C2 : Type)
    (edge : V -> V -> Prop) (edge2 : V2 -> V2 -> Prop)
    (quotient : V -> C) (quotient2 : V2 -> C2)
    (rename_vertex : V -> V2) (unrename_vertex : V2 -> V)
    (rename_component : C -> C2) (unrename_component : C2 -> C),
    (forall source target, edge source target ->
      edge2 (rename_vertex source) (rename_vertex target)) ->
    (forall source target, edge2 source target ->
      edge (unrename_vertex source) (unrename_vertex target)) ->
    (forall vertex, quotient2 (rename_vertex vertex) =
      rename_component (quotient vertex)) ->
    (forall vertex, quotient (unrename_vertex vertex) =
      unrename_component (quotient2 vertex)) ->
    (forall left right, rename_component left = rename_component right ->
      left = right) ->
    (forall left right, unrename_component left = unrename_component right ->
      left = right) ->
    (forall component, unrename_component (rename_component component) = component) ->
    forall source target,
      quotient_edge edge quotient source target <->
      quotient_edge edge2 quotient2
        (rename_component source) (rename_component target).
Proof.
  intros V C V2 C2 edge edge2 quotient quotient2
    rename_vertex unrename_vertex rename_component unrename_component
    Hedge_forward Hedge_backward Hcommutes_forward Hcommutes_backward
    Hrename_injective Hunrename_injective Hcomponent_inverse source target.
  split.
  - intro Hsource_edge.
    exact (@quotient_edge_natural V C V2 C2
      edge edge2 quotient quotient2 rename_vertex rename_component
      Hedge_forward Hcommutes_forward Hrename_injective
      source target Hsource_edge).
  - intro Hrenamed_edge.
    pose proof (@quotient_edge_natural V2 C2 V C
      edge2 edge quotient2 quotient unrename_vertex unrename_component
      Hedge_backward Hcommutes_backward Hunrename_injective
      (rename_component source) (rename_component target)
      Hrenamed_edge) as Hbackward.
    rewrite !Hcomponent_inverse in Hbackward.
    exact Hbackward.
Qed.

Definition enumerated_edge {V : Type} (edges : list (V * V)) (source target : V) : Prop :=
  In (source, target) edges.

Theorem edge_enumeration_permutation_invariant :
  forall (V : Type) (first second : list (V * V)),
    Permutation first second ->
    forall source target,
      enumerated_edge first source target <-> enumerated_edge second source target.
Proof.
  intros V first second Hpermutation source target.
  unfold enumerated_edge.
  split; intro Hin.
  - exact (Permutation_in (source, target) Hpermutation Hin).
  - exact (Permutation_in (source, target) (Permutation_sym Hpermutation) Hin).
Qed.

Theorem edge_enumeration_duplicate_invariant :
  forall (V : Type) (edges : list (V * V)) (source target : V),
    enumerated_edge (edges ++ edges) source target <->
    enumerated_edge edges source target.
Proof.
  intros V edges source target.
  unfold enumerated_edge.
  rewrite in_app_iff.
  tauto.
Qed.

Theorem quotient_edge_extensional_invariant :
  forall (V C : Type) (first second : V -> V -> Prop) (quotient : V -> C),
    (forall source target, first source target <-> second source target) ->
    forall source target,
      quotient_edge first quotient source target <->
      quotient_edge second quotient source target.
Proof.
  intros V C first second quotient Hextensional source target.
  split; intros [Hdistinct [source_vertex [target_vertex
      [Hsource [Htarget Hedge]]]]].
  - split; [exact Hdistinct |].
    exists source_vertex, target_vertex.
    repeat split; try assumption.
    apply (proj1 (Hextensional source_vertex target_vertex)).
    exact Hedge.
  - split; [exact Hdistinct |].
    exists source_vertex, target_vertex.
    repeat split; try assumption.
    apply (proj2 (Hextensional source_vertex target_vertex)).
    exact Hedge.
Qed.

Theorem same_wavefront_has_no_dependency :
  forall (V C : Type) (edge : V -> V -> Prop) (quotient : V -> C)
    (rank : C -> nat),
    (forall source target,
      quotient_edge edge quotient source target -> rank source < rank target) ->
    forall left right,
      rank left = rank right ->
      ~ quotient_edge edge quotient left right /\
      ~ quotient_edge edge quotient right left.
Proof.
  intros V C edge quotient rank Hincreases left right Hequal.
  split; intro Hedge.
  - specialize (Hincreases left right Hedge). lia.
  - specialize (Hincreases right left Hedge). lia.
Qed.

(** Logical work performed by one complete explicit-frame Tarjan traversal.

    [root_checks] counts the dense outer-loop positions, [discoveries] counts
    first visits, [edge_inspections] counts canonical CSR entries exactly once,
    [frame_finishes] counts explicit DFS-frame removals, [active_pops] counts
    removals from Tarjan's active stack, and [canonical_assignments] counts the
    final ascending dense-vertex scan that assigns canonical component ids and
    produces sorted fibers without comparison sorting. *)
Record tarjan_trace_counts := {
  root_checks : nat;
  discoveries : nat;
  edge_inspections : nat;
  frame_finishes : nat;
  active_pops : nat;
  canonical_assignments : nat
}.

Definition tarjan_work (counts : tarjan_trace_counts) : nat :=
  root_checks counts + discoveries counts + edge_inspections counts +
  frame_finishes counts + active_pops counts + canonical_assignments counts.

Record complete_tarjan_trace
    (vertex_count edge_count : nat) (counts : tarjan_trace_counts) : Prop := {
  roots_accounted : root_checks counts = vertex_count;
  discoveries_accounted : discoveries counts = vertex_count;
  edges_accounted : edge_inspections counts = edge_count;
  frames_accounted : frame_finishes counts = vertex_count;
  active_accounted : active_pops counts = vertex_count;
  canonical_assignments_accounted : canonical_assignments counts = vertex_count
}.

Theorem complete_tarjan_work_exact :
  forall vertex_count edge_count counts,
    complete_tarjan_trace vertex_count edge_count counts ->
    tarjan_work counts = 5 * vertex_count + edge_count.
Proof.
  intros vertex_count edge_count counts Htrace.
  destruct Htrace.
  unfold tarjan_work.
  lia.
Qed.

Theorem complete_tarjan_work_linear :
  forall vertex_count edge_count counts,
    complete_tarjan_trace vertex_count edge_count counts ->
    tarjan_work counts <= 5 * vertex_count + edge_count.
Proof.
  intros vertex_count edge_count counts Htrace.
  rewrite (@complete_tarjan_work_exact vertex_count edge_count counts Htrace).
  lia.
Qed.

(** Peak auxiliary heap slots for Tarjan, excluding the returned partition.
    Three dense arrays have exactly [vertex_count] slots. The active stack and
    explicit DFS-frame stack never exceed [vertex_count]. *)
Record tarjan_heap_counts := {
  discovery_slots : nat;
  low_link_slots : nat;
  raw_component_slots : nat;
  peak_active_slots : nat;
  peak_frame_slots : nat
}.

Definition tarjan_heap_slots (counts : tarjan_heap_counts) : nat :=
  discovery_slots counts + low_link_slots counts + raw_component_slots counts +
  peak_active_slots counts + peak_frame_slots counts.

Record bounded_tarjan_heap
    (vertex_count : nat) (counts : tarjan_heap_counts) : Prop := {
  discovery_slots_exact : discovery_slots counts = vertex_count;
  low_link_slots_exact : low_link_slots counts = vertex_count;
  raw_component_slots_exact : raw_component_slots counts = vertex_count;
  active_slots_bounded : peak_active_slots counts <= vertex_count;
  frame_slots_bounded : peak_frame_slots counts <= vertex_count
}.

Theorem tarjan_auxiliary_heap_linear :
  forall vertex_count counts,
    bounded_tarjan_heap vertex_count counts ->
    tarjan_heap_slots counts <= 5 * vertex_count.
Proof.
  intros vertex_count counts Hheap.
  destruct Hheap.
  unfold tarjan_heap_slots.
  lia.
Qed.

(** A stack-safe refinement has no recursive control edge and retains one
    native entry frame independent of graph size. All graph-depth state belongs
    to the bounded heap structures above. *)
Record iterative_control_shape := {
  recursive_control_edges : nat;
  resident_native_frames : nat
}.

Definition stack_safe_control (shape : iterative_control_shape) : Prop :=
  recursive_control_edges shape = 0 /\ resident_native_frames shape <= 1.

Theorem iterative_control_native_stack_constant :
  forall shape,
    stack_safe_control shape ->
    forall (vertex_count edge_count : nat),
      resident_native_frames shape <= 1.
Proof.
  intros shape [_ Hframes] vertex_count edge_count.
  exact Hframes.
Qed.

(** The dense quotient canonicalizer uses six 11-bit least-significant-digit
    radix passes over [u64] component pairs. Each full pass scans the candidates
    twice and the 2,048-entry bucket array twice: once to clear it and once to
    form prefixes. The final deduplication scans the candidate vector once.
    Inputs with fewer than two candidates take the exact zero-work fast path. *)
Definition quotient_radix_bucket_count : nat := 2048.

Definition quotient_radix_pass_count : nat := 6.

Definition quotient_radix_fixed_work : nat :=
  quotient_radix_pass_count * (2 * quotient_radix_bucket_count).

Definition quotient_radix_work (candidate_count : nat) : nat :=
  if candidate_count <? 2 then 0
  else
    quotient_radix_pass_count *
      (2 * candidate_count + 2 * quotient_radix_bucket_count) +
    candidate_count.

Theorem quotient_radix_work_small_exact :
  forall candidate_count,
    candidate_count < 2 ->
    quotient_radix_work candidate_count = 0.
Proof.
  intros candidate_count Hsmall.
  unfold quotient_radix_work.
  apply Nat.ltb_lt in Hsmall.
  rewrite Hsmall.
  reflexivity.
Qed.

Theorem quotient_radix_work_full_exact :
  forall candidate_count,
    2 <= candidate_count ->
    quotient_radix_work candidate_count =
      13 * candidate_count + quotient_radix_fixed_work.
Proof.
  intros candidate_count Hfull.
  unfold quotient_radix_work, quotient_radix_fixed_work,
    quotient_radix_pass_count.
  destruct (candidate_count <? 2) eqn:Hsmall.
  - apply Nat.ltb_lt in Hsmall.
    lia.
  - ring.
Qed.

Theorem quotient_radix_work_upper_bound :
  forall candidate_count,
    quotient_radix_work candidate_count <=
      13 * candidate_count + quotient_radix_fixed_work.
Proof.
  intros candidate_count.
  destruct (candidate_count <? 2) eqn:Hsmall.
  - apply Nat.ltb_lt in Hsmall.
    rewrite quotient_radix_work_small_exact by exact Hsmall.
    lia.
  - apply Nat.ltb_ge in Hsmall.
    rewrite quotient_radix_work_full_exact by exact Hsmall.
    lia.
Qed.

(** The complete quotient and linear-wavefront pipeline adds one source-edge
    scan, the exact radix cost above, one scan per distinct quotient edge, and
    three visits per component (indegree initialization, ready removal, and
    wave assignment) to the exact Tarjan cost. *)
Definition quotient_wavefront_work
    (vertex_count edge_count candidate_count component_count
      quotient_edge_count : nat) : nat :=
  5 * vertex_count + 2 * edge_count +
  quotient_radix_work candidate_count +
  3 * component_count + quotient_edge_count.

Record quotient_dimensions
    (vertex_count edge_count candidate_count component_count
      quotient_edge_count : nat) : Prop := {
  candidate_count_bounded : candidate_count <= edge_count;
  component_count_bounded : component_count <= vertex_count;
  quotient_edge_count_bounded : quotient_edge_count <= edge_count
}.

Theorem quotient_wavefront_work_linear :
  forall vertex_count edge_count candidate_count component_count
    quotient_edge_count,
    quotient_dimensions vertex_count edge_count candidate_count component_count
      quotient_edge_count ->
    quotient_wavefront_work vertex_count edge_count candidate_count component_count
      quotient_edge_count <=
        8 * vertex_count + 16 * edge_count + quotient_radix_fixed_work.
Proof.
  intros vertex_count edge_count candidate_count component_count
    quotient_edge_count Hdimensions.
  destruct Hdimensions.
  pose proof (quotient_radix_work_upper_bound candidate_count) as Hradix.
  unfold quotient_wavefront_work.
  lia.
Qed.

Print Assumptions strongly_connected_transitive.
Print Assumptions scc_quotient_kernel_exact.
Print Assumptions scc_quotient_fibers_nonempty.
Print Assumptions fiber_total.
Print Assumptions fibers_disjoint.
Print Assumptions quotient_edge_complete.
Print Assumptions quotient_edge_has_witness.
Print Assumptions quotient_reach_lifts.
Print Assumptions quotient_reach_antisymmetric.
Print Assumptions quotient_edge_natural.
Print Assumptions quotient_edge_rename_equivalent.
Print Assumptions edge_enumeration_permutation_invariant.
Print Assumptions edge_enumeration_duplicate_invariant.
Print Assumptions quotient_edge_extensional_invariant.
Print Assumptions same_wavefront_has_no_dependency.
Print Assumptions complete_tarjan_work_exact.
Print Assumptions complete_tarjan_work_linear.
Print Assumptions tarjan_auxiliary_heap_linear.
Print Assumptions iterative_control_native_stack_constant.
Print Assumptions quotient_radix_work_small_exact.
Print Assumptions quotient_radix_work_full_exact.
Print Assumptions quotient_radix_work_upper_bound.
Print Assumptions quotient_wavefront_work_linear.
