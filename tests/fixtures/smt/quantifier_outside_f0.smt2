; Quantifiers are outside the F0 subset (QF_LIA = quantifier-free).
; Expected: cvc5 rejects it for this logic. Must never be treated as proved.
(set-logic QF_LIA)
(set-info :source "intentower T09 fixture: quantifier outside F0")
(declare-fun n () Int)
(assert (forall ((x Int)) (>= (+ x n) x)))
(check-sat)
