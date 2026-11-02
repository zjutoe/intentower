; Event-B sets/relations are outside F0. Expected: rejected for QF_LIA.
(set-logic QF_LIA)
(set-info :source "intentower T09 fixture: set theory outside F0")
(declare-fun s () (Set Int))
(declare-fun e () Int)
(assert (and (not (= s (as emptyset (Set Int)))) (member e s)))
(check-sat)
