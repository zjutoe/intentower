//! The frozen report 1.0 contract, without a new runtime dependency.
//! Independent Draft202012Validator tests keep these explicit checks aligned
//! with schemas/report.schema.json and scripts/validate-report.py.

use std::collections::BTreeSet;

use serde_json::Value;

pub(super) fn validate(report: &Value) -> Result<(), String> {
    fields(
        report,
        "report",
        &[
            "schema_version",
            "model_id",
            "inputs",
            "tools",
            "build_ok",
            "po_total",
            "obligations",
            "issues",
            "gate",
            "scope",
        ],
        &[],
    )?;
    if report["schema_version"] != "1.0" {
        return Err("schema_version must be 1.0".to_string());
    }
    let model = pattern_text(&report["model_id"], "model_id")?;
    if !model
        .strip_prefix("MODEL-")
        .is_some_and(|suffix| suffix.len() == 3 && suffix.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err("model_id must match ^MODEL-[0-9]{3}$".to_string());
    }
    let build_ok = report["build_ok"]
        .as_bool()
        .ok_or("build_ok must be a boolean")?;
    integer(&report["po_total"], "po_total")?;

    let inputs = array(&report["inputs"], "inputs")?;
    if inputs.is_empty() {
        return Err("inputs must contain at least one input".to_string());
    }
    for (index, input) in inputs.iter().enumerate() {
        let path = format!("inputs/{index}");
        fields(input, &path, &["role", "path", "sha256"], &[])?;
        choice(
            &input["role"],
            &format!("{path}/role"),
            &["model_source", "po_artifact", "policy", "other"],
        )?;
        text(&input["path"], &format!("{path}/path"))?;
        let hash = pattern_text(&input["sha256"], &format!("{path}/sha256"))?;
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(format!(
                "{path}/sha256 must be 64 lowercase hexadecimal digits"
            ));
        }
    }

    let tools = &report["tools"];
    fields(tools, "tools", &["rossi", "cvc5"], &["kernel"])?;
    for name in ["rossi", "cvc5", "kernel"] {
        if let Some(version) = tools.get(name) {
            text(version, &format!("tools/{name}"))?;
        }
    }

    let obligations = array(&report["obligations"], "obligations")?;
    let mut ids = BTreeSet::new();
    for (index, row) in obligations.iter().enumerate() {
        let path = format!("obligations/{index}");
        fields(
            row,
            &path,
            &["id", "kind", "status", "elapsed_ms"],
            &["detail"],
        )?;
        let id = text(&row["id"], &format!("{path}/id"))?;
        if !pattern_text(&row["id"], &format!("{path}/id"))?
            .split_once("::")
            .is_some_and(|(component, name)| {
                !component.is_empty()
                    && component
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    && !name.is_empty()
                    && name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"_/.".contains(&byte))
            })
        {
            return Err(format!("{path}/id is not a component-qualified PO id"));
        }
        if !ids.insert(id) {
            return Err(format!("duplicate obligation id: {id}"));
        }
        choice(
            &row["kind"],
            &format!("{path}/kind"),
            &["INV", "GRD", "SIM", "FIS", "SEQ", "OTHER"],
        )?;
        choice(
            &row["status"],
            &format!("{path}/status"),
            &[
                "SMT_DISCHARGED",
                "NOT_DISCHARGED",
                "UNKNOWN",
                "TIMEOUT",
                "UNSUPPORTED",
                "TOOL_ERROR",
                "STALE",
                "MISSING",
            ],
        )?;
        integer(&row["elapsed_ms"], &format!("{path}/elapsed_ms"))?;
        if row.get("detail").is_some_and(|detail| !detail.is_string()) {
            return Err(format!("{path}/detail must be a string"));
        }
    }
    let count_matches = match report["po_total"].as_u64() {
        Some(total) => total == obligations.len() as u64,
        None => report["po_total"].as_f64() == Some(obligations.len() as f64),
    };
    if !count_matches {
        return Err(format!(
            "po_total {} != {} obligations listed",
            report["po_total"],
            obligations.len()
        ));
    }

    for (index, issue) in array(&report["issues"], "issues")?.iter().enumerate() {
        let path = format!("issues/{index}");
        fields(issue, &path, &["code", "message"], &["obligation_id"])?;
        let code = pattern_text(&issue["code"], &format!("{path}/code"))?;
        if code.len() < 3
            || !code.as_bytes()[0].is_ascii_uppercase()
            || !code
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(format!("{path}/code must match ^[A-Z][A-Z0-9_]{{2,}}$"));
        }
        text(&issue["message"], &format!("{path}/message"))?;
        if let Some(id) = issue.get("obligation_id") {
            text(id, &format!("{path}/obligation_id"))?;
        }
    }

    let gate = &report["gate"];
    fields(gate, "gate", &["status", "reasons"], &[])?;
    let verdict = choice(
        &gate["status"],
        "gate/status",
        &["PASS", "FAIL", "INCOMPLETE"],
    )?;
    let reasons = array(&gate["reasons"], "gate/reasons")?;
    for reason in reasons {
        text(reason, "gate/reasons entry")?;
    }
    if verdict == "PASS" {
        if !reasons.is_empty() {
            return Err("gate/reasons must be empty for PASS".to_string());
        }
        if !build_ok
            || obligations.is_empty()
            || obligations
                .iter()
                .any(|row| row["status"] != "SMT_DISCHARGED")
        {
            return Err(
                "PASS requires build_ok, nonzero po_total and all obligations SMT_DISCHARGED"
                    .to_string(),
            );
        }
    } else if reasons.is_empty() {
        return Err("gate/reasons must explain a non-PASS verdict".to_string());
    }

    let scope = &report["scope"];
    fields(
        scope,
        "scope",
        &[
            "eventb_model_smt_checked",
            "implementation_formally_verified",
        ],
        &[],
    )?;
    if !scope["eventb_model_smt_checked"].is_boolean() {
        return Err("scope/eventb_model_smt_checked must be a boolean".to_string());
    }
    if scope["implementation_formally_verified"] != false {
        return Err("scope/implementation_formally_verified must be false".to_string());
    }
    Ok(())
}

fn fields(value: &Value, path: &str, required: &[&str], optional: &[&str]) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))?;
    for name in required {
        if !object.contains_key(*name) {
            return Err(format!("{path}/{name} is required"));
        }
    }
    for name in object.keys() {
        if !required.contains(&name.as_str()) && !optional.contains(&name.as_str()) {
            return Err(format!("{path}/{name} is not an allowed field"));
        }
    }
    Ok(())
}

fn text<'a>(value: &'a Value, path: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("{path} must be a nonempty string"))
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{path} must be an array"))
}

fn choice<'a>(value: &'a Value, path: &str, choices: &[&str]) -> Result<&'a str, String> {
    let name = text(value, path)?;
    if !choices.contains(&name) {
        return Err(format!("{path} is not one of {}", choices.join(", ")));
    }
    Ok(name)
}

fn integer(value: &Value, path: &str) -> Result<(), String> {
    // JSON Schema integer is mathematical: 1.0 and -0.0 are valid too.
    if value.as_u64().is_some()
        || value
            .as_f64()
            .is_some_and(|number| number >= 0.0 && number.fract() == 0.0)
    {
        Ok(())
    } else {
        Err(format!("{path} must be a nonnegative integer"))
    }
}

fn pattern_text<'a>(value: &'a Value, path: &str) -> Result<&'a str, String> {
    let name = text(value, path)?;
    // The frozen patterns end in '$', which also matches before a final LF.
    // Preserve the original field; this only mirrors the pattern's semantics.
    Ok(name.strip_suffix('\n').unwrap_or(name))
}
