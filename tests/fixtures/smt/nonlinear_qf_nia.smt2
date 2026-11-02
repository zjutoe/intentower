; Non-linear integer arithmetic: outside F0, and a real source of long runs.
; This file does not answer within 180s. Observe the timeout path with an
; EXTERNAL timeout, e.g. `timeout 2 cvc5 nonlinear_qf_nia.smt2` -> exit 124,
; stderr `cvc5 interrupted by SIGTERM.`. Do NOT use cvc5's own --tlimit: it
; aborts (SIGABRT, exit 134) instead of printing a verdict.
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
