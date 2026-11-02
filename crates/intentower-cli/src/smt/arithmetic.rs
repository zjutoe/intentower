//! T16: linear integer arithmetic and the six comparisons.
//!
//! Output is always fully parenthesized prefix form, so no precedence can
//! be changed by string concatenation. Anything outside linear integer
//! arithmetic (multiplication of two variables, division, modulo,
//! exponentiation, sets, relations, quantifiers) is reported, never
//! approximated.

use std::collections::BTreeMap;

use rossi::formula::tag::{AssocExprOp, BinaryExprOp, RelationalOp, UnaryExprOp};
use rossi::formula::{Expression, ExpressionKind, Predicate, PredicateKind};

use super::atoms;
use super::{SmtError, SmtResult};
use crate::po::Sort;

/// An integer-valued expression in prefix form. Every sub-expression must
/// itself be an integer expression: there is no implicit conversion.
pub fn expression(expr: &Expression, types: &BTreeMap<String, Sort>) -> SmtResult<String> {
    match expr.kind() {
        ExpressionKind::IntegerLiteral(_) | ExpressionKind::FreeIdentifier(_) => {
            atoms::integer(expr, types)
        }
        // `-x` stays linear.
        ExpressionKind::Unary {
            op: UnaryExprOp::UnMinus,
            child,
        } => Ok(format!("(- {})", expression(child, types)?)),
        // Rossi folds `a + b + c` into one n-ary node; SMT-LIB `+` is n-ary too.
        ExpressionKind::Associative {
            op: AssocExprOp::Plus,
            children,
        } => Ok(format!("(+ {})", operands(children, types)?.join(" "))),
        ExpressionKind::Binary {
            op: BinaryExprOp::Minus,
            left,
            right,
        } => Ok(format!(
            "(- {} {})",
            expression(left, types)?,
            expression(right, types)?
        )),
        // A type ascription carries no arithmetic; encode what it wraps.
        ExpressionKind::Ascription { expr, .. } => expression(expr, types),
        other => Err(unsupported(other)),
    }
}

/// `E op F` for the six comparisons only. Set membership and subset are
/// outside F0 and are reported.
pub fn comparison(pred: &Predicate, types: &BTreeMap<String, Sort>) -> SmtResult<String> {
    match pred.kind() {
        PredicateKind::Relational { op, left, right } => {
            let symbol = match op {
                RelationalOp::Equal => "=",
                RelationalOp::NotEqual => "distinct",
                RelationalOp::Lt => "<",
                RelationalOp::Le => "<=",
                RelationalOp::Gt => ">",
                RelationalOp::Ge => ">=",
                other => return Err(SmtError::UnsupportedNode(format!("comparison {other:?}"))),
            };
            Ok(format!(
                "({symbol} {} {})",
                expression(left, types)?,
                expression(right, types)?
            ))
        }
        other => Err(SmtError::UnsupportedNode(format!(
            "not a relational predicate: {}",
            predicate_kind_name(other)
        ))),
    }
}

fn operands(children: &[Expression], types: &BTreeMap<String, Sort>) -> SmtResult<Vec<String>> {
    children
        .iter()
        .map(|child| expression(child, types))
        .collect()
}

/// Name the rejected node without dumping its whole subtree.
fn unsupported(kind: &ExpressionKind) -> SmtError {
    SmtError::UnsupportedNode(kind_name(kind))
}

fn kind_name(kind: &ExpressionKind) -> String {
    match kind {
        ExpressionKind::Binary { op, .. } => format!("binary operator {op:?}"),
        ExpressionKind::Associative { op, .. } => format!("n-ary operator {op:?}"),
        ExpressionKind::Unary { op, .. } => format!("unary operator {op:?}"),
        ExpressionKind::Atomic(op) => format!("atom {op:?}"),
        ExpressionKind::SetExtension(_) => "set in extension".to_string(),
        ExpressionKind::Bool(_) => "bool(P)".to_string(),
        ExpressionKind::Quantified { .. } => "quantified expression".to_string(),
        ExpressionKind::Extended { tag, .. } => format!("extension operator {tag}"),
        ExpressionKind::BoundIdentifier(index) => format!("bound variable {index}"),
        other => format!("{other:?}"),
    }
}

fn predicate_kind_name(kind: &PredicateKind) -> String {
    match kind {
        PredicateKind::Not(_) => "negation".to_string(),
        PredicateKind::Binary { op, .. } => format!("logical operator {op:?}"),
        PredicateKind::Associative { op, .. } => format!("n-ary logical operator {op:?}"),
        PredicateKind::Quantified { op, .. } => format!("quantifier {op:?}"),
        PredicateKind::Literal(op) => format!("literal {op:?}"),
        PredicateKind::PredicateVariable(name) => format!("predicate variable `{name}`"),
        PredicateKind::Simple(_) => "simple predicate".to_string(),
        PredicateKind::Multiple(_) => "multiple assignment".to_string(),
        PredicateKind::Application { function, .. } => {
            format!("application of `{function}`")
        }
        PredicateKind::Extended { tag, .. } => format!("extension operator {tag}"),
        other => format!("{other:?}"),
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

    fn ints() -> BTreeMap<String, Sort> {
        types(&[
            ("a", Sort::Int),
            ("b", Sort::Int),
            ("c", Sort::Int),
            ("delta", Sort::Int),
            ("count", Sort::Int),
            ("abstract_count", Sort::Int),
            ("S", Sort::Given("STATUS".to_string())),
        ])
    }

    fn expr(src: &str) -> Expression {
        parse_expression_str(src).unwrap_or_else(|err| panic!("parse `{src}`: {err}"))
    }

    fn pred(src: &str) -> Predicate {
        parse_predicate_str(src).unwrap_or_else(|err| panic!("parse `{src}`: {err}"))
    }

    #[test]
    fn arithmetic_encodes_in_prefix_form() {
        let t = ints();
        let cases = [
            ("a + b", "(+ a b)"),
            ("a + b + c", "(+ a b c)"),
            ("a − b", "(- a b)"),
            ("count - 1", "(- count 1)"),
            ("−a", "(- a)"),
            ("a + b − c", "(- (+ a b) c)"),
            ("(a + b) − (c − a)", "(- (+ a b) (- c a))"),
            ("abstract_count + delta", "(+ abstract_count delta)"),
            ("0", "0"),
            ("-42", "(- 42)"),
        ];
        for (source, want) in cases {
            assert_eq!(expression(&expr(source), &t).unwrap(), want, "`{source}`");
        }
    }

    #[test]
    fn comparisons_encode_with_both_operands() {
        let t = ints();
        let cases = [
            ("a ≥ 0", "(>= a 0)"),
            ("a + delta ≤ 1000", "(<= (+ a delta) 1000)"),
            ("abstract_count = a + b", "(= abstract_count (+ a b))"),
            ("delta > 0", "(> delta 0)"),
            ("a < b", "(< a b)"),
            ("a ≠ b", "(distinct a b)"),
            ("a − b ≥ c", "(>= (- a b) c)"),
        ];
        for (source, want) in cases {
            assert_eq!(comparison(&pred(source), &t).unwrap(), want, "`{source}`");
        }
    }

    #[test]
    fn multiplication_division_modulo_exponent_are_rejected() {
        let t = ints();
        for source in ["a ∗ b", "a / b", "a mod b", "a ^ b"] {
            let err = expression(&expr(source), &t).unwrap_err();
            assert!(
                matches!(err, SmtError::UnsupportedNode(_)),
                "`{source}` must be rejected, got {err}"
            );
        }
    }

    #[test]
    fn set_and_relation_nodes_are_rejected() {
        let t = ints();
        assert!(matches!(
            expression(&expr("S ∪ {a}"), &t),
            Err(SmtError::UnsupportedNode(_))
        ));
        assert!(matches!(
            expression(&expr("card(S)"), &t),
            Err(SmtError::UnsupportedNode(_))
        ));
        assert!(matches!(
            comparison(&pred("S ⊆ {a}"), &t),
            Err(SmtError::UnsupportedNode(_))
        ));
        assert!(matches!(
            comparison(&pred("a ∈ S"), &t),
            Err(SmtError::UnsupportedNode(_))
        ));
    }

    #[test]
    fn a_non_integer_operand_is_a_type_error_not_a_guess() {
        let t = ints();
        assert_eq!(
            expression(&expr("a + S"), &t),
            Err(SmtError::TypeMismatch {
                ident: "S".to_string(),
                expected: "Int".to_string(),
                found: "Given(\"STATUS\")".to_string(),
            })
        );
        assert_eq!(
            comparison(&pred("S = a"), &t),
            Err(SmtError::TypeMismatch {
                ident: "S".to_string(),
                expected: "Int".to_string(),
                found: "Given(\"STATUS\")".to_string(),
            })
        );
    }

    #[test]
    fn quantifiers_and_boolean_wrappers_are_rejected() {
        let t = ints();
        assert!(matches!(
            expression(&expr("{n | n ≥ 0}"), &t),
            Err(SmtError::UnsupportedNode(_))
        ));
        assert_eq!(
            comparison(&pred("∃ n · n ≥ 0"), &t),
            Err(SmtError::UnsupportedNode(
                "not a relational predicate: quantifier Exists".to_string()
            ))
        );
        assert!(matches!(
            comparison(&pred("¬ (a = b)"), &t),
            Err(SmtError::UnsupportedNode(_))
        ));
        assert!(matches!(
            comparison(&pred("a = b ∧ b = c"), &t),
            Err(SmtError::UnsupportedNode(_))
        ));
    }

    #[test]
    fn rejection_messages_name_the_operator_not_the_subtree() {
        let t = ints();
        let err = expression(&expr("a ∗ b ∗ c"), &t).unwrap_err();
        assert_eq!(
            err,
            SmtError::UnsupportedNode("n-ary operator Mul".to_string())
        );
    }
}
