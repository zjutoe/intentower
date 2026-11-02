; Probes what cvc5 prints for an option it does not support.
; Observed: cvc5 prints the bare word `unsupported` (that option alone exits 0),
; then a parse error for the bare `card` -- the namespaced `set.card` works --
; and the file as a whole exits 1 because of that parse error.
(set-logic ALL)
(set-option :sets-ext true)
(declare-fun s () (Set Int))
(assert (set.member 1 s))
(assert (> (card s) 3))
(check-sat)
