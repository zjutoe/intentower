//! T18: a complete SMT-LIB script for one proof obligation.
//!
//! Shape: `set-logic`, one declaration per free variable, then
//! `H1 ∧ … ∧ Hn ∧ ¬Goal`, then `(check-sat)`. Unsatisfiable means the
//! obligation holds under the encoded hypotheses; anything else is decided
//! by the solver in T19, never here.

use std::collections::{BTreeMap, BTreeSet};

use rossi::formula::{Expression, ExpressionKind, Predicate, PredicateKind};

use super::logic;
use super::{SmtError, SmtResult, describe_predicate, smt_name};
use crate::po::{Po, Sort};

/// Encode one obligation into a standalone SMT-LIB script.
pub fn encode(po: &Po) -> SmtResult<String> {
    let mut assertions = Vec::new();
    for hypothesis in &po.typed_hypotheses {
        assertions.push(logic::predicate(hypothesis, &po.types)?);
    }
    let goal = logic::predicate(&po.typed_goal, &po.types)?;

    let mut names: BTreeSet<String> = BTreeSet::new();
    for hypothesis in &po.typed_hypotheses {
        free_idents(hypothesis, &mut names)?;
    }
    free_idents(&po.typed_goal, &mut names)?;

    let mut lines = vec![
        "(set-logic QF_LIA)".to_string(),
        format!("(set-info :source \"{}::{}\")", po.machine, po.id),
    ];
    for name in &names {
        lines.push(format!(
            "(declare-fun {} () {})",
            smt_name(name)?,
            declaration_sort(&po.types, name)?
        ));
    }

    assertions.push(format!("(not {goal})"));
    let body = if assertions.len() == 1 {
        assertions.pop().unwrap()
    } else {
        format!("(and {})", assertions.join(" "))
    };
    lines.push(format!("(assert {body})"));
    lines.push("(check-sat)".to_string());
    Ok(lines.join("\n") + "\n")
}

fn declaration_sort(types: &BTreeMap<String, Sort>, name: &str) -> SmtResult<&'static str> {
    match types.get(name) {
        None => Err(SmtError::UnknownIdent(name.to_string())),
        Some(Sort::Int) => Ok("Int"),
        Some(Sort::Bool) => Ok("Bool"),
        Some(sort) => Err(SmtError::TypeMismatch {
            ident: name.to_string(),
            expected: "Int or Bool".to_string(),
            found: format!("{sort:?}"),
        }),
    }
}

/// Free identifiers of an already-encoded (F0) predicate. Bound variables
/// cannot appear here: quantifiers are rejected by `logic::predicate`.
fn free_idents(pred: &Predicate, out: &mut BTreeSet<String>) -> SmtResult<()> {
    match pred.kind() {
        PredicateKind::Literal(_) => Ok(()),
        PredicateKind::Relational { left, right, .. } => {
            expression_idents(left, out);
            expression_idents(right, out);
            Ok(())
        }
        PredicateKind::Binary { left, right, .. } => {
            free_idents(left, out)?;
            free_idents(right, out)
        }
        PredicateKind::Associative { children, .. } => children
            .iter()
            .try_for_each(|child| free_idents(child, out)),
        PredicateKind::Not(inner) => free_idents(inner, out),
        other => Err(SmtError::UnsupportedNode(describe_predicate(other))),
    }
}

fn expression_idents(expr: &Expression, out: &mut BTreeSet<String>) {
    match expr.kind() {
        ExpressionKind::FreeIdentifier(name) => {
            out.insert(name.clone());
        }
        ExpressionKind::IntegerLiteral(_) | ExpressionKind::BoundIdentifier(_) => {}
        ExpressionKind::Unary { child, .. } => expression_idents(child, out),
        ExpressionKind::Ascription { expr, .. } => expression_idents(expr, out),
        ExpressionKind::Associative { children, .. } | ExpressionKind::SetExtension(children) => {
            children
                .iter()
                .for_each(|child| expression_idents(child, out))
        }
        ExpressionKind::Binary { left, right, .. } => {
            expression_idents(left, out);
            expression_idents(right, out);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::po::{Kind, Source};
    use rossi::parser::parse_predicate_str;

    fn po(hypotheses: &[&str], goal: &str, sorts: &[(&str, &str)]) -> Po {
        Po {
            id: "evt/inv1/INV".to_string(),
            kind: Kind::Inv,
            machine: "m".to_string(),
            typed_hypotheses: hypotheses
                .iter()
                .map(|h| parse_predicate_str(h).unwrap())
                .collect(),
            typed_goal: parse_predicate_str(goal).unwrap(),
            types: sorts
                .iter()
                .map(|(name, sort)| {
                    (
                        name.to_string(),
                        match *sort {
                            "Int" => Sort::Int,
                            "Bool" => Sort::Bool,
                            other => Sort::Given(other.to_string()),
                        },
                    )
                })
                .collect(),
            source: Source {
                bpo: "m.bpo".to_string(),
                stamp: None,
            },
        }
    }

    #[test]
    fn no_hypotheses_asserts_only_the_negated_goal() {
        let script = encode(&po(&[], "a ≥ 0", &[("a", "Int")])).unwrap();
        assert_eq!(
            script,
            "(set-logic QF_LIA)\n\
             (set-info :source \"m::evt/inv1/INV\")\n\
             (declare-fun a () Int)\n\
             (assert (not (>= a 0)))\n\
             (check-sat)\n"
        );
    }

    #[test]
    fn several_hypotheses_are_conjoined_before_the_negated_goal() {
        let script = encode(&po(
            &["a ≥ 0", "b ≥ 1", "a = b"],
            "a + b ≥ 1",
            &[("a", "Int"), ("b", "Int")],
        ))
        .unwrap();
        assert!(
            script.contains("(assert (and (>= a 0) (>= b 1) (= a b) (not (>= (+ a b) 1))))"),
            "{script}"
        );
        assert!(script.starts_with("(set-logic QF_LIA)\n"));
        assert!(script.ends_with("(check-sat)\n"));
    }

    #[test]
    fn every_free_variable_is_declared_exactly_once() {
        let script = encode(&po(
            &["a ≥ 0 ∧ b ≥ 0", "a + b ≥ c", "c ≥ a"],
            "a ≠ b",
            &[("a", "Int"), ("b", "Int"), ("c", "Int")],
        ))
        .unwrap();
        for name in ["a", "b", "c"] {
            let needle = format!("(declare-fun {name} () Int)");
            assert_eq!(
                script.matches(&needle).count(),
                1,
                "variable `{name}` declared more than once:\n{script}"
            );
        }
        assert_eq!(script.matches("(declare-fun").count(), 3, "{script}");
    }

    #[test]
    fn a_non_integer_free_variable_is_a_type_conflict() {
        let sorts = ["a", "S"];
        let types: BTreeMap<String, Sort> =
            [("a", Sort::Int), ("S", Sort::Given("STATUS".to_string()))]
                .iter()
                .map(|(name, sort)| (name.to_string(), sort.clone()))
                .collect();
        assert_eq!(declaration_sort(&types, sorts[0]).unwrap(), "Int");
        assert_eq!(
            declaration_sort(&types, "missing"),
            Err(SmtError::UnknownIdent("missing".to_string()))
        );
        assert_eq!(
            declaration_sort(&types, "S"),
            Err(SmtError::TypeMismatch {
                ident: "S".to_string(),
                expected: "Int or Bool".to_string(),
                found: "Given(\"STATUS\")".to_string(),
            })
        );
    }

    #[test]
    fn distinct_idents_never_share_one_smt_symbol() {
        // smt_name is injective over names it accepts: bare names never start
        // with `|`, quoted names never contain one. So no runtime collision
        // check is needed in encode(); this test pins that property.
        let names = [
            "a",
            "b",
            "a b",
            "a'",
            "count'",
            "1a",
            "abstract_count",
            "a.b",
            "a-b",
        ];
        let symbols: BTreeSet<String> = names.iter().map(|n| smt_name(n).unwrap()).collect();
        assert_eq!(symbols.len(), names.len(), "smt_name must be injective");
        assert!(smt_name("a|b").is_err() && smt_name("").is_err());
    }

    #[test]
    fn an_unsupported_node_produces_no_script() {
        for goal in ["∃ n · n ≥ 0", "S ⊆ {a}", "a ∈ S"] {
            let err = encode(&po(
                &["a ≥ 0"],
                goal,
                &[("a", "Int"), ("n", "Int"), ("S", "Set")],
            ));
            assert!(
                matches!(err, Err(SmtError::UnsupportedNode(_))),
                "`{goal}` must yield no script, got {err:?}"
            );
        }
    }

    #[test]
    fn encoding_is_deterministic() {
        let obligation = po(
            &["a ≥ 0", "b ≥ 1"],
            "a + b ≥ 1",
            &[("a", "Int"), ("b", "Int")],
        );
        assert_eq!(encode(&obligation).unwrap(), encode(&obligation).unwrap());
    }

    #[test]
    fn nesting_is_fully_parenthesized() {
        let script = encode(&po(
            &["(a ≥ 0 ∨ b ≤ 0) ∧ a ≠ b"],
            "¬ (a = b ⇒ c ≥ 0)",
            &[("a", "Int"), ("b", "Int"), ("c", "Int")],
        ))
        .unwrap();
        assert!(
            script.contains(
                "(assert (and (and (or (>= a 0) (<= b 0)) (distinct a b)) \
             (not (not (=> (= a b) (>= c 0))))))"
            ),
            "{script}"
        );
    }

    #[test]
    fn a_real_rossi_obligation_encodes_to_a_full_script() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../artifacts/spike/counter-m0m1-rodin");
        let obligations = crate::po::load_project(&dir).unwrap();
        let obligation = obligations
            .iter()
            .find(|po| po.machine == "m1" && po.id == "sub/grd1/GRD")
            .unwrap();
        assert_eq!(
            encode(obligation).unwrap(),
            "(set-logic QF_LIA)\n\
             (set-info :source \"m1::sub/grd1/GRD\")\n\
             (declare-fun a () Int)\n\
             (declare-fun abstract_count () Int)\n\
             (declare-fun b () Int)\n\
             (declare-fun delta () Int)\n\
             (assert (and (>= abstract_count 0) (<= abstract_count 1000) (>= a 0) (>= b 0) \
             (= abstract_count (+ a b)) (>= delta 1) (<= delta a) \
             (not (>= abstract_count 1))))\n\
             (check-sat)\n"
        );
    }

    #[test]
    fn primed_variables_are_declared_and_quoted() {
        // `'` is illegal in an SMT-LIB simple symbol, so every occurrence of an
        // Event-B after-value must be pipe-quoted, declaration and use alike.
        let script = encode(&po(
            &[],
            "count' ≥ count",
            &[("count", "Int"), ("count'", "Int")],
        ))
        .unwrap();
        assert!(
            script.contains("(declare-fun |count'| () Int)")
                && script.contains("(>= |count'| count)"),
            "{script}"
        );
    }
}
