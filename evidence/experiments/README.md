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

`harness/dry-cap-exhausted.json`: `DRY_RUN=1` against MODEL-002 (a known-bad refinement) —
3 verifications, 2 repairs, `stop_reason: verify_cap_exhausted`, gate `FAIL` (exit 1) in all three
rounds, script exit 2, repair packets 3645 / 3645 bytes under the 8 KB budget.
`harness/dry-pass.json`: `DRY_RUN=1` against MODEL-004 — 1 verification, `stop_reason: pass`, exit 0.

After T34 the same two runs also write `escalation.md`. The post-T34 copies are kept as
`harness/T34-dry-cap-exhausted-run.json` + `T34-dry-cap-exhausted-escalation.md`
(`next=EXPERT_OR_SHRINK`, because one obligation is `UNSUPPORTED`) and
`harness/T34-dry-pass-run.json` + `T34-dry-pass-escalation.md` (`next=NONE`, `DRY_RUN=1` against
MODEL-006 with a pre-existing `th1`). The escalation record is decided only by the gate report and
the orchestrator counters: an agent's own claim of having fixed the model is not an input, which
`tests/feedback.rs::the_next_step_is_read_from_the_report_not_from_what_the_agent_claims` asserts by
comparing the output byte for byte with and without the claim.

`harness/T34-gate-exit0-no-report-stop.json` is a negative control run through a wrapper binary
(`INTENTOWER_BIN`) that lets `gate` exit 0 while writing no `report.json`. The loop stops with exit 2
instead of recording a pass: an exit code is not a verdict.

## 6. What these runs do **not** show

1. **Open book.** `docs/T33-refinement-target-proposal.md` §Appendix A contains a complete working
   `th1`. The agent read it (20 `pi.jsonl` lines reference that file in the first attempt). These
   runs measure the
   end-to-end loop, not blind refinement synthesis. A closed-book variant has **not** been run.
2. **The orchestrator's repair path was never exercised by a real model.** The gate → feedback
   packet → next Pi call path has only DRY_RUN evidence. In every real run the agent verified and
   fixed its own model within a single turn, so `repairs_used` stayed 0.
3. **The P4 cap of 3 verifications does not bound total verification.** In the seeded run the agent
   invoked `gate` about ten times inside one turn. The cap bounds the orchestrator's gates only.
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
