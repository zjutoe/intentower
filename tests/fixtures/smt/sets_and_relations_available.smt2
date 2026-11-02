; Operator census for F1 (Event-B sets/relations), verified on cvc5 1.4.2.
; These operator names parse and solve; the bare `card`, `relation.member` and
; `relation.image` spellings do NOT (see docs/cvc5-smoke.md §6).
(set-logic ALL)
(set-info :source "intentower T09 addendum: available set/relation operators")
(declare-fun s () (Set Int))
(declare-fun t () (Set Int))
(declare-fun e () Int)
(declare-fun R () (Relation Int Int))
(assert (set.member e s))
(assert (set.subset s (set.union s t)))
(assert (= (set.card s) 3))
(assert (set.member (tuple e e) R))
(assert (set.subset R R))
(check-sat)
