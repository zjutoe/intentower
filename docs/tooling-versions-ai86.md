# 环境复刻 — ai86（从 ai89 rsync 后的依赖实测与安装记录）

- **Date**: 2026-11-03 · **Host**: `ai86` / CentOS Stream 9（`Linux 5.14.0-721.el9.x86_64`，`x86_64`）
- **Scope**: 仅记录把 ai89 的 Intentower 工作树 rsync 到 ai86 后，为跑通现有测试与 `verify`/`gate` 所需的环境检查与安装动作。
- **Not a task**: 本次未执行任何 T* 任务，未改动任何受保护文件、模型、夹具或 `approvals/baseline.json`。
- **Rule applied**: 全部版本为实测输出；缺项标 `missing`；未运行的检查标 `not_run`。
- **Authoritative baseline**: 本机记录**不替代** `docs/tooling-versions.md`（ai89，T07）。两份并列，版本必须一致才可跨机比对。

---

## 1. 检查时的状态（安装动作之前）

| 工具 | 状态 | 实测 | 说明 |
|---|---|---|---|
| `rustc` / `cargo` | found（过旧） | `rustc 1.86.0 (05f9846f8 2025-03-31)`、`cargo 1.86.0` | 低于依赖 MSRV，见 §2 |
| `rustup` | found | `1.29.0`（更新后 `1.29.2`） | `~/.cargo`、`~/.rustup` |
| crates 依赖 | **missing** | `cargo build --offline` → `error: no matching package named 'rossi' found` | 本地 registry 缓存 71 个 crate，不含 `rossi`/`rossi-prove` |
| `rossi` CLI | **missing** | `which rossi` → 空 | 见 §3.2 |
| `cvc5` | **missing** | `which cvc5` → 空 | 见 §3.3 |
| `python3` / `jsonschema` | found | `Python 3.13.9`（`/opt/anaconda3/bin/python3`）、`jsonschema 4.25.0` | `scripts/validate-report.py` 与 `cargo test --test report_schema` 需要 |
| `pi` | found | `1.5.0`（`~/.nvm/versions/node/v24.8.0/bin/pi`） | 当前会话即由它驱动 |
| Strata 引擎（本机） | missing | `which strata` → 空；`~/tool` 下无 Strata | 不需要：走 ai90 隧道，见 §5 |
| `rodin` / `prob` / `z3` | missing | `which` 均为空 | Phase 1 不需要；`java` 仅 `1.8.0_362`，若将来跑 Rodin 需升级 |
| `git` / `make` / `cmake` | found | `/usr/bin/{git,make,cmake}` | — |

网络可达性（实测，与 ai89 的结论一致）：

| 目标 | 结果 |
|---|---|
| `static.rust-lang.org/dist/channel-rust-stable.toml` | `HTTP/2 200` |
| cargo sparse index（`cargo fetch`） | 成功，按 `Cargo.lock` checksum 解析 |
| `api.github.com` | `200` |
| `codeload.github.com`（tarball） | `200`，7,973,813 字节完整下载 |
| GitHub release 资产（`github.com/.../releases/download/...`） | `200`，43,693,999 字节完整下载 |
| `crates.io/api/v1/crates/rossi`（直接 curl） | `403`（不影响 cargo） |

---

## 2. Rust toolchain 升级（`rossi`/`cvc5` 之外的前置修复）

`cargo build` 在 1.86.0 下直接失败，因此先升级：

```
$ cargo build
error: rustc 1.86.0 is not supported by the following packages:
  ar_archive_writer@0.5.3 requires rustc 1.88.0
  psm@0.1.32 requires rustc 1.88.0
  zip@8.6.0 requires rustc 1.88
```

```
$ rustup update stable            # exit 0
info: latest update on 2026-10-15 for version 1.99.1 (58cf8f9c3 2026-10-13)
  stable-x86_64-unknown-linux-gnu updated - rustc 1.99.1 (58cf8f9c3 2026-10-13) (from rustc 1.86.0)
$ rustc --version && cargo --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)
cargo 1.99.1 (5f94df478 2026-08-27)
```

与 ai89 `docs/tooling-versions.md` 记录的 `1.99.1` 对齐。

---

## 3. 安装动作（均有本次人类授权："请检查哪些工具和环境缺失并设法安装"）

### 3.1 取源方式说明

`crates.io` 上的 `rossi 0.2.3` 是**纯库**（ai89 已实测 `cargo install rossi` → `there is nothing to install ... it has no binaries`），CLI 必须从上游 tag 源码构建。ai86 同样不可用 `git clone` GitHub，故走 codeload tarball。

### 3.2 `rossi` CLI 0.2.3

```
$ mkdir -p ~/tools && cd ~/tools
$ curl -sSL -o rossi-0.2.3.tar.gz \
    https://codeload.github.com/eventb-rossi/rossi/tar.gz/refs/tags/v0.2.3
  # exit 0；7,973,813 字节（与 ai89 记录字节数一致）
$ tar xzf rossi-0.2.3.tar.gz && mv rossi-0.2.3 rossi-0.2.3-src     # exit 0
$ cd ~/tools/rossi-0.2.3-src
$ cargo install --path crates/rossi-cli --locked                    # exit 0，1m50s
  Installing /home/mye/.cargo/bin/rossi
   Installed package `rossi-cli v0.2.3 (...)` (executable `rossi`)
$ ln -sf ~/.cargo/bin/rossi ~/.local/bin/rossi                      # exit 0
$ rossi --version                                                   # exit 0
rossi 0.2.3
```

产物：`~/.cargo/bin/rossi`（5,211,224 字节）、源码树 `~/tools/rossi-0.2.3-src`（229 MB）。

### 3.3 `cvc5` 1.4.2

```
$ cd ~/tools
$ curl -sSL -o cvc5.zip \
    https://github.com/cvc5/cvc5/releases/download/cvc5-1.4.2/cvc5-Linux-x86_64-static.zip
  # exit 0；43,693,999 字节（与 ai89 记录字节数一致）
$ unzip -q -o cvc5.zip -d cvc5-1.4.2                                # exit 0
$ ./cvc5-1.4.2/cvc5-Linux-x86_64-static/bin/cvc5 --version          # exit 0
cvc5 1.4.2 [git 282d5e0 on branch HEAD]
compiled as a unrestricted build with GCC version 11.4.0 on Oct  8 2026 01:24:07
$ ln -sf ~/tools/cvc5-1.4.2/cvc5-Linux-x86_64-static/bin/cvc5 ~/.local/bin/cvc5
$ cvc5 --version                                                    # exit 0
cvc5 1.4.2 [git 282d5e0 on branch HEAD]
```

与 ai89 为**同一官方构建**（同 tag、同 git hash、同构建时间、同字节数），跨机 SMT 结果可比。产物：`~/tools/cvc5-1.4.2`（151 MB）。

> 未采用 `pip install cvc5`：ai89 已实测该 wheel 只含 Python 绑定，不提供命令行。

### 3.4 PATH

`~/.cargo/bin` 已在 ai86 的 PATH 中（与 ai89 不同，无需为 cargo 做软链）。仍把 `rossi`、`cvc5` 软链到 `~/.local/bin`，使非登录非交互 shell 也能解析：

```
$ bash -c 'command -v rossi cvc5 cargo'
/home/mye/.local/bin/rossi
/home/mye/.local/bin/cvc5
/home/mye/.cargo/bin/cargo
```

---

## 4. 安装后的真实验证（命令 + 退出码）

```
$ cargo build                     # exit 0 — Finished `dev` profile
$ cargo test                      # 第 1 次 exit 0 — 全部通过：
                                  #   lib 72；baseline 11；cli 1；cli_gate 8；cli_report 7；
                                  #   feedback 8；gate_policy 7；integration_inv 6；
                                  #   integration_refine 6；inventory 6；po_spike 6；
                                  #   prompts 4；provenance 6；report_schema 5
$ cargo test                      # 复跑 exit 101 — 仅 tests/feedback.rs 2 项失败，
                                  #   其余套件仍 ok。见 §6.1，与依赖安装无关
$ rossi export --build -o /tmp/itrodin models/counter/m0.eventb models/counter/m1.eventb
                                  # exit 0 — m0/m1 的 .bcm/.bpo/.bps/.bum
$ python3 scripts/po-inventory.py /tmp/itrodin
                                  # exit 0 — rossi 0.2.3, po_total 20, {INV:16, GRD:2, SIM:1, FIS:1}
$ ./target/debug/intentower gate --model-id MODEL-001 --out /tmp/it-gate-m1.json models/counter
                                  # exit 2 — INCOMPLETE 20 obligations, inputs 4;
                                  #          1 UNSUPPORTED: m0::sub/act1/FIS
$ ./target/debug/intentower baseline
                                  # exit 0 — BASELINE OK: 4 hashes, 2 model ids pinned
$ python3 scripts/validate-report.py schemas/report.schema.json \
      tests/fixtures/report/counter-m0m1-incomplete.json
                                  # exit 0 — OK ... expected valid, valid
$ printf '(set-logic QF_LIA)\n(assert (>= (+ 1 2) 2))\n(check-sat)\n(exit)\n' > /tmp/t.smt2
$ cvc5 /tmp/t.smt2                # exit 0 — sat
```

PO 总数与类别分布、`m0::sub/act1/FIS` 的 `UNSUPPORTED` 结论均与 ai89 记录一致，属**复现**而非新结果。所有产物写入 `/tmp`，仓库工作树未被这些命令改动（`git status` 前后一致）。

---

## 5. Strata / Pi（未安装，走隧道）

| 项 | 实测 |
|---|---|
| 本机 Strata 引擎 | missing（`~/tool` 下无 Strata） |
| `strata-ai90-tunnel.service` | `loaded active running` — SSH tunnel to ai90 Strata（localhost:8081 → ai90:8080） |
| `curl http://127.0.0.1:8081/v1/models` | `HTTP 200`，`qwen3.8-flash-next-iq3_s`，`status=loaded`，`n_ctx=131072` |
| Pi 配置 | `defaultProvider=strata-ai90`、`defaultModel=qwen3.8-flash-next-iq3_s` |

结论：ai86 不需要本机部署 Strata。注意 `docs/rossi-smoke.md` 的教训仍然适用 —— 端口/进程核对才能发现 provider 是否真的活着。

---

## 6. 遗留风险（本次未修）

1. **测试竞态（非环境问题，已复现）**：`cargo test` 在 ai86 上不稳定 —— 复跑 4 次失败 2 次，失败项只出现在 `tests/feedback.rs`：
   - `the_committed_failure_fits_the_budget_and_names_the_obligation`（`tests/feedback.rs:101`，`the one writable file is named`）
   - `the_packet_sends_the_round_to_its_own_report_not_to_the_one_that_failed`（`tests/feedback.rs:247`，`the reproduce command must not overwrite the failed round's evidence`）

   原因在 `crates/intentower-cli/tests/feedback.rs:25`：`out_dir(label)` 只用 `pid + label`，而 `packet()`（第 55 行）固定 `label="packet"`，被第 85/126/220/246 行四个测试共用；`out_dir` 里的 `remove_dir_all` 会删掉并发测试正在读的 `packet.md`。修法是每个测试用独立 label（或把 label 加上测试名）。该代码为 rsync 带来的未跟踪 WIP（`tests/feedback.rs`、`src/feedback.rs`），**本次未改**，也未据此放宽任何验收判据。
2. **WIP 代码的静态检查未过**：`cargo fmt --check` 在 `crates/intentower-cli/src/feedback.rs:48` 报格式 diff（exit 1）；`cargo clippy` 有 2 个 warning。同为 WIP 自带，本次未改。
3. `rodin` / `prob` / `z3` 仍缺；`java 1.8.0_362` 不足以跑现代 Rodin。Phase 1 不需要，若启用独立复核（§9 of AGENTS）需另立任务。
4. 本机安装记录**未**更新 `approvals/baseline.json`，也未修改 `docs/tooling-versions.md`。跨机比对时以两份文档并列核对版本。

---

## 7. 更新（2026-11-04，P3 重签时）

§6.1 与 §6.2 记录的 WIP 缺陷已修复，原始记录保留不删：

- `tests/feedback.rs`：`packet(fixture)` → `packet(fixture, label)`，四个调用点各用唯一 label。
- 清掉本次改动造成的孤儿：`tests/prompts.rs` 未使用的 `Command` 导入与 `BIN` 常量；`src/feedback.rs` 两处 clippy。
- `cargo fmt --all -- --check` exit 0；`cargo clippy --offline --all-targets` 0 warnings。
- `cargo test --offline` 连续 5 次：**156 passed / 0 failed**（§4 的套件列表漏了 bin unit 3 与 doc-tests 0）。

逐条 gate 证据重跑见 `docs/rossi-smoke.md` §31。
