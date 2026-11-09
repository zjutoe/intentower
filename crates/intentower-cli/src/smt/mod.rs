//! Event-B to SMT-LIB encoding, restricted to the F0 subset (plan §P2):
//! `QF_LIA` plus boolean `true/false/not/and/or/=>`, integer literals,
//! typed integer variables, `+`, `-` and the six comparisons. Anything
//! else must be reported, never dropped and never replaced by `true`.

pub mod arithmetic;
pub mod atoms;
pub mod encode;
pub mod logic;

use std::fmt;

use rossi::formula::{ExpressionKind, PredicateKind};

#[derive(Debug, PartialEq, Eq)]
pub enum SmtError {
    /// A node outside F0. Reported, never silently skipped.
    UnsupportedNode(String),
    /// A variable that the obligation's type environment does not declare.
    UnknownIdent(String),
    /// A declared variable whose sort cannot be used here.
    TypeMismatch {
        ident: String,
        expected: String,
        found: String,
    },
    /// A name that cannot be written as an SMT-LIB symbol at all.
    BadName(String),
}

impl fmt::Display for SmtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SmtError::UnsupportedNode(node) => write!(f, "node outside F0: {node}"),
            SmtError::UnknownIdent(ident) => write!(f, "variable `{ident}` is not declared"),
            SmtError::TypeMismatch {
                ident,
                expected,
                found,
            } => write!(
                f,
                "variable `{ident}` has sort {found}, expected {expected}"
            ),
            SmtError::BadName(name) => write!(f, "cannot render `{name}` as an SMT symbol"),
        }
    }
}

pub type SmtResult<T> = Result<T, SmtError>;

/// Name a rejected expression node without dumping its whole subtree.
pub(crate) fn describe_expression(kind: &ExpressionKind) -> String {
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

/// Name a rejected predicate node without dumping its whole subtree.
pub(crate) fn describe_predicate(kind: &PredicateKind) -> String {
    match kind {
        PredicateKind::Not(_) => "negation".to_string(),
        PredicateKind::Binary { op, .. } => format!("logical operator {op:?}"),
        PredicateKind::Associative { op, .. } => format!("n-ary logical operator {op:?}"),
        PredicateKind::Quantified { op, .. } => format!("quantifier {op:?}"),
        PredicateKind::Literal(op) => format!("literal {op:?}"),
        PredicateKind::PredicateVariable(name) => format!("predicate variable `{name}`"),
        PredicateKind::Simple(_) => "simple predicate".to_string(),
        PredicateKind::Multiple(_) => "multiple assignment".to_string(),
        PredicateKind::Application { function, .. } => format!("application of `{function}`"),
        PredicateKind::Extended { tag, .. } => format!("extension operator {tag}"),
        other => format!("{other:?}"),
    }
}

/// Put every source identifier in the same private SMT namespace. Even a
/// source name already starting with `intentower_` receives the prefix, so
/// renaming never merges it with another source name or a built-in symbol.
/// Preserve the complete source text; quote spaces, Unicode and after-value
/// apostrophes. Reject quoted-symbol delimiters, escapes and control characters.
pub fn smt_name(ident: &str) -> SmtResult<String> {
    if ident.is_empty() || ident.contains(['|', '\\']) || ident.chars().any(char::is_control) {
        return Err(SmtError::BadName(ident.to_string()));
    }
    let symbol = format!("intentower_{ident}");
    // The fixed prefix supplies a legal initial character, even for `1a`.
    if symbol
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        Ok(symbol)
    } else {
        Ok(format!("|{symbol}|"))
    }
}
