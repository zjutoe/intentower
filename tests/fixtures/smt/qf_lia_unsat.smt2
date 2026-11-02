; QF_LIA proof-obligation shape: hypotheses AND NOT(goal) must be unsat.
; H: n >= 0            Goal: n + 1 > 0
(set-logic QF_LIA)
(set-info :source "intentower T09 fixture: discharged obligation")
(declare-fun n () Int)
(assert (>= n 0))
(assert (not (> (+ n 1) 0)))
(check-sat)
