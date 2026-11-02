//! Event-B to SMT-LIB encoding, restricted to the F0 subset (plan §P2):
//! `QF_LIA` plus boolean `true/false/not/and/or/=>`, integer literals,
//! typed integer variables, `+`, `-` and the six comparisons. Anything
//! else must be reported, never dropped and never replaced by `true`.

pub mod atoms;

use std::fmt;

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

/// Render an Event-B identifier as an SMT-LIB symbol. Plain names stay bare;
/// anything else is quoted, which SMT-LIB allows. Names containing `|` or
/// control characters cannot be quoted, so they fail instead of being mangled.
pub fn smt_name(ident: &str) -> SmtResult<String> {
    let bare = !ident.is_empty()
        && !ident.chars().next().unwrap().is_ascii_digit()
        && ident
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '\'');
    if bare {
        return Ok(ident.to_string());
    }
    if ident.is_empty() || ident.contains('|') || ident.chars().any(char::is_control) {
        return Err(SmtError::BadName(ident.to_string()));
    }
    Ok(format!("|{ident}|"))
}
