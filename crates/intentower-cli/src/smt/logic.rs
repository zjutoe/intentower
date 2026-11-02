//! T17: boolean connectives — `not`, `and`, `or`, `=>`, and equality via
//! T16's comparisons.
//!
//! Output is fully parenthesized prefix form, so the rendered string has no
//! precedence to reinterpret. Rossi's dialect already forbids mixing `∧` with
//! `∨` (and `⇒` with `⇔`) without parentheses, so the input tree is
//! unambiguous too. Quantifiers and set operators are reported: dropping a
//! hypothesis would make the obligation easier to prove, i.e. a false PASS.

use std::collections::BTreeMap;

use rossi::formula::tag::{AssocPredOp, BinaryPredOp};
use rossi::formula::{Predicate, PredicateKind};

use super::arithmetic;
use super::atoms;
use super::{SmtError, SmtResult, describe_predicate};
use crate::po::Sort;

pub fn predicate(pred: &Predicate, types: &BTreeMap<String, Sort>) -> SmtResult<String> {
    match pred.kind() {
        PredicateKind::Literal(_) => atoms::boolean(pred),
        PredicateKind::Relational { .. } => arithmetic::comparison(pred, types),
        PredicateKind::Not(inner) => Ok(format!("(not {})", predicate(inner, types)?)),
        PredicateKind::Binary { op, left, right } => {
            let symbol = match op {
                BinaryPredOp::LImp => "=>",
                // `⇔` is boolean equality, which SMT-LIB writes as `=`.
                BinaryPredOp::LEqv => "=",
            };
            Ok(format!(
                "({symbol} {} {})",
                predicate(left, types)?,
                predicate(right, types)?
            ))
        }
        // Rossi folds a chain of the same connective into one n-ary node.
        PredicateKind::Associative { op, children } => {
            let symbol = match op {
                AssocPredOp::LAnd => "and",
                AssocPredOp::LOr => "or",
            };
            let parts = children
                .iter()
                .map(|child| predicate(child, types))
                .collect::<SmtResult<Vec<String>>>()?;
            Ok(format!("({symbol} {})", parts.join(" ")))
        }
        other => Err(SmtError::UnsupportedNode(describe_predicate(other))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rossi::parser::parse_predicate_str;

    fn types() -> BTreeMap<String, Sort> {
        [
            ("a", Sort::Int),
            ("b", Sort::Int),
            ("c", Sort::Int),
            ("delta", Sort::Int),
            ("abstract_count", Sort::Int),
            ("S", Sort::Given("STATUS".to_string())),
        ]
        .iter()
        .map(|(name, sort)| (name.to_string(), sort.clone()))
        .collect()
    }

    fn encode(source: &str) -> String {
        predicate(
            &parse_predicate_str(source).unwrap_or_else(|err| panic!("`{source}`: {err}")),
            &types(),
        )
        .unwrap()
    }

    fn reject(source: &str) -> SmtError {
        let parsed = parse_predicate_str(source).unwrap_or_else(|err| panic!("`{source}`: {err}"));
        predicate(&parsed, &types()).unwrap_err()
    }

    #[test]
    fn connectives_encode_in_prefix_form() {
        assert_eq!(encode("a = b ∧ b = c"), "(and (= a b) (= b c))");
        assert_eq!(
            encode("a ≥ 0 ∧ a ≤ 1000 ∧ delta ≥ 1"),
            "(and (>= a 0) (<= a 1000) (>= delta 1))",
            "a chain of the same connective stays n-ary"
        );
        assert_eq!(encode("a ≥ 0 ∨ a ≤ 100"), "(or (>= a 0) (<= a 100))");
        assert_eq!(encode("¬ (a = b)"), "(not (= a b))");
        assert_eq!(
            encode("delta ≥ 1 ⇒ abstract_count ≥ 1"),
            "(=> (>= delta 1) (>= abstract_count 1))"
        );
        assert_eq!(encode("a = b ⇔ b = a"), "(= (= a b) (= b a))");
        assert_eq!(encode("⊤"), "true");
        assert_eq!(encode("⊥ ∧ a = b"), "(and false (= a b))");
    }

    #[test]
    fn nesting_keeps_every_group_explicit() {
        assert_eq!(
            encode("a = b ⇒ (b = c ∧ ¬ (a = c))"),
            "(=> (= a b) (and (= b c) (not (= a c))))"
        );
        assert_eq!(
            encode("(a ≥ 0 ∨ a ≤ 100) ∧ a ≠ 0"),
            "(and (or (>= a 0) (<= a 100)) (distinct a 0))"
        );
        assert_eq!(
            encode("¬ (a ≥ 0 ∨ a ≤ 100)"),
            "(not (or (>= a 0) (<= a 100)))"
        );
        assert_eq!(
            encode("(a = b ⇒ b = c) ⇒ a = c"),
            "(=> (=> (= a b) (= b c)) (= a c))"
        );
    }

    #[test]
    fn the_dialect_refuses_ambiguous_mixing_without_parentheses() {
        // Rossi rejects these at parse time, so no encoder guess is possible.
        for source in ["a ≥ 0 ∨ a ≤ 100 ∧ a ≠ 0", "a = b ⇒ b = c ⇔ c = a"] {
            assert!(
                parse_predicate_str(source).is_err(),
                "`{source}` must not parse"
            );
        }
    }

    #[test]
    fn quantifiers_are_reported_not_instantiated() {
        assert_eq!(
            reject("∃ n · n ≥ 0"),
            SmtError::UnsupportedNode("quantifier Exists".to_string())
        );
        assert_eq!(
            reject("∀ n · n ≥ 0"),
            SmtError::UnsupportedNode("quantifier Forall".to_string())
        );
    }

    #[test]
    fn set_operators_and_uninterpreted_nodes_are_reported() {
        for source in ["S ⊆ {a}", "a ∈ S", "S ∪ {a} = S"] {
            assert!(
                matches!(reject(source), SmtError::UnsupportedNode(_)),
                "`{source}` must be rejected"
            );
        }
        assert_eq!(
            reject("f(a) = b"),
            SmtError::UnsupportedNode("binary operator FunImage".to_string())
        );
    }

    #[test]
    fn a_rejected_leaf_inside_a_conjunction_rejects_the_whole_conjunction() {
        // Partial encoding would silently drop a hypothesis.
        assert!(matches!(
            reject("a ≥ 0 ∧ (∃ n · n ≥ 0)"),
            SmtError::UnsupportedNode(_)
        ));
    }
}
