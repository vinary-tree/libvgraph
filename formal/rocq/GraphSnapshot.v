From Stdlib Require Import List Arith Lia Bool PeanoNat.
Import ListNotations.
Set Implicit Arguments.

(** The byte-level refinement uses an 80-byte header followed by fixed-width
    little-endian [u32] words.  This Rocq layer proves the corresponding flat
    word-tape semantics; the exhaustive executable model proves the byte
    refinement and golden vectors. *)

Definition magic_word : nat := 42.
Definition schema_word : nat := 1.
Definition schema_major : nat := 1.
Definition schema_minor : nat := 0.
Definition digest_domain : list nat := [76; 86; 71; 73; 45; 68; 73; 71; 69; 83; 84].

Record graph_snapshot : Type := {
  snapshot_vertices : nat;
  snapshot_offsets : list nat;
  snapshot_targets : list nat;
  snapshot_profile : nat
}.

Fixpoint nondecreasingb (values : list nat) : bool :=
  match values with
  | [] | [_] => true
  | previous :: ((next_value :: _) as tail) =>
      Nat.leb previous next_value && nondecreasingb tail
  end.

Fixpoint strictly_increasingb (values : list nat) : bool :=
  match values with
  | [] | [_] => true
  | previous :: ((next_value :: _) as tail) =>
      Nat.ltb previous next_value && strictly_increasingb tail
  end.

Fixpoint adjacency_slices_orderedb
    (offsets targets : list nat) : bool :=
  match offsets with
  | first_offset :: ((next_offset :: _) as tail) =>
      strictly_increasingb
        (firstn (next_offset - first_offset) (skipn first_offset targets)) &&
      adjacency_slices_orderedb tail targets
  | _ => true
  end.

Definition offset_originb (offsets : list nat) : bool :=
  match offsets with
  | 0 :: _ => true
  | _ => false
  end.

Definition canonicalb (snapshot : graph_snapshot) : bool :=
  Nat.eqb (length snapshot.(snapshot_offsets))
    (S snapshot.(snapshot_vertices)) &&
  (offset_originb snapshot.(snapshot_offsets) &&
  (Nat.eqb (last snapshot.(snapshot_offsets) 0)
    (length snapshot.(snapshot_targets)) &&
  (nondecreasingb snapshot.(snapshot_offsets) &&
  (forallb
    (fun target => Nat.ltb target snapshot.(snapshot_vertices))
    snapshot.(snapshot_targets) &&
   adjacency_slices_orderedb
    snapshot.(snapshot_offsets) snapshot.(snapshot_targets))))).

Definition encode_words (snapshot : graph_snapshot) : list nat :=
  [magic_word; schema_word; schema_major; schema_minor;
   snapshot.(snapshot_profile);
   snapshot.(snapshot_vertices);
   length snapshot.(snapshot_targets)] ++
  snapshot.(snapshot_offsets) ++ snapshot.(snapshot_targets).

Definition decode_words
    (expected_profile : nat) (words : list nat) : option graph_snapshot :=
  match words with
  | magic :: schema :: major :: minor :: profile :: vertices :: edges :: payload =>
      if Nat.eqb magic magic_word &&
         (Nat.eqb schema schema_word &&
         (Nat.eqb major schema_major &&
         (Nat.eqb minor schema_minor &&
          Nat.eqb profile expected_profile)))
      then
        let offsets := firstn (S vertices) payload in
        let targets := skipn (S vertices) payload in
        let candidate := {|
          snapshot_vertices := vertices;
          snapshot_offsets := offsets;
          snapshot_targets := targets;
          snapshot_profile := profile
        |} in
        if Nat.eqb (length offsets) (S vertices) &&
           (Nat.eqb (length targets) edges && canonicalb candidate)
        then Some candidate
        else None
      else None
  | _ => None
  end.

Lemma firstn_exact_app :
  forall (A : Type) (prefix suffix : list A),
    firstn (length prefix) (prefix ++ suffix) = prefix.
Proof.
  intros A prefix.
  induction prefix as [|head tail IH]; intros suffix; simpl.
  - reflexivity.
  - rewrite IH. reflexivity.
Qed.

Lemma skipn_exact_app :
  forall (A : Type) (prefix suffix : list A),
    skipn (length prefix) (prefix ++ suffix) = suffix.
Proof.
  intros A prefix.
  induction prefix as [|head tail IH]; intros suffix; simpl.
  - reflexivity.
  - exact (IH suffix).
Qed.

Lemma canonical_offset_length :
  forall snapshot,
    canonicalb snapshot = true ->
    length snapshot.(snapshot_offsets) =
      S snapshot.(snapshot_vertices).
Proof.
  intros snapshot Hcanonical.
  unfold canonicalb in Hcanonical.
  apply andb_true_iff in Hcanonical as [Hlength _].
  now apply Nat.eqb_eq in Hlength.
Qed.

Theorem decode_encode_round_trip :
  forall snapshot,
    canonicalb snapshot = true ->
    decode_words snapshot.(snapshot_profile) (encode_words snapshot) =
      Some snapshot.
Proof.
  intros [vertices offsets targets profile] Hcanonical.
  pose proof
    (canonical_offset_length
      {| snapshot_vertices := vertices;
         snapshot_offsets := offsets;
         snapshot_targets := targets;
         snapshot_profile := profile |}
      Hcanonical) as Hoffsets.
  simpl in Hoffsets.
  unfold decode_words, encode_words.
  cbn -[firstn skipn canonicalb].
  repeat rewrite Nat.eqb_refl.
  rewrite <- Hoffsets.
  rewrite firstn_exact_app, skipn_exact_app.
  rewrite Hoffsets, Nat.eqb_refl.
  rewrite Nat.eqb_refl.
  rewrite Hcanonical.
  reflexivity.
Qed.

Theorem canonical_encoding_unique :
  forall left right,
    canonicalb left = true ->
    canonicalb right = true ->
    left.(snapshot_profile) = right.(snapshot_profile) ->
    encode_words left = encode_words right ->
    left = right.
Proof.
  intros left right Hleft Hright Hprofile Hencoded.
  pose proof (f_equal (decode_words left.(snapshot_profile)) Hencoded) as Hdecoded.
  rewrite decode_encode_round_trip in Hdecoded by exact Hleft.
  rewrite Hprofile in Hdecoded.
  rewrite decode_encode_round_trip in Hdecoded by exact Hright.
  now inversion Hdecoded.
Qed.

Definition encode_with_schema
    (schema : nat) (snapshot : graph_snapshot) : list nat :=
  [magic_word; schema; schema_major; schema_minor;
   snapshot.(snapshot_profile);
   snapshot.(snapshot_vertices);
   length snapshot.(snapshot_targets)] ++
  snapshot.(snapshot_offsets) ++ snapshot.(snapshot_targets).

Theorem unknown_schema_rejected :
  forall expected_profile schema snapshot,
    schema <> schema_word ->
    decode_words expected_profile (encode_with_schema schema snapshot) = None.
Proof.
  intros expected_profile schema snapshot Hschema.
  unfold decode_words, encode_with_schema.
  destruct snapshot as [vertices offsets targets profile].
  simpl.
  apply Nat.eqb_neq in Hschema.
  rewrite Hschema.
  reflexivity.
Qed.

Definition encode_with_version
    (major minor : nat) (snapshot : graph_snapshot) : list nat :=
  [magic_word; schema_word; major; minor;
   snapshot.(snapshot_profile);
   snapshot.(snapshot_vertices);
   length snapshot.(snapshot_targets)] ++
  snapshot.(snapshot_offsets) ++ snapshot.(snapshot_targets).

Theorem unknown_major_rejected :
  forall expected_profile major minor snapshot,
    major <> schema_major ->
    decode_words expected_profile
      (encode_with_version major minor snapshot) = None.
Proof.
  intros expected_profile major minor snapshot Hmajor.
  unfold decode_words, encode_with_version.
  destruct snapshot as [vertices offsets targets profile].
  simpl.
  apply Nat.eqb_neq in Hmajor.
  rewrite Hmajor.
  reflexivity.
Qed.

Theorem unknown_minor_rejected :
  forall expected_profile minor snapshot,
    minor <> schema_minor ->
    decode_words expected_profile
      (encode_with_version schema_major minor snapshot) = None.
Proof.
  intros expected_profile minor snapshot Hminor.
  unfold decode_words, encode_with_version.
  destruct snapshot as [vertices offsets targets profile].
  simpl.
  apply Nat.eqb_neq in Hminor.
  rewrite Hminor.
  reflexivity.
Qed.

Theorem semantic_profile_mismatch_rejected :
  forall expected_profile snapshot,
    snapshot.(snapshot_profile) <> expected_profile ->
    decode_words expected_profile (encode_words snapshot) = None.
Proof.
  intros expected_profile [vertices offsets targets profile] Hprofile.
  unfold decode_words, encode_words.
  simpl in *.
  apply Nat.eqb_neq in Hprofile.
  rewrite Hprofile.
  reflexivity.
Qed.

Record decode_limits : Type := {
  maximum_vertices : nat;
  maximum_edges : nat;
  maximum_bytes : nat
}.

Definition wire_bytes (snapshot : graph_snapshot) : nat :=
  80 + 4 *
    (S snapshot.(snapshot_vertices) +
     length snapshot.(snapshot_targets)).

Definition within_limitsb
    (limits : decode_limits) (snapshot : graph_snapshot) : bool :=
  Nat.leb snapshot.(snapshot_vertices) limits.(maximum_vertices) &&
  (Nat.leb (length snapshot.(snapshot_targets)) limits.(maximum_edges) &&
   Nat.leb (wire_bytes snapshot) limits.(maximum_bytes)).

Definition decoder_heap_words (snapshot : graph_snapshot) : nat :=
  2 * snapshot.(snapshot_vertices) + 1 +
  length snapshot.(snapshot_targets).

Definition decoder_work_upper_bound (snapshot : graph_snapshot) : nat :=
  8 + 2 * S snapshot.(snapshot_vertices) +
  2 * snapshot.(snapshot_vertices) +
  3 * length snapshot.(snapshot_targets).

Theorem admitted_snapshot_respects_vertex_limit :
  forall limits snapshot,
    within_limitsb limits snapshot = true ->
    snapshot.(snapshot_vertices) <= limits.(maximum_vertices).
Proof.
  intros limits snapshot Hadmitted.
  unfold within_limitsb in Hadmitted.
  apply andb_true_iff in Hadmitted as [Hvertices _].
  now apply Nat.leb_le in Hvertices.
Qed.

Theorem admitted_snapshot_respects_edge_limit :
  forall limits snapshot,
    within_limitsb limits snapshot = true ->
    length snapshot.(snapshot_targets) <= limits.(maximum_edges).
Proof.
  intros limits snapshot Hadmitted.
  unfold within_limitsb in Hadmitted.
  apply andb_true_iff in Hadmitted as [_ Htail].
  apply andb_true_iff in Htail as [Hedges _].
  now apply Nat.leb_le in Hedges.
Qed.

Theorem admitted_snapshot_respects_byte_limit :
  forall limits snapshot,
    within_limitsb limits snapshot = true ->
    wire_bytes snapshot <= limits.(maximum_bytes).
Proof.
  intros limits snapshot Hadmitted.
  unfold within_limitsb in Hadmitted.
  apply andb_true_iff in Hadmitted as [_ Htail].
  apply andb_true_iff in Htail as [_ Hbytes].
  now apply Nat.leb_le in Hbytes.
Qed.

Theorem decoder_heap_is_linear :
  forall snapshot,
    decoder_heap_words snapshot =
      2 * snapshot.(snapshot_vertices) + 1 +
      length snapshot.(snapshot_targets).
Proof.
  reflexivity.
Qed.

Theorem decoder_work_is_linear :
  forall snapshot,
    decoder_work_upper_bound snapshot =
      10 + 4 * snapshot.(snapshot_vertices) +
      3 * length snapshot.(snapshot_targets).
Proof.
  intros snapshot.
  unfold decoder_work_upper_bound.
  lia.
Qed.

Record decoder_state : Type := {
  decoder_cursor : nat;
  decoder_length : nat;
  decoder_native_depth : nat
}.

Definition decoder_state_valid (state : decoder_state) : Prop :=
  state.(decoder_cursor) <= state.(decoder_length) /\
  state.(decoder_native_depth) = 1.

Definition decoder_step (state : decoder_state) : decoder_state :=
  if Nat.ltb state.(decoder_cursor) state.(decoder_length)
  then {|
    decoder_cursor := S state.(decoder_cursor);
    decoder_length := state.(decoder_length);
    decoder_native_depth := 1
  |}
  else state.

Theorem decoder_step_progresses :
  forall state,
    decoder_state_valid state ->
    state.(decoder_cursor) < state.(decoder_length) ->
    decoder_cursor (decoder_step state) = S state.(decoder_cursor).
Proof.
  intros state _ Hremaining.
  unfold decoder_step.
  apply Nat.ltb_lt in Hremaining.
  now rewrite Hremaining.
Qed.

Theorem decoder_step_preserves_validity :
  forall state,
    decoder_state_valid state ->
    decoder_state_valid (decoder_step state).
Proof.
  intros [cursor length depth] [Hcursor Hdepth].
  unfold decoder_step.
  simpl in *.
  destruct (Nat.ltb cursor length) eqn:Hless.
  - apply Nat.ltb_lt in Hless.
    split; simpl; lia.
  - split; simpl; assumption.
Qed.

Theorem decoder_native_stack_is_constant :
  forall state,
    decoder_state_valid state ->
    decoder_native_depth (decoder_step state) = 1.
Proof.
  intros state Hvalid.
  pose proof (decoder_step_preserves_validity Hvalid) as [_ Hdepth].
  exact Hdepth.
Qed.

Record digest_preimage : Type := {
  preimage_domain : list nat;
  preimage_schema : nat;
  preimage_profile : nat;
  preimage_payload : list nat
}.

Definition snapshot_digest_preimage
    (domain : list nat) (schema profile : nat) (payload : list nat)
    : digest_preimage :=
  {| preimage_domain := domain;
     preimage_schema := schema;
     preimage_profile := profile;
     preimage_payload := payload |}.

Theorem digest_domain_separated :
  forall left_domain right_domain schema profile payload,
    left_domain <> right_domain ->
    snapshot_digest_preimage left_domain schema profile payload <>
    snapshot_digest_preimage right_domain schema profile payload.
Proof.
  intros left_domain right_domain schema profile payload Hdomains Hequal.
  inversion Hequal.
  contradiction.
Qed.

Theorem digest_schema_separated :
  forall domain left_schema right_schema profile payload,
    left_schema <> right_schema ->
    snapshot_digest_preimage domain left_schema profile payload <>
    snapshot_digest_preimage domain right_schema profile payload.
Proof.
  intros domain left_schema right_schema profile payload Hschemas Hequal.
  inversion Hequal.
  contradiction.
Qed.

Theorem digest_profile_separated :
  forall domain schema left_profile right_profile payload,
    left_profile <> right_profile ->
    snapshot_digest_preimage domain schema left_profile payload <>
    snapshot_digest_preimage domain schema right_profile payload.
Proof.
  intros domain schema left_profile right_profile payload Hprofiles Hequal.
  inversion Hequal.
  contradiction.
Qed.

Theorem canonical_enumeration_extensional :
  forall left right,
    left = right ->
    encode_words left = encode_words right.
Proof.
  intros left right Hequal.
  now subst.
Qed.

Theorem canonical_rename_round_trip :
  forall renamed,
    canonicalb renamed = true ->
    decode_words renamed.(snapshot_profile) (encode_words renamed) =
      Some renamed.
Proof.
  exact decode_encode_round_trip.
Qed.

Print Assumptions decode_encode_round_trip.
Print Assumptions canonical_encoding_unique.
Print Assumptions unknown_schema_rejected.
Print Assumptions unknown_major_rejected.
Print Assumptions unknown_minor_rejected.
Print Assumptions semantic_profile_mismatch_rejected.
Print Assumptions admitted_snapshot_respects_vertex_limit.
Print Assumptions admitted_snapshot_respects_edge_limit.
Print Assumptions admitted_snapshot_respects_byte_limit.
Print Assumptions decoder_heap_is_linear.
Print Assumptions decoder_work_is_linear.
Print Assumptions decoder_step_progresses.
Print Assumptions decoder_step_preserves_validity.
Print Assumptions decoder_native_stack_is_constant.
Print Assumptions digest_domain_separated.
Print Assumptions digest_schema_separated.
Print Assumptions digest_profile_separated.
Print Assumptions canonical_enumeration_extensional.
Print Assumptions canonical_rename_round_trip.
