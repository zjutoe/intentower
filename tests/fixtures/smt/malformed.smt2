; Deliberately malformed SMT-LIB: unbalanced parenthesis and an unknown command.
; Expected: a parse error on stderr/stdout and a non-zero exit code.
(set-logic QF_LIA
(declare-fun n () Int)
(assert (>= n 0)
(check-sat)
(not-a-real-command n)
