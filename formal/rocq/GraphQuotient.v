From Stdlib Require Import List Arith Lia Sorting.Permutation.
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
