//! Minimal `project.json` validation (T06). Only top-level shape is checked;
//! no global link graph yet.

use serde_json::Value;
use std::path::Path;

const SCHEMA_VERSION: &str = "schema_version";
const INTENT_IDS: &str = "intent_ids";
const SPEC_IDS: &str = "spec_ids";
const MODELS: &str = "models";
const REQUIRED_EVIDENCE: &str = "required_evidence";

/// The only top-level fields a Phase 1 manifest must carry.
const FIELDS: [&str; 5] = [
    SCHEMA_VERSION,
    INTENT_IDS,
    SPEC_IDS,
    MODELS,
    REQUIRED_EVIDENCE,
];

fn expected_type(field: &str) -> &'static str {
    if field == SCHEMA_VERSION {
        "a string"
    } else {
        "an array"
    }
}

/// Returns Ok only if every required top-level field exists with the right type.
/// All problems are reported at once; an empty error list means accepted.
pub fn validate(raw: &str) -> Result<(), Vec<String>> {
    let doc: Value = match serde_json::from_str(raw) {
        Ok(doc) => doc,
        Err(err) => return Err(vec![format!("invalid JSON: {err}")]),
    };
    let Some(obj) = doc.as_object() else {
        return Err(vec!["top level must be a JSON object".to_string()]);
    };
    let mut errors = Vec::new();
    for field in FIELDS {
        match obj.get(field) {
            None => errors.push(format!("missing top-level field: {field}")),
            Some(value) => {
                let ok = match field {
                    SCHEMA_VERSION => value.is_string(),
                    _ => value.is_array(),
                };
                if !ok {
                    errors.push(format!("{field} must be {}", expected_type(field)));
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn load(path: &Path) -> Result<(), Vec<String>> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| vec![format!("cannot read {}: {err}", path.display())])?;
    validate(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> Value {
        serde_json::from_str(
            r#"{"schema_version":"0.1","intent_ids":["INT-001"],"spec_ids":["SPEC-001"],
                "models":[],"required_evidence":["tool_versions"]}"#,
        )
        .unwrap()
    }

    #[test]
    fn accepts_the_repositories_manifest() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../project.json");
        load(&path).expect("root project.json must be valid");
    }

    #[test]
    fn rejects_each_missing_top_level_field_by_name() {
        for field in FIELDS {
            let mut doc = valid();
            doc.as_object_mut().unwrap().remove(field);
            let errors = validate(&doc.to_string()).unwrap_err();
            assert_eq!(
                errors,
                vec![format!("missing top-level field: {field}")],
                "{field}"
            );
        }
    }

    #[test]
    fn rejects_wrong_types() {
        let mut doc = valid();
        doc["schema_version"] = serde_json::json!(1);
        assert!(
            validate(&doc.to_string()).unwrap_err()[0]
                .starts_with("schema_version must be a string")
        );

        let mut doc = valid();
        doc["models"] = serde_json::json!("models/counter");
        assert!(validate(&doc.to_string()).unwrap_err()[0].starts_with("models must be an array"));
    }

    #[test]
    fn reports_every_missing_field_at_once() {
        let errors = validate("{}").unwrap_err();
        assert_eq!(errors.len(), FIELDS.len(), "{errors:?}");
    }

    #[test]
    fn rejects_malformed_and_non_object_json() {
        assert!(validate("{\"schema_version\":").unwrap_err()[0].starts_with("invalid JSON"));
        assert_eq!(
            validate("[]").unwrap_err(),
            vec!["top level must be a JSON object"]
        );
    }

    #[test]
    fn missing_file_is_a_read_error_not_a_pass() {
        let errors = load(Path::new("/nonexistent/intentower-project.json")).unwrap_err();
        assert!(errors[0].starts_with("cannot read "), "{errors:?}");
    }
}
