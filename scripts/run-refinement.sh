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
#   PI_TIMEOUT / GATE_TIMEOUT   seconds; every external tool gets a timeout
#
set -uo pipefail

MODEL_DIR=${1:-models/thermostat}
MODEL_ID=${2:-MODEL-006}
M1_FILE=${3:-models/thermostat/th1.eventb}

MAX_VERIFY=3
MAX_REPAIR=2
FEEDBACK_BUDGET=8192
PI_TIMEOUT=${PI_TIMEOUT:-900}
GATE_TIMEOUT=${GATE_TIMEOUT:-600}
BIN=${INTENTOWER_BIN:-./target/debug/intentower}
RUN_DIR=artifacts/refine/run-$(date -u +%Y%m%dT%H%M%SZ)-$$  # PID keeps two runs in the same second apart

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

mkdir -p "$RUN_DIR" || exit 2
echo "run dir: $RUN_DIR"

# ---- preflight: tools present, model pair named, M1 not pre-existing ----
for tool in "$BIN" rossi cvc5 pi; do
  command -v "$tool" >/dev/null 2>&1 || [ -x "$tool" ] || stop 2 "missing tool: $tool"
done
[ -f "$MODEL_DIR/models.json" ] || stop 2 "no models.json in $MODEL_DIR"
grep -q "\"$MODEL_ID\"" "$MODEL_DIR/models.json" || stop 2 "$MODEL_ID is not listed in $MODEL_DIR/models.json"
if [ -f "$M1_FILE" ] && [ "${ALLOW_EXISTING_M1:-0}" != 1 ]; then
  stop 2 "$M1_FILE already exists; move it out of the way so the run measures a real refinement (ALLOW_EXISTING_M1=1 overrides for harness tests)"
fi

# ---- baseline before the run ----
"$BIN" baseline || stop 2 "baseline changed before the run"

verify=0
repairs=0
gate_exit=2
stop_reason=verify_cap_exhausted

while [ "$verify" -lt "$MAX_VERIFY" ]; do
  verify=$((verify + 1))
  round=$RUN_DIR/round-$verify
  mkdir -p "$round"

  # ---- the LLM step ----
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
  prompt+="验证命令: $BIN gate $MODEL_DIR --model-id $MODEL_ID --out $round/report.json"$'\n'
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
    timeout "$PI_TIMEOUT" pi --print --no-session --mode json -- "$prompt" \
      > "$round/pi.jsonl" 2> "$round/pi.err"
    pi_exit=$?
    echo "$pi_exit" > "$round/pi.exit"
    echo $(( $(date +%s) - start )) > "$round/pi.seconds"
    echo "round $verify: pi exit $pi_exit in $(cat "$round/pi.seconds")s, log $round/pi.jsonl"
  fi

  # ---- protected inputs after the LLM touched the tree ----
  "$BIN" baseline || stop 2 "baseline changed after round $verify (the agent moved a protected input)"

  # ---- the only thing that produces new evidence ----
  start=$(date +%s)
  timeout "$GATE_TIMEOUT" "$BIN" gate "$MODEL_DIR" --model-id "$MODEL_ID" --out "$round/report.json" \
    > "$round/gate.log" 2>&1
  gate_exit=$?
  gate_seconds=$(( $(date +%s) - start ))
  echo "$gate_seconds" > "$round/gate.seconds"
  echo "round $verify: gate exit $gate_exit in ${gate_seconds}s ($(grep -oE 'PASS|FAIL|INCOMPLETE' "$round/gate.log" | head -1))"

  if [ "$gate_exit" -eq 0 ]; then
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

# ---- run.json: every round, every LLM call, every gate verdict ----
python3 - "$RUN_DIR" "$MODEL_DIR" "$MODEL_ID" "$M1_FILE" "$verify" "$repairs" "$gate_exit" "$stop_reason" <<'PY' || stop 2 "could not write run.json"
import json, pathlib, re, sys

run_dir, model_dir, model_id, m1, verify, repairs, gate_exit, reason = sys.argv[1:9]
root = pathlib.Path(run_dir)
rounds = []
for d in sorted(root.glob("round-*")):
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
    rounds.append({
        "round": int(d.name.split("-")[1]),
        "llm": pi,
        "prompt_bytes": (d / "prompt.md").stat().st_size if (d / "prompt.md").is_file() else None,
        "gate": gate,
        "packet_bytes": packet.stat().st_size if packet.is_file() else None,
    })
summary = {
    "model_dir": model_dir,
    "model_id": model_id,
    "refinement_file": m1,
    "caps": {"max_verifications": 3, "max_repairs": 2, "feedback_budget_bytes": 8192},
    "verifications_used": int(verify),
    "repairs_used": int(repairs),
    "gate_exit": int(gate_exit),
    "stop_reason": reason,
    "dry_run": not any(r["llm"]["calls"] for r in rounds),
    "llm_models_seen": sorted({m for r in rounds for m in r["llm"]["models"]}),
    "rounds": rounds,
}
(root / "run.json").write_text(json.dumps(summary, indent=2) + "\n")
print(f"run.json: {verify} verifications, {repairs} repairs, stop={reason}")
PY

echo "final gate exit: $gate_exit ($stop_reason)"
[ "$gate_exit" -eq 0 ] && exit 0
exit 2
