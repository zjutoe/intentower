//! T14: the internal `Po` structure, loaded from Rossi's typed proof
//! obligations (the T12 path: `rossi_prove`, no XML, no formula parsing).
//!
//! A `Po` carries its own hypotheses. Two obligations never share a
//! hypothesis list by accident: each is resolved through its own
//! predicate-set chain by Rossi's loader, and the tests below pin the
//! exact sets for two obligations that differ only in name.

use std::collections::BTreeMap;
use std::path::Path;

use rossi::formula::{Predicate, Type};
use rossi::pretty::PrettyPrinter;
use rossi_prove::{PoFile, PoProject};

/// A sort we can reason about when declaring SMT variables. Anything Rossi
/// reports that we do not model is kept verbatim as [`Sort::Other`] so the
/// encoder must report `UNSUPPORTED` instead of guessing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sort {
    Int,
    Bool,
    Given(String),
    Pow(Box<Sort>),
    Prod(Box<Sort>, Box<Sort>),
    Other(String),
}

impl Sort {
    fn of(ty: &Type) -> Sort {
        match ty {
            Type::Int => Sort::Int,
            Type::Bool => Sort::Bool,
            Type::Given(name) => Sort::Given(name.clone()),
            Type::Pow(inner) => Sort::Pow(Box::new(Sort::of(inner))),
            Type::Prod(left, right) => {
                Sort::Prod(Box::new(Sort::of(left)), Box::new(Sort::of(right)))
            }
            Type::Parametric { symbol, .. } => Sort::Other(symbol.clone()),
        }
    }
}

/// Obligation category, taken from the last segment of the obligation name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Inv,
    Grd,
    Sim,
    Fis,
    Seq,
    Other(String),
}

impl Kind {
    fn parse(name: &str) -> Kind {
        match name {
            "INV" => Kind::Inv,
            "GRD" => Kind::Grd,
            "SIM" => Kind::Sim,
            "FIS" => Kind::Fis,
            "SEQ" => Kind::Seq,
            other => Kind::Other(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Kind::Inv => "INV",
            Kind::Grd => "GRD",
            Kind::Sim => "SIM",
            Kind::Fis => "FIS",
            Kind::Seq => "SEQ",
            Kind::Other(name) => name,
        }
    }
}

/// Where the obligation came from: the `.bpo` file and its stamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub bpo: String,
    pub stamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Po {
    /// Obligation name inside its component, e.g. `sub/grd1/GRD`.
    pub id: String,
    pub kind: Kind,
    /// Component (machine) the obligation belongs to. Obligation names are
    /// only unique per component.
    pub machine: String,
    pub typed_hypotheses: Vec<Predicate>,
    pub typed_goal: Predicate,
    /// Solved types of every identifier in scope, including after-values
    /// such as `count'`.
    pub types: BTreeMap<String, Sort>,
    pub source: Source,
}

impl Po {
    /// Rodin-canonical text of the goal (reporting only; never parsed back).
    pub fn goal_text(&self) -> String {
        text(&self.typed_goal)
    }

    /// Rodin-canonical text of each hypothesis.
    pub fn hypothesis_texts(&self) -> Vec<String> {
        self.typed_hypotheses.iter().map(text).collect()
    }

    /// Stable identity for reports and cross-checks.
    pub fn full_id(&self) -> String {
        format!("{}::{}", self.machine, self.id)
    }
}

/// Rodin-canonical rendering. Note Rossi puts spaces only around `−`.
pub fn text(pred: &Predicate) -> String {
    PrettyPrinter::rodin_formula_string().print_formula_predicate(pred)
}

/// Read every obligation of one Rodin project directory.
///
/// All `.bpo` files of the project are registered together because
/// hypothesis chains cross component files. The result is sorted by
/// `(machine, id)` so two runs are identical and diffable.
pub fn load_project(dir: &Path) -> Result<Vec<Po>, String> {
    let mut files: Vec<std::path::PathBuf> = dir
        .read_dir()
        .map_err(|err| format!("{dir:?}: {err}"))?
        .map(|entry| {
            entry
                .map_err(|err| err.to_string())
                .map(|entry| entry.path())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "bpo"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("no .bpo file in {dir:?}"));
    }

    let mut project = PoProject::default();
    let mut names: Vec<String> = Vec::new();
    for file in &files {
        let contents = std::fs::read(file).map_err(|err| format!("{file:?}: {err}"))?;
        let name = file
            .file_name()
            .ok_or_else(|| format!("{file:?}: no file name"))?
            .to_str()
            .ok_or_else(|| format!("{file:?}: not utf-8"))?
            .to_string();
        project.insert(
            name.clone(),
            PoFile::read(contents.as_slice()).map_err(|err| format!("{file:?}: {err}"))?,
        );
        names.push(name);
    }

    let mut obligations = Vec::new();
    for name in names {
        let machine = name.trim_end_matches(".bpo").to_string();
        for entry in project.file(&name).unwrap().sequents() {
            let sequent = project.load(&name, &entry.name)?;
            let kind = Kind::parse(entry.name.rsplit('/').next().unwrap());
            obligations.push(Po {
                id: entry.name.clone(),
                kind,
                machine: machine.clone(),
                typed_hypotheses: sequent.hyp_iter().cloned().collect(),
                typed_goal: sequent.goal().clone(),
                types: sequent
                    .type_env()
                    .iter()
                    .map(|(ident, ty)| (ident.to_string(), Sort::of(ty)))
                    .collect(),
                source: Source {
                    bpo: name.clone(),
                    stamp: entry.stamp.clone(),
                },
            });
        }
    }
    obligations.sort_by(|a, b| (&a.machine, &a.id).cmp(&(&b.machine, &b.id)));
    Ok(obligations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(dir: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(dir)
    }

    fn load(dir: &str) -> Vec<Po> {
        load_project(&fixture(dir)).unwrap()
    }

    fn find<'a>(pos: &'a [Po], machine: &str, id: &str) -> &'a Po {
        pos.iter()
            .find(|po| po.machine == machine && po.id == id)
            .unwrap_or_else(|| panic!("no {machine}::{id}"))
    }

    const M0M1: &str = "artifacts/spike/counter-m0m1-rodin";

    #[test]
    fn loads_every_obligation_in_stable_order() {
        let pos = load(M0M1);
        assert_eq!(pos.len(), 20);
        let mut sorted: Vec<(&str, &str)> = pos
            .iter()
            .map(|po| (po.machine.as_str(), po.id.as_str()))
            .collect();
        sorted.sort();
        assert_eq!(
            pos.iter()
                .map(|po| (po.machine.as_str(), po.id.as_str()))
                .collect::<Vec<_>>(),
            sorted,
            "output must be sorted by (machine, id)"
        );
        let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
        for po in &pos {
            *kinds.entry(po.kind.as_str()).or_insert(0) += 1;
        }
        assert_eq!(
            kinds,
            BTreeMap::from([("FIS", 1), ("GRD", 2), ("INV", 16), ("SIM", 1)])
        );
        assert_eq!(load(M0M1), pos, "loading must be deterministic");
    }

    /// Two obligations must never carry each other's hypotheses.
    #[test]
    fn each_obligation_carries_exactly_its_own_hypotheses() {
        let pos = load(M0M1);

        assert_eq!(
            find(&pos, "m1", "INITIALISATION/inv1/INV").typed_hypotheses,
            vec![]
        );

        let sub = find(&pos, "m1", "sub/grd1/GRD");
        assert_eq!(
            sub.hypothesis_texts(),
            [
                "abstract_count≥0",
                "abstract_count≤1000",
                "a≥0",
                "b≥0",
                "abstract_count=a+b",
                "delta≥1",
                "delta≤a",
            ]
        );
        assert_eq!(sub.goal_text(), "abstract_count≥1");

        let add = find(&pos, "m1", "add/grd2/GRD");
        assert_eq!(
            add.hypothesis_texts(),
            [
                "abstract_count≥0",
                "abstract_count≤1000",
                "a≥0",
                "b≥0",
                "abstract_count=a+b",
                "delta≥1",
                "a+delta≤1000",
            ]
        );
        assert_eq!(add.goal_text(), "abstract_count+delta≤1000");
        assert_ne!(sub.typed_hypotheses, add.typed_hypotheses);

        // The abstract machine's obligations must not inherit the concrete
        // machine's invariants.
        let m0_add = find(&pos, "m0", "add/inv1/INV");
        assert_eq!(
            m0_add.hypothesis_texts(),
            [
                "abstract_count≥0",
                "abstract_count≤1000",
                "delta≥1",
                "abstract_count+delta≤1000"
            ]
        );
    }

    /// Obligation names repeat across components; identity is (machine, id).
    #[test]
    fn same_name_in_two_components_stays_separate() {
        let pos = load("artifacts/spike/refinement-rodin");
        let same: Vec<&Po> = pos
            .iter()
            .filter(|po| po.id == "INITIALISATION/inv1/INV")
            .collect();
        assert_eq!(same.len(), 2);
        assert_eq!(same[0].machine, "refinement_abstract");
        assert_eq!(same[1].machine, "refinement_concrete");
        assert_eq!(same[0].source.bpo, "refinement_abstract.bpo");
        assert_eq!(same[1].source.bpo, "refinement_concrete.bpo");
        let ids: BTreeMap<String, usize> =
            pos.iter()
                .map(|po| po.full_id())
                .fold(BTreeMap::new(), |mut m, k| {
                    *m.entry(k).or_insert(0) += 1;
                    m
                });
        assert_eq!(ids.values().sum::<usize>(), 18);
        assert_eq!(ids.len(), 18, "full_id must be unique");
    }

    #[test]
    fn types_are_mapped_to_sorts() {
        let pos = load("artifacts/spike/counter-rodin");
        let po = find(&pos, "counter", "increment/inv1/INV");
        assert_eq!(po.types.get("count"), Some(&Sort::Int));
        assert_eq!(po.types.get("count'"), Some(&Sort::Int));
        assert_eq!(
            po.types.get("STATUS"),
            Some(&Sort::Pow(Box::new(Sort::Given("STATUS".into()))))
        );
    }

    #[test]
    fn a_directory_without_bpo_fails_loudly() {
        let err = load_project(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")).unwrap_err();
        assert!(err.contains("no .bpo file"), "unexpected error: {err}");
    }
}
