; Probes what cvc5 prints for an option/theory it does not support.
; Observed: cvc5 prints the bare word `unsupported` for the option, then a
; parse error for `card`, and exits 1.
(set-logic ALL)
(set-option :sets-ext true)
(declare-fun s () (Set Int))
(assert (set.member 1 s))
(assert (> (card s) 3))
(check-sat)
