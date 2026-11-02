//! T12 spike: read **typed** proof obligations out of Rossi `.bpo` files
//! through `rossi_prove`'s public API (`PoProject` / `PoFile` / `ProverSequent`).
//! No XML walking, no regex on formulas (plan rule for T12).
//!
//! Repeatable: `cargo test --test po_spike -- --nocapture`

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rossi::formula::tag::{AssocPredOp, QuantPredOp, RelationalOp};
use rossi::formula::{PredicateKind, Type};
use rossi::pretty::PrettyPrinter;
use rossi_prove::{PoFile, PoProject, ProverSequent};

const COUNTER: &str = "artifacts/spike/counter-rodin";
const REFINEMENT: &str = "artifacts/spike/refinement-rodin";
const M0M1: &str = "artifacts/spike/counter-m0m1-rodin";
const M0M1BROKEN: &str = "artifacts/spike/counter-m0m1broken-rodin";

/// Integration tests run with the crate directory as cwd; the fixtures
/// live at the repository root.
fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// Load every `.bpo` of one Rodin project; chains cross component files,
/// so the whole project must be registered together.
fn project(dir: &str) -> (PoProject, Vec<String>) {
    let mut files: Vec<PathBuf> = repo(dir)
        .read_dir()
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "bpo"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no .bpo in {dir}");
    let mut project = PoProject::default();
    let mut keys = Vec::new();
    for file in files {
        let key = file.file_name().unwrap().to_str().unwrap().to_string();
        let contents = std::fs::read(&file).unwrap();
        project.insert(key.clone(), PoFile::read(contents.as_slice()).unwrap());
        keys.push(key);
    }
    (project, keys)
}

fn sequent(dir: &str, key: &str, name: &str) -> ProverSequent {
    let (project, _) = project(dir);
    project
        .load(key, name)
        .unwrap_or_else(|err| panic!("{dir}/{key} {name}: {err}"))
}

/// One obligation rendered as plain text, for reporting and cross-checks.
#[derive(Debug)]
struct Obl {
    hyps: Vec<String>,
    goal: String,
    idents: Vec<(String, String)>,
}

/// Obligation identity is **(component file, name)**: the refinement project
/// has `INITIALISATION/inv1/INV` in both components, so a name alone collides.
type Obligations = BTreeMap<(String, String), Obl>;

fn obligations(dir: &str) -> Obligations {
    let (project, keys) = project(dir);
    let printer = PrettyPrinter::rodin_formula_string();
    let mut out = BTreeMap::new();
    for key in &keys {
        for entry in project.file(key).unwrap().sequents() {
            let seq = project.load(key, &entry.name).unwrap();
            let ob = Obl {
                hyps: seq
                    .hyp_iter()
                    .map(|h| printer.print_formula_predicate(h))
                    .collect(),
                goal: printer.print_formula_predicate(seq.goal()),
                idents: seq
                    .type_env()
                    .iter()
                    .map(|(n, t)| (n.to_string(), format!("{:?}", t)))
                    .collect(),
            };
            assert!(
                out.insert((key.clone(), entry.name.clone()), ob).is_none(),
                "duplicate obligation {} in {dir}/{key}",
                entry.name
            );
        }
    }
    out
}

fn get<'a>(ob: &'a Obligations, file: &str, name: &str) -> &'a Obl {
    ob.get(&(file.to_string(), name.to_string()))
        .unwrap_or_else(|| panic!("no obligation {file}/{name}"))
}

#[test]
fn counter_obligations_load_with_typed_hypotheses_and_goal() {
    let ob = obligations(COUNTER);
    assert_eq!(ob.len(), 6, "counter must yield 6 obligations");

    let init = get(&ob, "counter.bpo", "INITIALISATION/inv1/INV");
    assert_eq!(
        init.hyps,
        ["max_value=100", "max_value>0"],
        "initialisation hypotheses are the context axioms only"
    );
    let inc = get(&ob, "counter.bpo", "increment/inv1/INV");
    assert_eq!(
        inc.hyps,
        [
            "max_value=100",
            "max_value>0",
            "count≥0",
            "count≤max_value",
            "count<max_value"
        ],
        "event hypotheses are axioms + invariants + guard"
    );
    assert_eq!(inc.goal, "count+1≥0");

    // Typed, not textual: the goal is a `≥` relation over integers.
    let seq = sequent(COUNTER, "counter.bpo", "increment/inv1/INV");
    let PredicateKind::Relational { op, .. } = seq.goal().kind() else {
        panic!("goal is not relational: {:?}", seq.goal().kind());
    };
    assert_eq!(*op, RelationalOp::Ge);
    assert_eq!(seq.type_env().get("count"), Some(&Type::Int));
    assert_eq!(seq.type_env().get("count'"), Some(&Type::Int));
    assert_eq!(seq.type_env().get("max_value"), Some(&Type::Int));
}

#[test]
fn refinement_project_yields_grd_sim_fis_obligations() {
    let ob = obligations(REFINEMENT);
    let kinds: BTreeMap<&str, usize> =
        ob.keys()
            .map(|(_, n)| n.rsplit('/').next().unwrap())
            .fold(BTreeMap::new(), |mut m, k| {
                *m.entry(k).or_insert(0) += 1;
                m
            });
    assert_eq!(
        kinds,
        BTreeMap::from([("FIS", 1), ("GRD", 3), ("INV", 13), ("SIM", 1)]),
        "refinement must produce more than INV"
    );

    // Feasibility is an existential; a guard obligation is an inequality.
    let fis = sequent(REFINEMENT, "refinement_abstract.bpo", "decrease/act1/FIS");
    let PredicateKind::Quantified { op, .. } = fis.goal().kind() else {
        panic!("FIS goal is not quantified: {:?}", fis.goal().kind());
    };
    assert_eq!(*op, QuantPredOp::Exists);

    let grd = sequent(REFINEMENT, "refinement_concrete.bpo", "increase/grd2/GRD");
    let PredicateKind::Relational { op, .. } = grd.goal().kind() else {
        panic!("GRD goal is not relational: {:?}", grd.goal().kind());
    };
    assert_eq!(*op, RelationalOp::Le);
    assert_eq!(grd.type_env().get("step_size"), Some(&Type::Int));

    // Simulation obligations are conjunctions.
    let sim = sequent(REFINEMENT, "refinement_concrete.bpo", "decrease/act1/SIM");
    let PredicateKind::Associative { op, .. } = sim.goal().kind() else {
        panic!("SIM goal is not associative: {:?}", sim.goal().kind());
    };
    assert_eq!(*op, AssocPredOp::LAnd);
}

/// Two independent readers must agree: the typed Rossi API here, and the
/// XML-based inventory of T11. A mismatch means one of them is misreading
/// the predicate-set chains.
#[test]
fn typed_api_hypothesis_counts_match_the_xml_inventory() {
    let raw = std::fs::read_to_string(repo("artifacts/spike/po-inventory.json")).unwrap();
    let inv: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let mut compared = 0usize;
    let mut total = 0usize;
    for project in inv["projects"].as_array().unwrap() {
        total += project["po_total"].as_u64().unwrap() as usize;
        let dir = project["project"].as_str().unwrap();
        let api = obligations(dir);
        for ob in project["obligations"].as_array().unwrap() {
            let name = ob["id"].as_str().unwrap();
            let file = ob["bpo_file"].as_str().unwrap();
            let from_xml = ob["hypothesis_count"].as_u64().unwrap() as usize;
            let from_api = get(&api, file, name).hyps.len();
            assert_eq!(
                from_api, from_xml,
                "{dir} {file} {name}: typed API sees {from_api} hypotheses, XML sees {from_xml}"
            );
            compared += 1;
        }
    }
    assert_eq!(
        compared, total,
        "every inventoried obligation cross-checked"
    );
    assert!(compared >= 64, "expected at least 64 obligations");
}

/// T13's own M0 -> M1 pair (two concrete integers coupled by
/// `abstract_count = a + b`). The coupling invariant is what makes the
/// guard-strengthening obligation non-trivial.
#[test]
fn counter_refinement_yields_refinement_obligations_through_the_typed_api() {
    let ob = obligations(M0M1);
    let kinds: BTreeMap<&str, usize> =
        ob.keys()
            .map(|(_, n)| n.rsplit('/').next().unwrap())
            .fold(BTreeMap::new(), |mut m, k| {
                *m.entry(k).or_insert(0) += 1;
                m
            });
    assert_eq!(
        kinds,
        BTreeMap::from([("FIS", 1), ("GRD", 2), ("INV", 16), ("SIM", 1)]),
        "M0 -> M1 must produce refinement obligations, not only INV"
    );

    let grd = get(&ob, "m1.bpo", "sub/grd1/GRD");
    assert_eq!(grd.goal, "abstract_count≥1");
    assert!(
        grd.hyps.contains(&"abstract_count=a+b".to_string())
            && grd.hyps.contains(&"b≥0".to_string())
            && grd.hyps.contains(&"delta≤a".to_string()),
        "the coupling invariant and b >= 0 must be among the hypotheses: {:?}",
        grd.hyps
    );

    // The deliberately broken variant produces the same obligation shape,
    // with a coupling invariant that no longer supports them.
    let broken = obligations(M0M1BROKEN);
    assert_eq!(
        broken.len(),
        ob.len(),
        "broken variant must not change the PO set"
    );
    let bgrd = get(&broken, "m1_broken.bpo", "sub/grd1/GRD");
    assert!(
        bgrd.hyps.contains(&"abstract_count=a − b".to_string()),
        "broken coupling must be a hypothesis: {:?}",
        bgrd.hyps
    );
}

#[test]
fn report_typed_obligations() {
    for dir in [COUNTER, REFINEMENT, M0M1, M0M1BROKEN] {
        println!("\n== {dir}");
        for ((file, name), ob) in obligations(dir) {
            println!(
                "  {:24} {:28} hyps={} goal={} idents={:?}",
                file,
                name,
                ob.hyps.len(),
                ob.goal,
                ob.idents
                    .iter()
                    .map(|(n, t)| format!("{n}:{t}"))
                    .collect::<Vec<_>>()
            );
        }
    }
}
