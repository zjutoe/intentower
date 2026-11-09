#!/usr/bin/env bash
# T31 — one automatic-refinement run: Pi -> gate -> feedback -> Pi.
#
# Not a framework: a single manual entry point that enforces the P4 limits
# (docs/rossi-smoke.md §27, AGENTS.md §6):
#   * at most 3 candidate verifications,
#   * at most 2 automatic repairs,
#   * `intentower baseline` before the run, after every Pi call, and after the run.
#
# Every round keeps its evidence under artifacts/refine/run-<UTC>/ and the run
# ends with run.json: per-round gate verdicts plus the provider/model/tokens of
# every LLM call. The model identity is recorded because docs/rossi-smoke.md §30
# showed a signing round whose recorded model was not the one that served it.
#
# Usage: scripts/run-refinement.sh [MODEL_DIR] [MODEL_ID] [M1_FILE]
#        defaults: models/thermostat MODEL-006 models/thermostat/th1.eventb
#
# Environment knobs (all optional):
#   DRY_RUN=1            skip the LLM call; exercises the loop, caps and guards only
#   ALLOW_EXISTING_M1=1  let a run start when the refinement file already exists
#   INTENTOWER_BIN=...   path to the intentower binary
#   PI_TIMEOUT / GATE_TIMEOUT   positive whole seconds; TERM then KILL after 1s
#   SEED_M1=<file>       start from a deliberately imperfect M1: it is copied to the
#                        refinement file and round 1 is verified without any LLM call,
#                        so the only way to a fix is gate -> packet -> Pi.
#   NO_SELFCHECK=1       the prompt forbids the agent from running gate itself, so the
#                        orchestrator's repair packet is its only source of information.
#   Both knobs are recorded in run.json (seeded_from, prompt_condition): a run made under
#   them is a different condition and must never be merged into the main results table.
#
set -uo pipefail

MODEL_DIR=${1:-models/thermostat}
MODEL_ID=${2:-MODEL-006}
M1_FILE=${3:-models/thermostat/th1.eventb}

MAX_VERIFY=3
MAX_REPAIR=2
FEEDBACK_BUDGET=8192
KILL_GRACE=1s
PI_TIMEOUT=${PI_TIMEOUT-900}
GATE_TIMEOUT=${GATE_TIMEOUT-600}
BIN=${INTENTOWER_BIN:-./target/debug/intentower}
RUN_DIR=artifacts/refine/run-$(date -u +%Y%m%dT%H%M%SZ)-$$  # PID keeps two runs in the same second apart
# The agent's own gate self-checks go here, never into RUN_DIR: in the first run 2
# the agent wrote round-2/report.json and round-3/ inside the evidence directory.
SCRATCH=${SCRATCH:-/tmp/intentower-refine-scratch/$(basename "$RUN_DIR")}

stop() { # stop <exit> <reason>
  echo "STOP: $2" >&2
  python3 - "$RUN_DIR" "$2" "$1" <<'PY'
import json, sys, pathlib
run_dir, reason, code = sys.argv[1], sys.argv[2], int(sys.argv[3])
path = pathlib.Path(run_dir)
if path.is_dir():
    (path / "stop.json").write_text(json.dumps({"stop_reason": reason, "exit": code}, indent=2) + "\n")
PY
  exit "$1"
}

candidate_binding() { # candidate_binding [successful gate report]
  python3 - "$MODEL_DIR" "$MODEL_ID" "$M1_FILE" "${1:-}" <<'PY'
import hashlib, json, pathlib, sys

model_dir, model_id, candidate, report_path = sys.argv[1:]
try:
    directory = pathlib.Path(model_dir)
    manifest = json.loads((directory / "models.json").read_text())
    if not isinstance(manifest, dict) or model_id not in manifest:
        raise ValueError(f"{model_id} is not listed in {directory / 'models.json'}")
    files = manifest[model_id]
    if not isinstance(files, list) or not files or not all(isinstance(p, str) and p for p in files):
        raise ValueError(f"{model_id} must name a nonempty array of source paths")
    sources = sorted(str((directory / path).resolve()) for path in files)
    if len(set(sources)) != len(sources):
        raise ValueError(f"{model_id} lists a source more than once")
    candidate = str(pathlib.Path(candidate).resolve())
    if candidate not in sources:
        raise ValueError(f"candidate {candidate} is not a source of {model_id}")
    if report_path:
        report = json.loads(pathlib.Path(report_path).read_text())
        if report.get("model_id") != model_id or report.get("gate", {}).get("status") != "PASS":
            raise ValueError("successful gate report does not describe this model's PASS")
        rows = [row for row in report["inputs"] if row.get("role") == "model_source"]
        paths = [str(pathlib.Path(row["path"]).resolve()) for row in rows]
        if sorted(paths) != sources:
            raise ValueError("gate report source set does not match this round's model sources")
        for path, row in zip(paths, rows):
            if hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest() != row["sha256"]:
                raise ValueError(f"gate report source hash does not match current bytes: {path}")
except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
    print(f"candidate binding: {error}", file=sys.stderr)
    sys.exit(2)
print(json.dumps(sources))
PY
}

python3 - "$PI_TIMEOUT" "$GATE_TIMEOUT" <<'PY' || stop 2 "invalid timeout configuration"
import sys
for name, value in zip(("PI_TIMEOUT", "GATE_TIMEOUT"), sys.argv[1:]):
    try:
        valid = value.isascii() and value.isdecimal() and int(value) > 0
    except ValueError:
        valid = False
    if not valid:
        print(f"{name} must be positive whole seconds: {value!r}", file=sys.stderr)
        sys.exit(2)
PY

mkdir -p "$RUN_DIR" "$SCRATCH" || exit 2
echo "run dir: $RUN_DIR"

# ---- preflight: tools present, model pair named, M1 not pre-existing ----
for tool in "$BIN" rossi cvc5 pi timeout; do
  command -v "$tool" >/dev/null 2>&1 || [ -x "$tool" ] || stop 2 "missing tool: $tool"
done
MODEL_SOURCES=$(candidate_binding) || stop 2 "candidate is not bound to the selected model"
if [ -f "$M1_FILE" ] && [ "${ALLOW_EXISTING_M1:-0}" != 1 ]; then
  stop 2 "$M1_FILE already exists; move it out of the way so the run measures a real refinement (ALLOW_EXISTING_M1=1 overrides for harness tests)"
fi
seed_sha=""
if [ -n "${SEED_M1:-}" ]; then
  [ -f "$SEED_M1" ] || stop 2 "SEED_M1 points at a file that does not exist: $SEED_M1"
  seed_sha=$(sha256sum "$SEED_M1" | cut -d' ' -f1)
fi

# ---- baseline before the run ----
"$BIN" baseline || stop 2 "baseline changed before the run"

if [ -n "${SEED_M1:-}" ]; then
  cp "$SEED_M1" "$M1_FILE" || stop 2 "could not seed $M1_FILE from $SEED_M1"
  echo "seeded $M1_FILE from $SEED_M1 (sha256 $seed_sha); round 1 runs no LLM call"
fi

verify=0
repairs=0
gate_exit=2
stop_reason=verify_cap_exhausted

while [ "$verify" -lt "$MAX_VERIFY" ]; do
  verify=$((verify + 1))
  round=$RUN_DIR/round-$verify
  mkdir -p "$round"

  # ---- the LLM step ----
  if [ "$verify" -eq 1 ] && [ -n "${SEED_M1:-}" ]; then
    # the seeded model is verified as it stands: no LLM gets a chance to rewrite it
    # first, which is what hid the repair circuit in the earlier runs
    echo "round $verify: seeded model, no LLM call"
    : > "$round/pi.jsonl"
    echo 0 > "$round/pi.exit"
    echo 0 > "$round/pi.seconds"
  else
  if [ "$verify" -eq 1 ]; then
    prompt=$(cat prompts/refine.md)
  else
    prompt=$(cat prompts/repair.md)
    prompt+=$'\n\n## 本轮反馈包（唯一输入）\n\n'
    prompt+=$(cat "$RUN_DIR/round-$((verify - 1))/packet.md")
  fi
  prompt+=$'\n\n## 本轮参数（上文 prompt 里的 models/counter、MODEL-001 只是示例，一律以下述为准）\n\n'
  prompt+="模型目录: $MODEL_DIR"$'\n'
  prompt+="model id: $MODEL_ID"$'\n'
  prompt+="唯一可写模型文件: $M1_FILE"$'\n'
  if [ "${NO_SELFCHECK:-0}" = 1 ]; then
    prompt+="本轮禁止运行 gate 或任何验证命令（包括上文和反馈包里“修完必须重跑”的指令）：编排器会在你之后自己跑 gate。你对模型状态的唯一信息来源是上面的反馈包。"$'\n'
  else
    prompt+="自查命令: $BIN gate $MODEL_DIR --model-id $MODEL_ID --out $SCRATCH/round-$verify.json"$'\n'
  fi
  prompt+="自查报告只准写到上面 $SCRATCH 路径；不得读写 artifacts/refine/ 下的任何文件（包括编排器的 report.json、packet.md、run.json），那是编排器的证据目录。"$'\n'
  prompt+="这是第 $verify / $MAX_VERIFY 次候选验证，已用 $repairs / $MAX_REPAIR 次自动修复。"$'\n'
  prompt+="只允许新建或修改这一个模型文件：$M1_FILE。上文提到的其它模型文件一律只读。"$'\n'
  printf '%s' "$prompt" > "$round/prompt.md"

  if [ "${DRY_RUN:-0}" = 1 ]; then
    echo "round $verify: DRY_RUN — no LLM call, model unchanged"
    : > "$round/pi.jsonl"
    echo 0 > "$round/pi.exit"
    echo 0 > "$round/pi.seconds"
  else
    start=$(date +%s)
    timeout --kill-after="$KILL_GRACE" "$PI_TIMEOUT" pi --print --no-session --mode json -- "$prompt" \
      > "$round/pi.jsonl" 2> "$round/pi.err"
    pi_exit=$?
    echo "$pi_exit" > "$round/pi.exit"
    echo $(( $(date +%s) - start )) > "$round/pi.seconds"
    echo "round $verify: pi exit $pi_exit in $(cat "$round/pi.seconds")s, log $round/pi.jsonl"
  fi
  fi

  # ---- protected inputs after the LLM touched the tree ----
  "$BIN" baseline || stop 2 "baseline changed after round $verify (the agent moved a protected input)"
  current_sources=$(candidate_binding) || stop 2 "candidate binding changed after round $verify"
  [ "$current_sources" = "$MODEL_SOURCES" ] || stop 2 "selected model sources changed after round $verify"

  # ---- the only thing that produces new evidence ----
  start=$(date +%s)
  timeout --kill-after="$KILL_GRACE" "$GATE_TIMEOUT" "$BIN" gate "$MODEL_DIR" --model-id "$MODEL_ID" --out "$round/report.json" \
    > "$round/gate.log" 2>&1
  gate_exit=$?
  echo "$gate_exit" > "$round/gate.exit"
  gate_seconds=$(( $(date +%s) - start ))
  echo "$gate_seconds" > "$round/gate.seconds"
  echo "round $verify: gate exit $gate_exit in ${gate_seconds}s ($(grep -oE 'PASS|FAIL|INCOMPLETE' "$round/gate.log" | head -1))"

  # a verdict only exists if the report exists; an exit code alone never passes
  if [ ! -f "$round/report.json" ]; then
    stop 2 "gate exited $gate_exit but wrote no report.json for round $verify; there is no verdict to act on"
  fi

  # keep what the agent verified for itself, labelled as the agent's own run
  for f in "$SCRATCH"/round-"$verify"*.json; do
    [ -f "$f" ] && cp "$f" "$round/agent-selfcheck-$(basename "$f")"
  done

  if [ "$gate_exit" -eq 0 ]; then
    current_sources=$(candidate_binding "$round/report.json") || stop 2 "gate PASS is not bound to this round's sources"
    [ "$current_sources" = "$MODEL_SOURCES" ] || stop 2 "selected model sources changed during round $verify"
    stop_reason=pass
    break
  fi
  if [ "$verify" -ge "$MAX_VERIFY" ]; then
    stop_reason=verify_cap_exhausted
    break
  fi

  # ---- one repair packet per failed round; over budget is an error, not a truncation ----
  "$BIN" feedback "$round/report.json" --out "$round/packet.md" --max-bytes "$FEEDBACK_BUDGET" \
    > "$round/feedback.log" 2>&1
  fb_exit=$?
  if [ "$fb_exit" -ne 0 ]; then
    stop 2 "round $verify produced no repair packet (exit $fb_exit); see $round/feedback.log"
  fi
  repairs=$((repairs + 1))
  echo "round $verify: repair packet $(wc -c < "$round/packet.md") bytes ($repairs / $MAX_REPAIR)"
done

# ---- baseline after the run ----
"$BIN" baseline || stop 2 "baseline changed at the end of the run"

# ---- T34: the next step, decided by the gate report and the counters ----
# What the agent said about its own work is not an input to this step; only a
# re-run of gate can change a row of the table.
last_report=$(ls -1 "$RUN_DIR"/round-*/report.json 2>/dev/null | tail -1 || true)
if [ -n "$last_report" ] && "$BIN" escalate "$last_report" --stop-reason "${stop_reason:-not_stopped}" \
    --verifications "$verify" --repairs "$repairs" --out "$RUN_DIR/escalation.md"; then
  :
else
  {
    echo "# 下一步（升级路径）"
    echo
    if [ -n "$last_report" ]; then
      echo "最后一轮的 gate 报告（$last_report）无法读取或校验失败：判定不可信，自动流程不得继续。"
    else
      echo "本轮没有任何 gate 报告可依据：判定能力本身缺失，自动流程不得继续。"
    fi
    echo
    echo "**下一步：\`TOOLCHAIN_FIRST\`**"
  } > "$RUN_DIR/escalation.md"
fi
cat "$RUN_DIR/escalation.md"

# ---- run.json: every round, every LLM call, every gate verdict ----
python3 - "$RUN_DIR" "$MODEL_DIR" "$MODEL_ID" "$M1_FILE" "$verify" "$repairs" "$gate_exit" "$stop_reason" "${SEED_M1:-}" "${seed_sha:-}" "${NO_SELFCHECK:-0}" "${DRY_RUN:-0}" <<'PY' || stop 2 "could not write run.json"
import json, pathlib, re, sys

run_dir, model_dir, model_id, m1, verify, repairs, gate_exit, reason, seed, seed_sha, noself, dry_run = sys.argv[1:13]
root = pathlib.Path(run_dir)
rounds = []
for d in sorted(root.glob("round-*")):
    if not (d / "pi.jsonl").is_file():
        continue  # a directory the agent created, not a round the orchestrator ran
    pi = {"calls": 0, "models": [], "input_tokens": 0, "output_tokens": 0,
          "reasoning_tokens": 0, "cache_read_tokens": 0, "cache_write_tokens": 0,
          "total_tokens": 0, "max_context_tokens": 0}
    for line in (d / "pi.jsonl").read_text().splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("type") != "message_end":
            continue
        message = event.get("message", {})
        if message.get("role") != "assistant":
            continue
        usage = message.get("usage", {})
        pi["calls"] += 1
        pi["models"].append(f"{message.get('provider')}/{message.get('model')}")
        for key, name in (("input", "input_tokens"), ("output", "output_tokens"),
                         ("reasoning", "reasoning_tokens"), ("cacheRead", "cache_read_tokens"),
                         ("cacheWrite", "cache_write_tokens"), ("totalTokens", "total_tokens")):
            pi[name] += usage.get(key, 0) or 0
        # totalTokens already counts cached prefix reads, so context is the sum of the input sides.
        context = sum(usage.get(key, 0) or 0 for key in ("input", "cacheRead", "cacheWrite"))
        pi["max_context_tokens"] = max(pi["max_context_tokens"], context)
    for name, key in (("pi.exit", "pi_exit"), ("pi.seconds", "pi_seconds"), ("gate.seconds", "gate_seconds")):
        if (d / name).is_file():
            pi[key] = int((d / name).read_text().strip() or 0)
    report_path = d / "report.json"
    gate = {}
    if report_path.is_file():
        report = json.loads(report_path.read_text())
        counts = {}
        for obligation in report["obligations"]:
            counts[obligation["status"]] = counts.get(obligation["status"], 0) + 1
        gate = {
            "verdict": report["gate"]["status"],
            "po_total": report["po_total"],
            "status_counts": counts,
            "tools": report["tools"],
            "reasons": report["gate"]["reasons"],
            "report": str(report_path),
        }
    packet = d / "packet.md"
    selfcheck = sorted(p.name for p in d.glob("agent-selfcheck-*"))
    rounds.append({
        "round": int(d.name.split("-")[1]),
        "llm": pi,
        "prompt_bytes": (d / "prompt.md").stat().st_size if (d / "prompt.md").is_file() else None,
        "gate": gate,
        "packet_bytes": packet.stat().st_size if packet.is_file() else None,
        "agent_selfcheck_reports": selfcheck,
    })
# anything in the evidence directory that the orchestrator did not create is reported, not deleted
expected = {"run.json", "stop.json", "escalation.md"}
own = {"prompt.md", "pi.jsonl", "pi.err", "pi.exit", "pi.seconds", "gate.log",
       "gate.exit", "gate.seconds", "report.json", "packet.md", "feedback.log"}
for d in root.glob("round-*"):
    if (d / "pi.jsonl").is_file():
        expected.add(str(d.relative_to(root)))
        for p in d.rglob("*"):
            rel = str(p.relative_to(root))
            if p.name in own or "pog" in p.parts or p.name.startswith("agent-selfcheck-"):
                expected.add(rel)
unexpected = sorted(str(p.relative_to(root)) for p in root.rglob("*")
                    if str(p.relative_to(root)) not in expected)
summary = {
    "model_dir": model_dir,
    "model_id": model_id,
    "refinement_file": m1,
    "caps": {"max_verifications": 3, "max_repairs": 2, "feedback_budget_bytes": 8192},
    "verifications_used": int(verify),
    "repairs_used": int(repairs),
    "gate_exit": int(gate_exit),
    "stop_reason": reason,
    # Missing completion events do not establish that Pi was skipped.
    "dry_run": dry_run == "1",
    "seeded_from": {"path": seed, "sha256": seed_sha} if seed else None,
    "prompt_condition": "no_selfcheck" if noself == "1" else "selfcheck_allowed",
    "llm_models_seen": sorted({m for r in rounds for m in r["llm"]["models"]}),
    "rounds": rounds,
    "unexpected_entries": unexpected,
}
(root / "run.json").write_text(json.dumps(summary, indent=2) + "\n")
print(f"run.json: {verify} verifications, {repairs} repairs, stop={reason}")
PY

echo "final gate exit: $gate_exit ($stop_reason)"
[ "$gate_exit" -eq 0 ] && exit 0
exit 2
