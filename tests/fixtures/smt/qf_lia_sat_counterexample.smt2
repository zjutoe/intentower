; QF_LIA obligation that fails: hypotheses AND NOT(goal) is sat, so the model
; is a candidate counterexample. The (get-value ...) below only works with
; --produce-models; without it cvc5 prints an error yet STILL exits 0.
(set-logic QF_LIA)
(set-info :source "intentower T09 fixture: not-discharged obligation")
(declare-fun a () Int)
(declare-fun b () Int)
(assert (>= a 0))
(assert (>= b 0))
(assert (not (>= (+ a b) 1)))
(check-sat)
(get-value (a b))
