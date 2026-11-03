//! T30: the repair feedback packet.
//!
//! One round of repair gets exactly one input: this packet. It carries the
//! failed obligations of *this* round, their real sequent text, the model
//! fragment to change, and the tool's own words — and nothing else. The whole
//! point is the byte budget (`BUDGET`): a packet that would exceed it is
//! refused loudly rather than silently truncated, because a truncated proof
//! obligation is a different proof obligation.
//!
//! This is the machine half of `prompts/repair.md`; the prompt states the same
//! rules for the human and the agent, the tests keep the two in step.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::gate::provenance::{MODEL_SOURCE, PO_ARTIFACT};
use crate::po::{self, Po};
use crate::report;

/// Approved P4 parameter (docs/rossi-smoke.md §27): one feedback round ≤ 8 KB.
pub const BUDGET: usize = 8192;

/// Which failures to quote in full, most actionable first. A refuted
/// obligation is a real counterexample; an undecided one is a gap in our
/// method. Both matter more than a long list of ids.
const PRIORITY: &[&str] = &[
    "NOT_DISCHARGED",
    "MISSING",
    "STALE",
    "TOOL_ERROR",
    "TIMEOUT",
    "UNKNOWN",
    "UNSUPPORTED",
];

/// At most this many obligations are quoted in full per round. Everything
/// else is reported by type and count, which is what makes the budget hold
/// when a model grows.
const MAX_QUOTED_KINDS: usize = 2;

/// Where a repair round writes its own report. The packet must not tell the
/// agent to overwrite the report it was rendered from — that would destroy
/// the evidence of the round that failed.
const ROUND_OUT: &str = "artifacts/refine/report.json";

/// Group the not-discharged obligations by status.
pub fn grouped(report: &Value) -> BTreeMap<String, Vec<String>> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in report["obligations"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let status = row["status"].as_str().unwrap_or("?").to_string();
        if status == "SMT_DISCHARGED" {
            continue;
        }
        groups
            .entry(status)
            .or_default()
            .push(row["id"].as_str().unwrap_or("?").to_string());
    }
    groups
}

/// The statuses to quote in full, in PRIORITY order, at most MAX_QUOTED_KINDS.
fn quoted_statuses(groups: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut ordered: Vec<&String> = groups.keys().collect();
    ordered.sort_by_key(|status| {
        (
            PRIORITY.iter().position(|p| p == status).unwrap_or(99),
            status.as_str(),
        )
    });
    ordered
        .iter()
        .take(MAX_QUOTED_KINDS)
        .map(|status| (*status).to_string())
        .collect()
}

/// Load every obligation the report's pinned `.bpo` files contain, keyed by
/// the same id the report uses.
fn sequents(report: &Value) -> Result<BTreeMap<String, Po>, String> {
    let dirs: Vec<PathBuf> = report::recorded(report)
        .iter()
        .filter(|input| input.role == PO_ARTIFACT)
        .filter_map(|input| Path::new(&input.path).parent())
        .map(PathBuf::from)
        .collect();
    if dirs.is_empty() {
        return Err(
            "the report pins no .bpo file: the obligation text cannot be recovered".to_string(),
        );
    }
    let mut found = BTreeMap::new();
    for dir in dirs {
        for po in po::load_project(&dir)? {
            found.insert(po.full_id(), po);
        }
    }
    Ok(found)
}

fn model_sources(report: &Value) -> Vec<String> {
    let mut paths: Vec<String> = report::recorded(report)
        .iter()
        .filter(|input| input.role == MODEL_SOURCE)
        .map(|input| input.path.clone())
        .collect();
    paths.sort();
    paths
}

/// Render the packet. `Err` means the packet does not fit the budget, or the
/// report cannot be traced to its inputs — never a shortened packet.
pub fn render(report: &Value, budget: usize) -> Result<String, String> {
    let stale = report::freshness(report);
    if !stale.is_empty() {
        return Err(format!(
            "the report is not traceable to the current inputs, so there is nothing honest to feed back: {}",
            stale.join("; ")
        ));
    }

    let groups = grouped(report);
    let sources = model_sources(report);
    let refined: Vec<&String> = sources
        .iter()
        .filter(|path| {
            std::fs::read_to_string(path)
                .map(|body| body.to_uppercase().contains("REFINES"))
                .unwrap_or(false)
        })
        .collect();
    if refined.is_empty() {
        return Err(format!(
            "no model source of {} carries REFINES: there is no refinement layer to repair",
            report["model_id"].as_str().unwrap_or("?")
        ));
    }
    if groups.is_empty() {
        return Err(format!(
            "every obligation of {} is discharged: there is no failure to repair, and a PASS report is not a repair input",
            report["model_id"].as_str().unwrap_or("?")
        ));
    }

    let sequents = sequents(report)?;
    let quoted = quoted_statuses(&groups);
    let round = PathBuf::from(ROUND_OUT);
    let reproduce = report::reproduce(report, &round);

    let mut out = String::new();
    out.push_str("# 修复反馈包（本轮唯一输入）\n\n");

    out.push_str("## 1. 本轮判定\n\n");
    out.push_str(&format!(
        "- 判定：`{}`（退出码 `{}`）\n- 义务总数：{}，未通过：{}\n- 复现：\n\n```sh\n{}\n```\n\n",
        report["gate"]["status"].as_str().unwrap_or("?"),
        match report["gate"]["status"].as_str() {
            Some("PASS") => 0,
            Some("FAIL") => 1,
            _ => 2,
        },
        report["po_total"],
        groups.values().map(Vec::len).sum::<usize>(),
        reproduce,
    ));

    out.push_str("## 2. 失败原因按类型归纳\n\n");
    out.push_str("| 状态 | 条数 | 义务 id | 工具原文 |\n|---|---|---|---|\n");
    for (status, ids) in &groups {
        let detail = report["obligations"]
            .as_array()
            .and_then(|rows| {
                rows.iter().find(|row| {
                    row["status"].as_str() == Some(status.as_str())
                        && row["detail"].as_str().is_some_and(|d| !d.is_empty())
                })
            })
            .and_then(|row| row["detail"].as_str())
            .unwrap_or("-");
        out.push_str(&format!(
            "| `{status}` | {} | {} | {detail} |\n",
            ids.len(),
            ids.join(", ")
        ));
    }
    out.push('\n');

    out.push_str(&format!(
        "## 3. 代表义务原文（每类 1 条，最多 {MAX_QUOTED_KINDS} 类；其余按上表只给 id）\n\n"
    ));
    for status in &quoted {
        let id = &groups[status][0];
        out.push_str(&format!("### `{id}` — `{status}`\n\n"));
        match sequents.get(id) {
            Some(po) => {
                out.push_str(&format!("来源：`{}`\n\n", po.source.bpo));
                out.push_str("假设：\n\n");
                for hypothesis in po.hypothesis_texts() {
                    out.push_str(&format!("- `{hypothesis}`\n"));
                }
                out.push_str("\n目标：\n\n");
                out.push_str(&format!("- `{}`\n", po.goal_text()));
            }
            None => out.push_str(
                "在报告钉住的 .bpo 里找不到这条义务 —— 清单与文本不一致，停止修复并报告 `MISSING`。\n",
            ),
        }
        out.push('\n');
    }
    if groups.len() > quoted.len() {
        out.push_str(&format!(
            "本轮未展开的类型：{}（条数见 §2）。需要它们的原文时，下一轮再取，不得挤掉已展开的。\n\n",
            groups
                .keys()
                .filter(|status| !quoted.contains(status))
                .map(|status| format!("`{status}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    out.push_str("## 4. 相关模型片段（本轮涉及的两个文件，全文）\n\n");
    for path in &sources {
        let body = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("cannot read pinned model source {path}: {err}"));
        out.push_str(&format!("### `{path}`\n\n```eventb\n{body}```\n\n"));
    }

    out.push_str("## 5. 只改这一处\n\n");
    out.push_str(&format!(
        "- 可写：{}\n",
        refined
            .iter()
            .map(|path| format!("`{path}`"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str(&format!(
        "- 只读：{}\n",
        sources
            .iter()
            .filter(|path| !refined.contains(path))
            .map(|path| format!("`{path}`"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str(
        "- 禁止顺带重构：重命名无关变量、调整无关格式、拆分无关事件、改注释、改无关不变量\n",
    );
    out.push_str("- 禁止改测试、schema、策略、基线；禁止扩大 F0\n\n");

    out.push_str("## 6. 修完必须重跑（口头声明无效）\n\n```sh\n");
    out.push_str(&reproduce);
    out.push_str("\n./target/debug/intentower baseline\n```\n");

    let bytes = out.len();
    if bytes > budget {
        return Err(format!(
            "packet is {bytes} bytes, over the {budget} byte budget. do not truncate a proof obligation to fit: \
             narrow the model or split the round, and say so in the handoff"
        ));
    }
    Ok(out)
}

/// The byte count of the packet, for the budget check in `main`.
pub fn bytes_of(packet: &str) -> usize {
    packet.len()
}
