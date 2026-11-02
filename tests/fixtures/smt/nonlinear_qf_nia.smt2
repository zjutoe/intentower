; Non-linear integer arithmetic: outside F0, and a common source of UNKNOWN.
; Run with --tlimit=1 to observe the timeout/unknown path.
(set-logic QF_NIA)
(set-info :source "intentower T09 fixture: non-linear arithmetic")
(declare-fun x () Int)
(declare-fun y () Int)
(declare-fun z () Int)
(assert (>= x 2))
(assert (>= y 2))
(assert (>= z 2))
(assert (= (* x y z) 999999937))
(assert (= (* x x y) (* y z z)))
(check-sat)
