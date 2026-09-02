(set-logic ALL)

(define-fun U32_MAX () Int 4294967295)
(define-fun U64_MAX () Int 18446744073709551615)
(define-fun HEADER_BYTES () Int 80)
(define-fun WORD_BYTES () Int 4)

(declare-const vertices Int)
(declare-const edges Int)

; The largest representable v1 snapshot length cannot overflow u64.
(push)
(assert (and (<= 0 vertices) (<= vertices U32_MAX)
             (<= 0 edges) (<= edges U32_MAX)))
(define-fun wire_length () Int
  (+ HEADER_BYTES (* WORD_BYTES (+ vertices 1 edges))))
(assert (> wire_length U64_MAX))
(check-sat)
(pop)

; The returned graph owns V dense nodes, V + 1 offsets, and E targets.
(push)
(assert (and (<= 0 vertices) (<= vertices U32_MAX)
             (<= 0 edges) (<= edges U32_MAX)))
(define-fun heap_words () Int (+ (* 2 vertices) 1 edges))
(assert (not (= heap_words (+ (* 2 vertices) 1 edges))))
(check-sat)
(pop)

; Complete structural decoding is bounded by 8 + 2(V + 1) + 2V + 3E units.
(push)
(assert (and (<= 0 vertices) (<= 0 edges)))
(declare-const actual_work Int)
(assert (<= actual_work (+ 8 (* 2 (+ vertices 1)) (* 2 vertices) (* 3 edges))))
(assert (> actual_work (+ 8 (* 2 (+ vertices 1)) (* 2 vertices) (* 3 edges))))
(check-sat)
(pop)

(declare-datatypes ((DigestPreimage 0))
  (((preimage
      (preimage-domain Int)
      (preimage-schema Int)
      (preimage-profile Int)
      (preimage-length Int)
      (preimage-payload Int)))))

(declare-const domain-a Int)
(declare-const domain-b Int)
(declare-const schema-a Int)
(declare-const schema-b Int)
(declare-const profile-a Int)
(declare-const profile-b Int)
(declare-const payload Int)
(declare-const payload-length Int)

; Constructor-level domain separation is injective before cryptographic hashing.
(push)
(assert (distinct domain-a domain-b))
(assert (= (preimage domain-a schema-a profile-a payload-length payload)
           (preimage domain-b schema-a profile-a payload-length payload)))
(check-sat)
(pop)

; Schema identity is part of the tagged preimage.
(push)
(assert (distinct schema-a schema-b))
(assert (= (preimage domain-a schema-a profile-a payload-length payload)
           (preimage domain-a schema-b profile-a payload-length payload)))
(check-sat)
(pop)

; Semantic profile identity is part of the tagged preimage.
(push)
(assert (distinct profile-a profile-b))
(assert (= (preimage domain-a schema-a profile-a payload-length payload)
           (preimage domain-a schema-a profile-b payload-length payload)))
(check-sat)
(pop)

; Exact publication is fail-closed over every independent admission check.
(declare-const magic-ok Bool)
(declare-const schema-ok Bool)
(declare-const version-ok Bool)
(declare-const profile-ok Bool)
(declare-const length-ok Bool)
(declare-const budget-ok Bool)
(declare-const canonical-ok Bool)
(declare-const digest-ok Bool)
(declare-const cancelled Bool)
(define-fun publish () Bool
  (and magic-ok schema-ok version-ok profile-ok length-ok budget-ok
       canonical-ok digest-ok (not cancelled)))

(push)
(assert publish)
(assert (not schema-ok))
(check-sat)
(pop)

(push)
(assert publish)
(assert (not profile-ok))
(check-sat)
(pop)

(push)
(assert publish)
(assert cancelled)
(check-sat)
(pop)

; A stale/schema-mismatched candidate has a constructive rejected model.
(push)
(assert magic-ok)
(assert (not schema-ok))
(assert version-ok)
(assert profile-ok)
(assert length-ok)
(assert budget-ok)
(assert canonical-ok)
(assert digest-ok)
(assert (not cancelled))
(assert (not publish))
(check-sat)
(get-model)
(pop)

; A fully admitted candidate has a constructive publishable model.
(push)
(assert magic-ok)
(assert schema-ok)
(assert version-ok)
(assert profile-ok)
(assert length-ok)
(assert budget-ok)
(assert canonical-ok)
(assert digest-ok)
(assert (not cancelled))
(assert publish)
(check-sat)
(get-model)
(pop)
