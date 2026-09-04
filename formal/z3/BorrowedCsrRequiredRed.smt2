(set-logic QF_LIA)
(set-option :produce-models true)

; Each push/pop block is a satisfiable counterexample after one mandatory
; admission/publication check is removed. The verifier requires six "sat"
; results; changing any result to "unsat" means the negative control no longer
; demonstrates that its corresponding invariant is necessary.

; Header mutation: terminal offset differs from target count but is accepted.
(push)
(declare-const header_terminal Int)
(declare-const header_target_count Int)
(assert (= header_terminal 0))
(assert (= header_target_count 1))
(assert (not (= header_terminal header_target_count)))
(check-sat)
(pop)

; Offset mutation: a row decreases.
(push)
(declare-const offset_start Int)
(declare-const offset_stop Int)
(assert (= offset_start 1))
(assert (= offset_stop 0))
(assert (> offset_start offset_stop))
(check-sat)
(pop)

; Target mutation: an indexed target lies outside the dense domain.
(push)
(declare-const target_vertices Int)
(declare-const bad_target Int)
(assert (= target_vertices 1))
(assert (= bad_target 1))
(assert (>= bad_target target_vertices))
(check-sat)
(pop)

; Order mutation: a row decreases.
(push)
(declare-const order_previous Int)
(declare-const order_next Int)
(assert (= order_previous 1))
(assert (= order_next 0))
(assert (>= order_previous order_next))
(check-sat)
(pop)

; Duplicate mutation: adjacent row targets alias.
(push)
(declare-const duplicate_previous Int)
(declare-const duplicate_next Int)
(assert (= duplicate_previous 0))
(assert (= duplicate_next 0))
(assert (= duplicate_previous duplicate_next))
(check-sat)
(pop)

; Publication mutation: a result is visible before completion.
(push)
(declare-const red_published Bool)
(declare-const red_completed Bool)
(assert red_published)
(assert (not red_completed))
(check-sat)
(pop)

(exit)
