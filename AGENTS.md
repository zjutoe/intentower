# AGENTS.md — Intentower (意浮图)

**Purpose:** Execute small, verifiable tasks for the Intentower Phase 1 prototype using Pi + Strata. Do not redesign the project or claim verification without evidence.

**Principle:** *Refine Downward. Verify Upward.*

## 1. Before every task

1. Execute **one assigned task ID** only. The task specification is in `Intentower_Phase1_Research_Development_Plan.md` or a `tasks/TASK-*.md` file.
2. Read this file, the task, and only the explicitly relevant input files. Do not load the entire repository into context.
3. Identify the approved parent Intent and its relevant clauses, the parent Spec, `allowed_edits`, inputs, outputs, and exact acceptance checks. For infrastructure-only work, record `parent: infrastructure-only`.
4. Inspect the Git state and protected files before editing. Do not overwrite existing changes.
5. Confirm actual CLI/API behavior with local `--help`, versions, source or tests. Never guess.
6. If the Intent leaves a consequential choice unresolved, or the work requires changing its commitments, **stop the dependent work and request a human decision** using §3. Correct a derived Spec that conflicts with Intent through technical review; do not ask humans to approve the whole Spec.

## 2. Task size and scope

- Produce **one independently testable result** per task.
- Change at most **3 production files** by default, plus direct tests/docs. Split larger tasks before coding.
- Make the smallest change that satisfies the agreed acceptance checks.
- Do not introduce frameworks, abstractions, features, dependencies or refactors that the task does not require.
- Do not start the next task after completion. Return a handoff and stop.
- Do not combine new requirements, architecture, implementation and final approval in one weak-model task.
- Ask for a task split or stronger review if you must invent the design to continue.

## 3. Human approval and protected files

Human-approved **Intent is the content authority**. Humans review Intent, including consequential choices found while deriving lower-level artifacts. Agents derive and technically review Specs, models and tests under that Intent; these artifacts do not require separate human content approval.

**Intent decision loop (human-directed revision, 2026-11-09):**

1. Trace each Spec requirement and relevant assumption to approved Intent clauses and recorded decisions. Complete the task's actual checks and use independent technical review for material correctness risks. Record unverified downstream work honestly.
2. If derivation exposes ambiguity, a consequential choice not settled by Intent, a new assumption, a narrower scope, or a changed observable commitment, pause only the dependent work. Present the relevant Intent clauses, the unresolved choice, options with effects, and a recommendation. Do not choose on the human's behalf while waiting.
3. After an explicit human decision, the agent may faithfully backfill that decision into the relevant Intent and record its source. Do not invent a signature, identity or date. Then update the Spec and its traceability; do not request approval of the same decision again. A further change beyond the decision requires another decision.
4. Resolve task-local implementation details that preserve Intent autonomously. A new Spec uses `proposed` until its technical review and task-scoped checks are complete and all consequential choices are settled, then `reviewed`. Record the parent Intent revision, traceability, reviewer/check evidence and remaining limits. `reviewed` does not mean formally proved or implemented.

Preserve historical human signatures and `approved` statuses as historical facts; do not add new human Spec/model approval blocks. Deriving a Spec is not proof that natural-language Intent has been captured correctly. Content approval and approval of a concrete protected operation are separate.

Protected after approval, technical review or baseline freezing, as applicable:

- `docs/INT-*.md` and reviewed or historically approved `docs/SPEC-*.md`
- Reviewed or historically approved Event-B models, including M0
- `approvals/*` (especially `approvals/baseline.json`)
- Approved policies such as baselined `project.json`

A task may create a **proposed draft** (for example T02/T03), but creation does not mean approval. Never silently edit approved files, weaken requirements, update baselines, delete tests, or rewrite project policy to make a task pass. Do not modify this `AGENTS.md` or the phase plan without an explicit task. Escalate any necessary protected change with a concise semantic diff.

Within approved Intent, an explicitly scoped task may revise a derived Spec/model through technical review. This delegation does not unfreeze pinned bytes: record the delta and required checks, and obtain separate approval before changing baseline pins. Escalate content to the human when the Intent decision loop requires it, not merely because a derived file changed.

**Baseline pinning and repinning (human-directed revision, 2026-11-09):** The agent must first show the complete commands it will execute, including any script body, and the proposed diff with affected paths, old/new hashes and model mappings where applicable. It must then explicitly request human approval for that concrete operation. Only after receiving that approval may the agent execute the displayed commands. Approval applies to the displayed commands, content and scope; any change before execution requires a new display and approval. The underlying Intent must be human-approved; derived Specs, models and policy must have the required technical checks and no unresolved consequential choices. This operational approval does not require humans to read the whole derived artifact. Do not invent approval identities or dates, and never refresh a baseline automatically to make checks pass. Approval of this workflow change is not approval of a particular pinning operation.

This workflow replaces the former requirement that humans personally write the baseline. The P1-era read-only wording in `approvals/baseline.json`'s `note` and older approval records describes the previous workflow; preserve those historical records. Their old wording does not prohibit an operation explicitly approved under this rule.

**Note:** Hash comparisons are workflow guards, not security isolation from malicious local processes.

## 4. Traceability and refinement

Maintain this relation where applicable:

`INT-* -> SPEC-* -> MODEL-* (M0 -> M1) -> PO -> Evidence`

- State what each lower-level model adds and what higher-level behavior it preserves.
- Explain assumptions, invariants, and gluing relations that matter to the task.
- Preserve source-to-PO traceability; do not fabricate IDs or evidence.
- Keep **model verification** separate from **source-code testing**. Passing one never proves the other.
- Do not generate empty documents or unsupported proof claims simply to complete the hierarchy.

## 5. Verification rules (strict)

- Prefer existing tools: **Rossi** for Event-B parsing/checking/POG, **cvc5** for SMT, Rodin/ProB for optional independent checks.
- `rossi prove` checks stored proofs; it is **not** a general automatic search for new proofs.
- The first SMT subset is **Boolean logic + quantifier-free linear integer arithmetic**. Add other theories only by an approved, tested task.
- Use **typed AST / sequents**. Do not translate Event-B mathematical expressions using regex or unvalidated text replacement.
- A PO encoding must preserve **all applicable hypotheses, types, context and well-definedness requirements**; assert the negation of its goal for SMT validity checking.
- Unsupported expressions must yield `UNSUPPORTED`, never be skipped or replaced with a convenient constant.
- Interpret solver results carefully: `unsat` -> `SMT_DISCHARGED` (assuming sound encoding); `sat` -> `NOT_DISCHARGED` (candidate counterexample); `unknown` -> no proof.
- Other non-success states: `TIMEOUT`, `UNSUPPORTED`, `TOOL_ERROR`, `STALE`, `MISSING`.
- Count **all** POs generated from the current model. Missing, duplicate, stale or unproved POs mean **no PASS**. Phase 1 also rejects zero POs for proof-required tasks.
- A Gate must rebuild/reverify current inputs. A historical or agent-edited JSON report cannot authorize PASS.
- `rossi validate`, `rossi export --build`, a passing test suite, or `SMT_DISCHARGED` do **not** prove arbitrary Rust/Python/C++ code correct. An SMT result is not automatically a kernel-checked certificate.

If a tool cannot run, report `blocked` or `incomplete`; never substitute invented output.

## 6. Execution and repair limit

1. Implement the assigned change, then run the task's **real positive and negative tests**.
2. Record commands, exit codes and important output. Mark unexecuted checks `not_run`.
3. Inspect `git diff` for unrelated changes and protected-file edits.
4. Attempt at most **two repairs** for a failed task. Then stop for human/Codex review.
5. In automated-refinement experiments, attempt at most **three candidate verifications** unless the approved task says otherwise.
6. Apply timeouts to external tools. Do not install/upgrade dependencies, publish packages, create remotes, or run destructive commands without explicit authorization.

A stub, TODO, mock or skipped check does not satisfy a task that requires a real implementation or a real solver run.

## 7. Handoff format (required)

After each task, return a short report using these fields. Do not claim a test ran unless it actually ran.

```yaml
task_id: Txx
status: done | blocked | needs_review
changed_files: []
commands_run: []       # each command + actual exit code/result
acceptance: []         # passed | failed | not_run for each check
proof_obligations:
  status: not_applicable  # or applicable
  total: null
  discharged: null
  not_discharged: null
  unknown: null
  unsupported: null
remaining_risks: []
intent_or_spec_changed: false
next_task_suggestion: Tyy
```

For a task that requires formal proof, `total: 0` is a failed check, not `not_applicable`. If Intent needs a new decision, request it before editing the affected content and faithfully backfill only the received decision. Spec/model changes within Intent follow §3 technical review and task scope; baseline writes still need explicit operational approval.

## 8. Phase gates and escalation

- **P0:** Human approves top-level Intent and MVP scope; agents derive and technically review the Spec, returning unresolved choices to Intent.
- **P1:** Verify typed PO access and technically review M0 against Intent/Spec; human approves the concrete protected-baseline operation, without a separate model content-approval gate.
- **P2:** Codex/strong reviewer checks Event-B -> SMT semantics and failure handling.
- **P3:** Negative tests must prevent false PASS.
- **P4:** Human checks the agent did not change approved intent.
- **P5:** Dogfooding review distinguishes model proofs from code tests.

An unsuccessful experiment is a valid **research result**, not a successful MVP. Keep failures visible and preserve original acceptance criteria.

## 9. Default first action and references

Unless a different task is explicitly assigned, execute **T01 only**: initialize the smallest Rust CLI with `--help` and `--version`; do not integrate Rossi, cvc5 or orchestration yet.

Read external docs only when needed: [Rossi](https://github.com/eventb-rossi/rossi), [Event-B skill](https://github.com/eventb-rossi/eventb-skill), [cvc5](https://cvc5.github.io/docs/latest/), [Strata](https://github.com/Niko1221/Strata).

**Never make Intentower appear more complete, more verified, or more aligned with human intent than the real evidence establishes.**
