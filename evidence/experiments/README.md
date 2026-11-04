# T33 — refinement experiment record

Task: **T33** (P4). One abstract machine, one Agent-generated refinement, every run recorded.
Orchestrator: `scripts/run-refinement.sh` (T31). Caps as approved in P4: **3 verifications, 2 repairs, 8 KB feedback per round**.

Date of all runs: 2026-11-03 (UTC). Model served by `strata-ai90 / qwen3.8-flash-next-iq3_s`
(SSH tunnel to ai89; the bonsai2-27b service has been down since 2026-09-30 — see `approvals/P3-GATE-R2.md` §1).

## 1. What was measured

- abstract machine: **MODEL-005** = `models/thermostat/th0.eventb`, human-approved and pinned in `approvals/baseline.json`
- refinement: **MODEL-006** = `th0.eventb` + `th1.eventb`, where `th1.eventb` is written by Pi and is **not** pinned
- between runs `models/thermostat/th1.eventb` is deleted, so each run must produce a refinement from scratch
- `intentower baseline` runs before and after every Pi call; every run below ended `BASELINE OK: 5 hashes, 3 model ids pinned`

## 2. Three main runs (no seed, no `ALLOW_EXISTING_M1`)

| run | verdict | POs | statuses | verif | repairs | pi calls | input | output | cacheRead | total tokens | peak ctx | pi s | gate s | th1 sha256 (12) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A (`run-A.json`) | PASS | 33 | 33 SMT_DISCHARGED | 1 | 0 | 16 | 15304 | 7368 | 263964 | 286636 | 23588 | 151 | 1 | 3485b71fd0d1 |
| B (`run-B.json`) | PASS | 33 | 33 SMT_DISCHARGED | 1 | 0 | 20 | 16773 | 11074 | 375939 | 403786 | 30354 | 231 | 0 | 2ac7339f6d66 |
| C (`run-C.json`) | PASS | 33 | 33 SMT_DISCHARGED | 1 | 0 | 13 | 14755 | 6864 | 198685 | 220304 | 24412 | 147 | 1 | 50cd12e7a50b |

All three runs stopped with `stop_reason: pass` and gate exit 0. No run was discarded.
The three refinements are **not identical**: A and B use `heater ∈ {0,1}` with thresholds 70/30,
C uses the same shape with thresholds 80/20 (`models/th1-runA.eventb`, `th1-runB.eventb`, `th1-runC.eventb`).

## 3. Supplementary seeded run (labelled separately, not mixed into the table above)

Input seed: `models/th1-seed-input.eventb` — a legal `th1 REFINES th0` whose guards are too loose.
Pre-gate (`seed-1-pregate.json`, exit 1): `FAIL 33 obligations`, exactly
`2 obligation(s) NOT_DISCHARGED: th1::chill/inv6/INV, th1::warm/inv5/INV`.

Result (`seed-1.json`): gate **PASS 33/33** at verification 1, `repairs_used: 0`, 26 pi calls,
444375 tokens, peak context 26260, pi 215 s, gate 0 s. Output `models/th1-seed-fixed.eventb`
tightens the guards to `temperature + 1 <= 70` / `temperature - 1 >= 30`.

Why `repairs_used: 0`: the agent ran the same `gate` itself inside its own turn. Its self-check
report is kept as evidence (`agent-selfcheck-round-1.json` inside the run directory) and says
`FAIL 33` with the same two obligations; the agent then fixed the model and the orchestrator's
gate passed. The repair happened **inside one Pi turn**, not through the orchestrator's feedback packet.

## 4. Attempts before the harness was fixed (kept, not hidden)

- `prefix-run1.json` + `models/th1-prefix-run1.eventb`: first attempt, PASS 33/33, 24 calls,
  539356 tokens, peak 35289, pi 209 s. The Pi agent wrote `summary.md`,
  `negative-control-report.json`, `negative-control-summary.md` **into the evidence directory**.
- a second attempt (07:16 UTC) ended with the harness failing to write `run.json`: the agent had
  created `round-2/` and `round-3/` inside the evidence directory, and the writer expected
  `round-*/pi.jsonl`. The orchestrator's gate for that attempt was `PASS 33`. Its artifacts were
  lost when `artifacts/refine/` was cleared before the next attempt; only the console record remains.

Both defects are fixed in `scripts/run-refinement.sh`:
the agent's self-check output is redirected to `/tmp/intentower-refine-scratch/...` and copied back
as `agent-selfcheck-*`, rounds without `pi.jsonl` are skipped, and anything in the evidence
directory the orchestrator did not create is listed in `run.json: unexpected_entries` instead of
being trusted or silently deleted. The three main runs and the seeded run above all report
`unexpected_entries: []`.

## 5. Harness regression evidence (deterministic, no LLM)

`harness/P4-T31-dry-cap-exhausted.json`: `DRY_RUN=1` against MODEL-002 (a known-bad refinement) —
3 verifications, 2 repairs, `stop_reason: verify_cap_exhausted`, gate `FAIL` (exit 1) in all three
rounds, script exit 2, repair packets 3645 / 3645 bytes under the 8 KB budget.
`harness/P4-T31-dry-pass.json`: `DRY_RUN=1` against MODEL-004 — 1 verification, `stop_reason: pass`, exit 0.

After T34 the same two runs also write `escalation.md`. The post-T34 copies are kept as
`harness/P4-T34-dry-cap-exhausted-run.json` + `P4-T34-dry-cap-exhausted-escalation.md`
(`next=EXPERT_OR_SHRINK`, because one obligation is `UNSUPPORTED`) and
`harness/P4-T34-dry-pass-run.json` + `P4-T34-dry-pass-escalation.md` (`next=NONE`, `DRY_RUN=1` against
MODEL-006 with a pre-existing `th1`). The escalation record is decided only by the gate report and
the orchestrator counters: an agent's own claim of having fixed the model is not an input, which
`tests/feedback.rs::the_next_step_is_read_from_the_report_not_from_what_the_agent_claims` asserts by
comparing the output byte for byte with and without the claim.

`harness/P4-T31-gate-exit0-no-report-stop.json` is a negative control run through a wrapper binary
(`INTENTOWER_BIN`) that lets `gate` exit 0 while writing no `report.json`. The loop stops with exit 2
instead of recording a pass: an exit code is not a verdict.

**Naming rule (2026-11-04, `docs/decisions/ADR-0003-evidence-file-naming.md`).** Evidence files are
named `P<gate>-<task>-<scenario>`; `<task>` is the task whose acceptance the file answers, taken
from the row that the gate record lists it under. The ten files in this directory were renamed on
2026-11-04 under that rule (`git mv` only, no byte of evidence content changed — none of them names
a task id inside). The old names remain resolvable through the mapping table in ADR-0003; in
particular the old `T35-dry-seeded-*` trio was **P4** evidence for the T31 orchestrator loop, not
the plan's P5 task T35 (which writes `docs/INT-*` and `docs/SPEC-002.md` and produces no run
evidence). Files are not deleted, and the gitignored transcripts under `artifacts/` still contain
the old names because a transcript is not editable history.

## 6. What these runs do **not** show

1. **Open book.** `docs/T33-refinement-target-proposal.md` §Appendix A contains a complete working
   `th1`. The agent read it (20 `pi.jsonl` lines reference that file in the first attempt). These
   runs measure the
   end-to-end loop, not blind refinement synthesis. A closed-book variant has **not** been run.
2. **The orchestrator's repair path was never exercised by a real model.** The gate → feedback
   packet → next Pi call path has only DRY_RUN evidence. In every real run the agent verified and
   fixed its own model within a single turn, so `repairs_used` stayed 0.
   *Status at 2026-11-03: closed for one seeded case by §9, under a deliberately different prompt
   condition. The three main runs in §2 still have `repairs_used: 0` and are not re-labelled.*
3. **The P4 cap of 3 verifications does not bound total verification.** In the seeded run the agent
   invoked `gate` 3 times inside one turn (measured by `tool_execution_start` in `pi.jsonl`; the
   P4 Gate run: 2 times). The cap bounds the orchestrator's gates only.
4. **The agent reads and runs the verifier's own source** (`crates/intentower-cli/src/gate/*.rs`)
   and ran `gate` on other models (MODEL-001) as examples. No baseline violation occurred, but the
   allowed scope of agent access is not yet a written rule — that belongs to T34 / human review.
5. **No claim about code correctness.** `SMT_DISCHARGED` means cvc5 found no counterexample under
   the F0 encoding. It is not a kernel-checked certificate, and it says nothing about Rust code.

## 7. Reproduce

```bash
cargo build --offline
rm -f models/thermostat/th1.eventb
./scripts/run-refinement.sh                       # main run: MODEL-005 -> MODEL-006
# seeded supplementary run
cp evidence/experiments/models/th1-seed-input.eventb models/thermostat/th1.eventb
ALLOW_EXISTING_M1=1 ./scripts/run-refinement.sh
# harness regression, no LLM
DRY_RUN=1 ALLOW_EXISTING_M1=1 ./scripts/run-refinement.sh models/counter MODEL-002 models/counter/m1_badadd.eventb
```

Run directories live under `artifacts/refine/` and are gitignored; only `run.json`, the generated
models and the seed record are committed here.

## 8. P4 Gate verification run (`p4-gate/`) — run by the human, not by the agent

The human ran the gate sequence at the prompt on 2026-11-03 (UTC):

```bash
./target/debug/intentower baseline                       # BASELINE OK: 5 hashes, 3 model ids pinned, exit 0
rm -f models/thermostat/th1.eventb                       # no-op: th1 is not committed, by design
./scripts/run-refinement.sh                              # the run below
```

| run | verdict | POs | statuses | verif | repairs | pi calls | input | output | cacheRead | total tokens | peak ctx | pi s | gate s | th1 sha256 (12) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| human (`p4-gate/human-run-1.json`) | PASS | 23 | 23 SMT_DISCHARGED | 1 | 0 | 12 | 11158 | 6716 | 169492 | 187366 | 20545 | 125 | 1 | db28b21a8441 |

`intentower baseline` reported `BASELINE OK: 5 hashes, 3 model ids pinned` at all three checkpoints
(before Pi, after Pi, at the end). `escalation.md` says `next=NONE`. `unexpected_entries: []`.

**23 obligations, not 33.** This `th1` (`p4-gate/th1-human-run-1.eventb`) adds only `heater_on` /
`heater_off` as new events; `warm` and `chill` do not touch `heater`, and there is no invariant tying
`heater` to `temperature`. The refinement is legal and every generated obligation is discharged, but
it is **thinner** than the T33 refinements, which also constrained `heater` against the temperature.
A lower PO count here is a different model shape, not a looser gate.

Negative control, run in the same session: `intentower gate models/counter --model-id MODEL-001`
→ exit **2**, `INCOMPLETE`, `1 obligation(s) UNSUPPORTED (not decided): m0::sub/act1/FIS`,
19 SMT_DISCHARGED + 1 UNSUPPORTED of 20 (`p4-gate/negative-control-MODEL-001.json`). The known
permanent gap still fails, so this PASS did not come from weakening the verifier.

What this run does **not** change: `repairs_used: 0`, so the orchestrator's
gate → feedback packet → next Pi call path is still unevidenced by a real model (§6.2), and the
open-book condition (§6.1) applies to this run as well.

`p4-gate/manifest.json` fingerprints every archived file, records who ran which command, and
fingerprints the 2 MB `pi.jsonl` transcript that is **not** committed (it stays in the gitignored
run directory). Because `models/thermostat/th1.eventb` was deleted after archiving, the archived
gate report no longer traces to the working tree — measured, not assumed:
`intentower escalate` on the archived report now returns `next=TOOLCHAIN_FIRST` with a `STALE`
warning (`p4-gate/human-run-1-escalation-after-cleanup.md`). A stored PASS cannot be reused as a
current verdict; re-running `gate` is the only way to get one.

## 9. Repair-loop closure attempt (`repair/`) — a different condition, on purpose

§6.2 was the only P4 limitation that one bounded experiment could close, so one was run at the
human's instruction ("跑有界尝试"). Two knobs were added to `scripts/run-refinement.sh` for it, and
both are recorded in `run.json` so the run can never be mistaken for a §2 main run:

- `SEED_M1=<file>` copies an imperfect M1 into the refinement file and **runs no LLM call in round
  1**, so the orchestrator's own `gate` sees the failure first. Without this the agent always fixed
  the model before the orchestrator ever looked, which is why `repairs_used` was 0 everywhere.
- `NO_SELFCHECK=1` makes the prompt forbid the agent from running `gate` (explicitly overriding the
  "修完必须重跑" instruction in `prompts/repair.md` and in the packet), so the packet is its only
  source of information about the failure.

Seed: `models/th1-seed-input.eventb` (sha256 `5ae432f7…`, the probe-v1 shape with 2 refutable
guards). Caps unchanged: 3 verifications, 2 repairs.

| run | verifications | repairs | round 1 | round 2 | stop |
|---|---|---|---|---|---|
| `repair/seedfix-1-run.json` | 2 | 1 | `FAIL` 33 (31 discharged, 2 `NOT_DISCHARGED`), no LLM call | `PASS` 33, 5 LLM calls, 49,722 tokens, 69 s | `pass` |

The circuit that had never run with a real model now has evidence:
`gate` (FAIL) → `feedback` packet (2,850 bytes, `seedfix-1-round1-packet.md`) → second Pi call
(prompt = `prompts/repair.md` + that packet, 8,871 bytes, `seedfix-1-round2-prompt.md`) → `gate`
(PASS). `intentower baseline` was OK before the run, after each Pi call and at the end; the agent
touched only `models/thermostat/th1.eventb`; `unexpected_entries` is empty.

Condition held, measured from `round-2/pi.jsonl` by `tool_execution_start`: 5 tool executions
(3 bash, 1 edit, 1 write), **0** running `intentower gate` or `verify`, **0** touching
`artifacts/refine/`. The fix was minimal and not the reference fix: it kept the original guards and
added `@grd3 temperature <= 69` / `@grd3 temperature >= 31`
(`repair/th1-seedfix-1-fixed.eventb`, sha256 `1033ea8d…`).

**First attempt under this condition was invalid and is kept**
(`repair/seedfix-1a-flawed-condition-run.json`): the prompt branch for `NO_SELFCHECK=1` omitted the
rule against reading `artifacts/refine/`, and the agent went and read the orchestrator's own
`report.json` and `pog/` output directly. It also produced a PASS with `repairs_used: 1`, but the
"packet is the only source" condition did not hold, so it is **not** counted as the closure
evidence. The rule is now emitted in both prompt branches.

What §9 does **not** show: one repaired case is not a repair success rate; the agent still had the
reference `th1` in `docs/` available to read; `escalate` still only records the outcome; and the
§2 main runs remain single-round passes with no repair. Reproduce:

```bash
SEED_M1=evidence/experiments/models/th1-seed-input.eventb NO_SELFCHECK=1 ./scripts/run-refinement.sh
# deterministic version, no LLM: adds DRY_RUN=1 and stops at verify_cap_exhausted
# (harness/P4-T31-dry-seeded-run.json, harness/P4-T31-dry-seeded-packet.md)
```
