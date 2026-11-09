//! T15: boolean constants and integer literals / integer variables.
//!
//! The sort of a variable is never guessed: it comes from the obligation's
//! type environment (`Po::types`). A variable that is not declared, or is
//! declared with a non-integer sort, is an error — not `0`, not `true`.

use std::collections::BTreeMap;

use rossi::formula::tag::{LiteralPredOp, UnaryExprOp};
use rossi::formula::{Expression, ExpressionKind, Predicate, PredicateKind};

use super::{SmtError, SmtResult, smt_name};
use crate::po::Sort;

/// `⊤` / `⊥` as SMT booleans. Any other predicate is not an atom.
pub fn boolean(pred: &Predicate) -> SmtResult<String> {
    match pred.kind() {
        PredicateKind::Literal(LiteralPredOp::BTrue) => Ok("true".to_string()),
        PredicateKind::Literal(LiteralPredOp::BFalse) => Ok("false".to_string()),
        other => Err(SmtError::UnsupportedNode(format!("{other:?}"))),
    }
}

/// An integer literal, or a variable known to be an integer.
pub fn integer(expr: &Expression, types: &BTreeMap<String, Sort>) -> SmtResult<String> {
    match expr.kind() {
        ExpressionKind::IntegerLiteral(value) => Ok(value.to_string()),
        // Rossi parses `-42` as unary minus over a literal, never as a
        // negative literal. Fold it: SMT-LIB writes negatives as `(- n)`.
        // `-x` over a variable is arithmetic, i.e. T16, and stays reported.
        ExpressionKind::Unary {
            op: UnaryExprOp::UnMinus,
            child,
        } => match child.kind() {
            ExpressionKind::IntegerLiteral(value) => {
                let text = (-value).to_string();
                match text.strip_prefix('-') {
                    Some(magnitude) => Ok(format!("(- {magnitude})")),
                    None => Ok(text),
                }
            }
            other => Err(SmtError::UnsupportedNode(format!("{other:?}"))),
        },
        ExpressionKind::FreeIdentifier(name) => match types.get(name) {
            None => Err(SmtError::UnknownIdent(name.clone())),
            Some(Sort::Int) => smt_name(name),
            Some(sort) => Err(SmtError::TypeMismatch {
                ident: name.clone(),
                expected: "Int".to_string(),
                found: format!("{sort:?}"),
            }),
        },
        other => Err(SmtError::UnsupportedNode(format!("{other:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rossi::parser::{parse_expression_str, parse_predicate_str};

    fn types(pairs: &[(&str, Sort)]) -> BTreeMap<String, Sort> {
        pairs
            .iter()
            .map(|(name, sort)| (name.to_string(), sort.clone()))
            .collect()
    }

    fn expr(src: &str) -> Expression {
        parse_expression_str(src).unwrap_or_else(|err| panic!("parse `{src}`: {err}"))
    }

    fn pred(src: &str) -> Predicate {
        parse_predicate_str(src).unwrap_or_else(|err| panic!("parse `{src}`: {err}"))
    }

    #[test]
    fn integer_literals_survive_exactly() {
        let empty = types(&[]);
        for (source, want) in [
            ("0", "0"),
            ("-42", "(- 42)"),
            ("-0", "0"),
            ("7", "7"),
            ("1000", "1000"),
        ] {
            assert_eq!(integer(&expr(source), &empty).unwrap(), want, "`{source}`");
        }
    }

    #[test]
    fn two_different_variables_stay_different() {
        let t = types(&[("a", Sort::Int), ("count'", Sort::Int)]);
        assert_eq!(integer(&expr("a"), &t).unwrap(), "intentower_a");
        assert_eq!(integer(&expr("count'"), &t).unwrap(), "|intentower_count'|");
    }

    #[test]
    fn a_non_integer_variable_is_a_type_error() {
        let t = types(&[(
            "STATUS",
            Sort::Pow(Box::new(Sort::Given("STATUS".to_string()))),
        )]);
        assert_eq!(
            integer(&expr("STATUS"), &t),
            Err(SmtError::TypeMismatch {
                ident: "STATUS".to_string(),
                expected: "Int".to_string(),
                found: "Pow(Given(\"STATUS\"))".to_string(),
            })
        );
    }

    #[test]
    fn an_undeclared_variable_fails() {
        assert_eq!(
            integer(&expr("delta"), &types(&[("a", Sort::Int)])),
            Err(SmtError::UnknownIdent("delta".to_string()))
        );
    }

    #[test]
    fn boolean_constants_map_to_smt_booleans() {
        assert_eq!(boolean(&pred("⊤")).unwrap(), "true");
        assert_eq!(boolean(&pred("⊥")).unwrap(), "false");
    }

    #[test]
    fn a_non_atom_predicate_is_reported() {
        assert!(matches!(
            boolean(&pred("a = b")),
            Err(SmtError::UnsupportedNode(_))
        ));
    }

    #[test]
    fn sets_and_arithmetic_are_not_guessed() {
        for source in ["ℙ(STATUS)", "a + b", "a − b", "card(S)", "ℤ"] {
            assert!(
                matches!(
                    integer(&expr(source), &types(&[("a", Sort::Int)])),
                    Err(SmtError::UnsupportedNode(_))
                ),
                "`{source}` must be reported, not encoded"
            );
        }
    }

    #[test]
    fn names_become_valid_smt_symbols() {
        assert_eq!(smt_name("count'").unwrap(), "|intentower_count'|");
        assert_eq!(
            smt_name("abstract_count").unwrap(),
            "intentower_abstract_count"
        );
        assert_eq!(smt_name("a b").unwrap(), "|intentower_a b|");
        assert_eq!(smt_name("true").unwrap(), "intentower_true");
        assert_eq!(
            smt_name("intentower_true").unwrap(),
            "intentower_intentower_true"
        );
        assert_eq!(smt_name("a|b"), Err(SmtError::BadName("a|b".to_string())));
        assert_eq!(smt_name(r"a\b"), Err(SmtError::BadName(r"a\b".to_string())));
        assert_eq!(smt_name("a\nb"), Err(SmtError::BadName("a\nb".to_string())));
        assert_eq!(smt_name(""), Err(SmtError::BadName(String::new())));
    }
}
