; Quantifiers are outside the F0 subset (QF_LIA = quantifier-free).
; OBSERVED on cvc5 1.4.2: cvc5 does NOT reject this -- it answers `sat` with
; exit 0, even with --force-logic=QF_LIA. There is no solver-side guard, so
; Intentower's encoder must classify such a construct as UNSUPPORTED before
; calling cvc5. This fixture must never be counted as a proof.
(set-logic QF_LIA)
(set-info :source "intentower T09 fixture: quantifier outside F0")
(declare-fun n () Int)
(assert (forall ((x Int)) (>= (+ x n) x)))
(check-sat)
