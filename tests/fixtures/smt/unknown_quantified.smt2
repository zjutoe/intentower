; Genuine UNKNOWN: an undecidable-in-practice quantified formula over Ints.
; Observed: cvc5 prints `unknown` and exits 0 -> exit code alone is not a verdict.
(set-logic ALL)
(set-info :source "intentower T09 addendum: reproducible UNKNOWN verdict")
(declare-fun f (Int) Int)
(assert (forall ((x Int)) (exists ((y Int)) (> (f y) (f x)))))
(check-sat)
