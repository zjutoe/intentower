# Intentower · 意浮图

**Refine Downward. Verify Upward.**

> **Status:** Phase 1 research and development. This repository is an early-stage toolchain experiment, **not a working end-to-end verifier yet**. Command examples marked *planned* are design targets, not released features.

Intentower is an **intent-driven, verification-assisted software engineering toolchain** for coding agents. It aims to help humans guide software projects through a hierarchy of increasingly concrete specifications, while tools provide evidence that each refinement preserves the intended behavior.

**中文简介：** 意浮图让人类仅审阅 INT，由 Coding Agent 展开并核验 SPEC、模型和实现。展开时遇到需要明确的选择，Agent 展示选项与影响，由人类裁定后回填 INT，再继续；工具提供自下而上的证据。目标是降低理解、审查和维护 AI 生成软件的认知负担。

The name combines **intent** with **tower**: a small, understandable top level supported by larger, more detailed levels below.

## The problem

Coding agents can write far more code than a human can carefully review. Passing tests alone does not establish that the agent understood the original intent. Intentower explores a workflow in which a human can inspect concise behavioral commitments and the evidence supporting them, and drill down only where needed.

The central distinction is:

- **Intent:** What the human actually wants and what must never change without approval.
- **Specification:** Explicit, reviewable behavioral obligations and assumptions.
- **Refinement:** A lower-level model introduces detail while maintaining a formally defined relationship to the higher-level model.
- **Implementation:** Executable code that must eventually be linked to the specification by appropriate evidence.
- **Evidence:** Real tool outputs, proofs or checks, test results, provenance and known gaps.

A checked Event-B model is **not** automatically a proof that separately written Rust, Python or C++ code implements that model.

## Approach

```text
Human reviews Intent and resolves open choices
                   |
                   v
 Coding agent derives and reviews Specification
                   |
                   v
      Event-B models M0 -> M1 -> ...
                   |
                   v
       Rossi: parse / check / POG
                   |
                   v
        Typed proof obligations
                   |
                   v
    Intentower: restricted SMT encoding
                   |
                   v
           cvc5: SMT solving
                   |
                   v
 Evidence report + strict acceptance gate
                   |
    unresolved choices -> human -> Intent
```

**Design principles:**

1. Human-approved Intent is authoritative. Agents derive and technically review Specs and models; unresolved consequential choices return to the human and are recorded in Intent before dependent work continues. Humans do not separately approve derived artifacts.
2. Refine in small, reviewable steps; preserve traceability between levels.
3. Use existing verification infrastructure before building new infrastructure.
4. Treat `unknown`, `timeout`, `unsupported`, missing and stale evidence as **not proved**.
5. Report the precise scope of each guarantee. Never equate passing tests or model proofs with full source-code correctness.
6. Prefer independent, machine-executed evidence to an agent's confidence statement.

Human content review and operational approval are separate. Baseline pinning still requires the agent to display complete commands and the concrete diff, obtain explicit human approval, then execute. Historical signatures are retained; technical review does not prove that natural-language Intent was captured correctly. See `AGENTS.md` §3.

## Phase 1: smallest useful vertical slice

The first phase uses six incremental milestones. These are **targets**, not claims of completion.

| Stage | Goal | Exit evidence |
| --- | --- | --- |
| **P0 — Foundation** | Define Intent, Spec, agent rules, repository and task templates. | Human-approved Intent, resolved choices and technically reviewed derived Spec. |
| **P1 — Tool feasibility** | Confirm Pi + Strata, Rossi PO generation and typed PO access, and cvc5 execution. | Reproducible model and typed PO inventory. |
| **P2 — SMT prototype** | Encode a deliberately small Event-B formula subset (initially Boolean logic and quantifier-free linear integer arithmetic). | Real cvc5 discharge of supported INV and refinement obligations; negative examples are rejected. |
| **P3 — Evidence gate** | Record complete PO inventory, provenance and precise statuses. | Missing, stale, unsupported or failed obligations cannot produce PASS. |
| **P4 — Agent loop** | Let Pi generate a refinement, verify it and attempt bounded repairs. | At least one reproducible M0 → M1 experiment without changing approved intent. |
| **P5 — Dogfooding** | Specify, model, implement and review Intentower's own acceptance gate. | Intent → Spec → Model → PO → Evidence → Code/Tests linkage and an evaluation report. |

The detailed execution plan contains **T01–T41** and explicit phase gates. Keep `Intentower_Phase1_Research_Development_Plan.md` available to the executing agent; it is the source for task IDs, dependencies, expected artifacts and acceptance criteria.

**Out of scope for Phase 1:** a new general-purpose language, a full Event-B prover, GUI/web platform, arbitrary Event-B mathematical support, proof of arbitrary source code, and a new multi-agent orchestration framework.

## Intended tools

| Tool | Role | Phase 1 stance |
| --- | --- | --- |
| [Pi](https://pi.dev/) | Coding agent and tool execution | Primary execution harness. |
| [Strata](https://github.com/Niko1221/Strata) | Local model serving for Pi | Default model backend; keep tasks small. |
| [Rossi](https://github.com/eventb-rossi/rossi) | Event-B parser, checker, proof-obligation generation and stored-proof checking | Prefer its typed interfaces and existing CLI. It does **not** generally search for new proofs. |
| [cvc5](https://cvc5.github.io/docs/latest/) | SMT solver | Prove supported, faithfully encoded obligations by checking the negated goal for unsatisfiability. |
| [Rodin](https://www.event-b.org/) / [ProB](https://prob.hhu.de/) | Independent reference / model checking | Optional for cross-checking and counterexamples; not required as Phase 1 runtime dependencies. |
| Codex | Independent high-risk reviews | Use at phase gates and on semantics/security-sensitive changes, not on every small task. |

SMT is the **method** (Satisfiability Modulo Theories); cvc5 is a **concrete solver**. The Event-B → SMT translation is a trust-sensitive component, not just text replacement.

## Getting started (current development state)

This is a research repository. There is **no released `intentower` executable yet**. To begin Phase 1:

1. Make the Phase 1 research/development plan available in the repository.
2. Read this README and [`AGENTS.md`](AGENTS.md).
3. Confirm local versions and connectivity before installing or updating tools; do not assume a particular version or command is available.
4. Use Pi + Strata to execute **only T01** from the plan, then validate its explicit acceptance commands and review its handoff before assigning T02.
5. Continue one small task at a time. Technical phase gates require their specified evidence and reviews; request human decisions for unresolved Intent choices and explicit authorization for concrete protected operations. Stop after the assigned task rather than starting the next one automatically.

The CLI surface implemented so far (`intentower --help` is authoritative):

```sh
intentower verify models/counter --model-id MODEL-004 --out artifacts/counter
intentower gate models/counter --model-id MODEL-004 --policy project.json --out artifacts/gated-counter/report.json
intentower report artifacts/counter/report.json --format md --out artifacts/counter/summary.md
intentower baseline
```

`doctor` and `po` are not implemented. Exit codes are `0` PASS, `1` FAIL, `2`
INCOMPLETE or tool/usage error; `report` runs nothing and authorizes nothing.
`baseline` compares the human-frozen `approvals/baseline.json` (four SHA256
values plus the source sets behind `MODEL-001` and `MODEL-004`) against the
working tree; it reads only, never refreshes the baseline, and exits `2` the
moment a protected input moves. `verify` and `gate` do not read it — a
refinement round must run it before and after.
`verify models/counter --model-id MODEL-001` exits `2` by design: its
`m0::sub/act1/FIS` obligation contains `∃`, which is outside the F0 (QF_LIA)
subset — see SPEC-001 §13.1.
These commands are provisional. Verify actual flags against implemented `--help`; do not present examples here as successful test results.

## Evidence and acceptance

Intentower's planned verification pipeline records **all** generated proof obligations, not just the ones its SMT backend can translate. Phase 1 accepts a model only when the current build succeeds, the required nonempty PO inventory is complete, all required obligations have a supported and trusted `SMT_DISCHARGED` result, and approved inputs match their recorded baseline.

An obligation may be reported as `SMT_DISCHARGED`, `NOT_DISCHARGED`, `UNKNOWN`, `TIMEOUT`, `UNSUPPORTED`, `TOOL_ERROR`, `STALE`, or `MISSING`. A `sat` result for the negated PO is initially a **candidate counterexample**, not an automatically validated Event-B trace. `SMT_DISCHARGED` is **not** a Lean-kernel-checked proof certificate.

The acceptance gate must re-run verification on **current** source inputs; it cannot trust an editable historical JSON report as authorization to pass. Agents may revise derived Specs and high-level models within an explicitly scoped task under approved Intent and the required technical review. Unresolved choices return to the human and are recorded in Intent. Approval-record and baseline writes require explicit authorization of the displayed operation; model edits do not automatically refresh pins. Simple hash checks are workflow safeguards, **not** isolation against a malicious local process.

## Repository layout (target, created incrementally)

```text
intentower/
├── README.md
├── AGENTS.md
├── Intentower_Phase1_Research_Development_Plan.md
├── project.json
├── docs/                  # INT-*, SPEC-*, methodology, decisions
├── tasks/                 # Small bounded tasks and handoffs
├── models/                # Event-B reference models and refinements
├── crates/intentower-cli/ # Planned Rust CLI
├── schemas/               # Machine-readable evidence schema
├── tests/fixtures/        # Positive, negative, and fault-injection inputs
├── evidence/              # Reproducible evidence and experiment reports
├── approvals/             # Human-approved baselines (phase gate)
└── artifacts/             # Generated scratch output; normally untracked
```

Files and directories not yet created are **planned**. Do not generate empty scaffolding unless a task requires it.

## Working with coding agents

`AGENTS.md` defines task size, protected files, the repair limit, evidence rules and required handoff fields. Pi + Strata should execute **one independently testable task at a time**, normally with no more than three production files modified. After two unsuccessful repairs, stop and ask for review instead of widening the task or weakening its contract.

**Human review focuses on:** approved intent and behavior, new assumptions, changes to cross-level semantics, incomplete proof obligations, and unverified risks. Source diffs remain available for drilling down; they are not replaced by a misleading universal “verified” badge.

## Research status and limitations

The questions under study include whether AI-assisted formal refinement is reliable, whether verification is fast enough for local-agent iteration, and whether higher-level review measurably reduces cognitive cost. Phase 1 does **not** establish these claims in advance; it collects reproducible evidence for them.

Project implementation, tests, benchmarks, licensing and public release details will be filled in as they actually exist. **No license is declared by this README.**

## References

- [Event-B / Rodin](https://www.event-b.org/) — refinement and proof obligations.
- [Rossi](https://github.com/eventb-rossi/rossi) — Event-B frontend, POG and stored-proof checking.
- [Rossi Event-B agent skill](https://github.com/eventb-rossi/eventb-skill) — existing agent-oriented modeling workflows.
- [cvc5 documentation](https://cvc5.github.io/docs/latest/) — SMT solving and SMT-LIB.
- [Pi](https://pi.dev/) and [Strata](https://github.com/Niko1221/Strata) — intended local coding-agent workflow.
