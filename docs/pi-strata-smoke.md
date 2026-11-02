# T08 — Pi + Strata 冒烟记录

- **Date**: 2026-11-02 · **Executed by**: Pi（本会话即由 Strata 驱动）
- **Plan reference**: §3 P1/T08、§5.1（角色分层：Pi 是 harness，Strata 是模型服务）
- **范围说明**：`strata connect` 一步**未执行**（见 §5），其余检查点全部实测。人类已决定不把 Strata 引擎加入 PATH（由 systemd 管理）。

---

## 1. Pi 与 provider 配置（实测）

| 项 | 实测值 | 来源 |
|---|---|---|
| Pi 版本 | `1.1.0` | `pi --version` |
| 默认 provider / model | `strata` / `qwen3.8-flash-next-iq3_s` | `~/.pi/agent/settings.json` |
| 默认思考等级 | `high` | 同上 |
| provider 定义 | `baseUrl=http://127.0.0.1:8080/v1`，`api=openai-completions`，`models=['qwen3.8-flash-next-iq3_s']`，`apiKey` 长度 12（值未打印） | `~/.pi/agent/models.json` |
| 当前会话环境 | `PI_CODING_AGENT=true`、`PI_PROVIDER=strata`、`PI_MODEL=qwen3.8-flash-next-iq3_s`、`PI_REASONING_LEVEL=high`、`PI_SESSION_ID=01a19a7a-7c49-7446-9ae6-b6c13bd4dedb` | `env` |
| 会话记录文件 | `~/.pi/agent/sessions/--home-mye-src-llm-intentower--/2026-11-02T04-18-00-906Z_01a19a7a-….jsonl` | `PI_SESSION_FILE` |

## 2. Strata 服务（systemd 管理）

| 项 | 实测值 |
|---|---|
| 单元 | `strata-server.service` — `loaded active running`（`systemctl --user list-units --type=service`） |
| 单元命令 | `/usr/bin/bash /home/mye/tool/Strata/run-iq3_s.sh` |
| HTTP 包装进程 | `/home/mye/tool/Strata/.venv/bin/python /home/mye/tool/Strata/serve/server.py --engine strata --config /home/mye/tool/Strata/strata-iq3_s.json --port 8080` |
| 推理引擎进程 | `/home/mye/tool/Strata/engine/strata --serve --pack .../packs/iq3_s --native .../Qwen3.8-Flash-Next-GSQ-RCO-IQ3_S-00001-of-00002.gguf --ple-gguf ...00002-of-00002.gguf --expert-profile ... --spec 4 --spec-min-p 0.5 --mtp .../rt --max-context 131072 --kv int8 --kv-resident 32768` |
| 引擎可执行文件 | `/home/mye/tool/Strata/engine/strata`（63,311,816 字节，**不在 PATH，按人类决定不加软链**） |
| `GET /v1/models` | `HTTP=200`，`time=0.000983s`，`{"id":"qwen3.8-flash-next-iq3_s","status":{"value":"loaded"},"meta":{"n_ctx":131072}}` |

## 3. 只读工具调用（Pi 驱动，成功）

- **调用**：Pi 的 `read` 工具读取 `project.json`（T06 产物）。
- **结果**：成功返回 14 行 JSON，内容与仓库文件一致（`schema_version: "0.1"`、5 个顶层字段）。
- **意义**：该调用由 `PI_PROVIDER=strata` 的会话发起并完成 → **Strata 可驱动 Pi 的只读工具调用：已证实**。

## 4. 直接补全冒烟（独立于 Pi，验证端点可推理）

`POST http://127.0.0.1:8080/v1/chat/completions`，prompt = `Reply with exactly: PI_OK`，`temperature=0`，密钥从 `models.json` 读取且不打印。

| `max_tokens` | HTTP | 耗时 | `finish_reason` | `content` | usage |
|---|---|---|---|---|---|
| 8 | 200 | 0.96s | `length` | `None` | 58 prompt / 8 completion |
| 64 | 200 | 0.50s | `stop` | `PI_OK` | 58 / 31（`cached_tokens=53`） |
| 256 | 200 | 0.44s | `stop` | `PI_OK` | 58 / 31（`cached_tokens=53`） |

响应 `message` 的键为 `['content', 'reasoning_content', 'role']`。

**对 P4 有直接影响的两个发现**：

1. 该模型把思考写入 `reasoning_content`，`content` 才是最终答复。`max_tokens=8` 时思考未结束就被截断，`content` 为 `None` 且 `finish_reason=length`。**给 Agent 的 token 预算必须把推理开销算进去**，否则会拿到空答复却误判为"成功返回"。
2. 推理引擎 `--max-context 131072`、`--kv int8`、`--spec 4`（投机解码）、`--kv-resident 32768`；小请求 P50 约 0.5s，且出现 `cached_tokens=53` 的前缀缓存命中。计划 §4.3 的耗时阈值应以真实 PO 验证为准，不能拿这个数当模型推理上限。

## 5. 未执行项（`not_run`）

| 检查 | 状态 | 原因 |
|---|---|---|
| `strata connect` 查看官方连接方式 | **not_run** | `strata` 不在 PATH（`command -v strata` → 空）；引擎由 user-level systemd 服务管理，人类明确决定不加软链。替代证据 = §2 的单元状态与进程命令行 |
| Strata 上游文档核对连接方式 | not_run | GitHub HTML / raw 不可达（见 `docs/tooling-versions.md` §4），需走 codeload 或 api.github.com |

## 6. 结论

- **Strata 驱动 Pi：已证实**（会话环境 + 只读工具调用成功）。
- **端点可推理：已证实**（三次补全，`content` 正确返回 `PI_OK`）。
- **Pi 与 Strata 是两层**：Pi 1.1.0 是 harness，Strata 是 `openai-completions` 服务；计划 §1 的分层成立。
- 第一阶段按 §1.1 只用 Pi 的基础 shell 工具，**不做 Pi Extension** —— 本次未触碰该边界。

## 7. 风险与待办

- `reasoning_content` 语义由本仓库自行约定：P4 的 `prompts/refine.md` 与 `scripts/run-refinement.sh` 必须显式区分"思考"与"结论"，并把 `finish_reason=length` 当作失败信号而非空答复。
- 上下文上限 131072 token 是硬约束；计划 §2.4 的"一次反馈不超过指定上下文预算"需在 T30 里给出具体数值。
- 未测：长上下文下的吞吐与 P95、并发调用、`strata` 引擎重启对会话的影响。
