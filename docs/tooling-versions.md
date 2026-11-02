# T07 — 工具与环境实测清单

- **Date**: 2026-11-02 · **Host**: `CentOS Stream 9` · **Executed by**: Pi
- **Plan reference**: §3 P1/T07、§1.1（版本必须实测，不得猜测）
- **Rule applied**: 缺项标 `missing`；未运行的检查标 `not_run`；安装只在获得明确授权后执行，并记录来源与版本。

---

## 1. 工具状态（2026-11-02 安装动作之后）

| 工具 | 状态 | 版本（实测） | 路径 | 证据命令 |
|---|---|---|---|---|
| `pi` | found | `1.1.0` | `/home/mye/.nvm/versions/node/v25.1.0/bin/pi`（另有 `~/.local/bin/pi`） | `pi --version` |
| `node`（pi 运行时） | found | `v25.1.0` | nvm | `node --version` |
| Pi provider | found | `defaultProvider=strata`、`defaultModel=qwen3.8-flash-next-iq3_s`、`defaultThinkingLevel=high` | `~/.pi/agent/settings.json` | 读取 JSON（密钥脱敏） |
| Strata 服务端点 | found（运行中） | 模型 `qwen3.8-flash-next-iq3_s`，`status=loaded`，`n_ctx=131072` | `http://127.0.0.1:8080/v1` | `curl .../v1/models` → `HTTP=200` |
| `strata` 引擎 | found（不在 PATH） | — | `/home/mye/tool/Strata/engine/strata`（63,311,816 字节） | `ls -l`；进程 `strata --serve --pack .../iq3_s ... --max-context 131072 --kv int8` |
| Strata 启动方式 | found | user-level systemd unit `strata-server.service`（`active/running`），ExecStart `/usr/bin/bash /home/mye/tool/Strata/run-iq3_s.sh`；包装进程 `/home/mye/tool/Strata/.venv/bin/python /home/mye/tool/Strata/serve/server.py --engine strata --config /home/mye/tool/Strata/strata-iq3_s.json --port 8080` | — | `systemctl --user list-units`、`ps` |
| `rustc` | found | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` | `~/.cargo/bin/rustc` → 软链 `~/.local/bin/rustc` | `rustc --version` |
| `cargo` | found | `cargo 1.99.1 (5f94df478 2026-08-27)` | `~/.cargo/bin/cargo` → 软链 `~/.local/bin/cargo` | `cargo --version` |
| `rustup` | found | `rustup 1.29.2 (d8c92c676 2026-10-21)` | `~/.cargo/bin/rustup` | `rustup --version` |
| Rust toolchain | found | `stable-x86_64-unknown-linux-gnu (active, default)`；含 `rustfmt`、`clippy` | `~/.rustup` | `rustup show` |
| `rossi` | **found** | `rossi 0.2.3` | `~/.cargo/bin/rossi` → 软链 `~/.local/bin/rossi` | `rossi --version` |
| `rossi-prove` | found（库 crate） | `0.2.3` | crates.io，进 `Cargo.lock` | **已是 `intentower-cli` 生产依赖**（T14，见 ADR-0002） |
| `cvc5` | **found** | `cvc5 1.4.2 [git 282d5e0 on branch HEAD]`，unrestricted build，GCC 11.4.0，built 2026-10-08 | `~/tools/cvc5-1.4.2/cvc5-Linux-x86_64-static/bin/cvc5` → 软链 `~/.local/bin/cvc5` | `cvc5 --version` |
| `z3` | missing（第一阶段不需要） | — | — | `command -v z3` → 空 |

## 2. 机器与环境

| 项 | 实测值 |
|---|---|
| OS | CentOS Stream 9（`ID_LIKE=rhel fedora`） |
| Kernel / arch | `Linux 5.14.0-631.el9.x86_64`，`x86_64` |
| CPU | Intel(R) Xeon(R) CPU E5-2673 v4 @ 2.30GHz，`nproc=80` |
| 内存 | 总 503GiB，available 332GiB |
| 包管理器 | `dnf 4.14.0`（无 `cvc5` / `python3-cvc5` 包：`错误：没有匹配的软件包可以列出`） |
| Python / pip | `/opt/anaconda3/bin/python3`（3.13）、`pip 25.1` |

## 3. 已执行的安装（均有明确授权）

| 时间 | 动作 | 结果与来源 |
|---|---|---|
| T01 | `rustup` minimal + stable | `rustc/cargo 1.99.1`，用户级 `~/.cargo`、`~/.rustup` |
| T01 后 | `rustup component add rustfmt clippy` | `cargo fmt --check`、`cargo clippy` 可用 |
| T07 | `pip install --user cvc5==1.4.2` → **随后卸载** | 该 wheel 只含 Python 绑定（`cvc5.libs/libcvc5-*.so.1.4.2`、`cvc5_python_base...so`），**不提供 `cvc5` 命令行**，不满足计划 §1.1 的命令行调用方式 |
| T07 | 下载官方 release 资产 `cvc5-Linux-x86_64-static.zip`（43,693,999 字节，`cvc5-1.4.2`）解压到 `~/tools/cvc5-1.4.2/` | `cvc5 --version` → `cvc5 1.4.2 [git 282d5e0 on branch HEAD]` |
| T07 | `cargo install rossi --version 0.2.3` **失败** | `error: there is nothing to install in rossi v0.2.3, because it has no binaries`（crates.io 上的 `rossi` 是纯库） |
| T07 | 改用 codeload tarball `refs/tags/v0.2.3`（7,973,813 字节）解到 `~/tools/rossi-0.2.3/`，`cargo install --path crates/rossi-cli --locked` | 编译 4m45s，安装 `rossi-cli v0.2.3` → `~/.cargo/bin/rossi`；`rossi --version` → `rossi 0.2.3` |
| T07 | 把 `~/.cargo/bin/*` 全部软链到 `~/.local/bin`（已在 PATH、可写） | 非交互 shell 现在能找到 `cargo`、`rustc`、`rossi`、`cvc5` |

**Rossi 版本溯源**：tag `v0.2.3`，commit `888073efe336`（`api.github.com/repos/eventb-rossi/rossi/tags`），与 crates.io `rossi 0.2.3` / `rossi-prove 0.2.3` 对齐。

## 4. 关键环境发现

1. **PATH**：`~/.bashrc:28` 与 `~/.profile:1` 已含 `. "$HOME/.cargo/env"`，但 Pi 用的是非登录非交互 shell（`bash -c 'command -v cargo'` → 空；`bash -lc` → 有）。已用 `~/.local/bin` 软链解决，无需每条命令 `source ~/.cargo/env`。
2. **GitHub 部分不可达**（影响 T10/T13 取源方式）：

   | 目标 | 结果 |
   |---|---|
   | `static.rust-lang.org` | `HTTP/2 200` |
   | `crates.io/api/v1/...`、cargo index | `HTTP=200`，`cargo add --dry-run rossi@0.2.3` 成功解析 |
   | `api.github.com` | `HTTP=200` |
   | `codeload.github.com`（tarball） | `HTTP=200`，实测 7.7–8.0 MB 下载成功 |
   | GitHub release 资产（`github.com/.../releases/download/...` → `release-assets.githubusercontent.com`） | `HTTP=200`，43,693,999 字节完整下载 |
   | `github.com`（HTML） | 超时无输出 |
   | `raw.githubusercontent.com` | `curl: (35) Recv failure: 连接被对方重设`，`HTTP=000` |
   | `git ls-remote https://github.com/...` | **超时，exit 124** |

   **结论**：`git clone` GitHub 不可用；取上游源码一律走 codeload tarball 或 api.github.com。
3. **Rossi CLI 接口与计划一致**（`rossi --help` / `rossi export --help` 实测）：子命令 `validate import export fmt build dump clean prove completions`；`rossi export --build` 明确"Also static-check and generate proof obligations, including the checked `.bcc`/`.bcm` and generated **`.bpo`/`.bps`** files"。计划 §1.1 与 T10 的假设成立，无需 ADR。
4. **Rossi 官方 counter 示例已在本地源码树**（T10/T13 直接可用，不必自造语法）：`~/tools/rossi-0.2.3/crates/rossi/examples/` 下有 `counter.eventb`、`counter_machine.eventb`、`counter_ctx.buc`、`counter.bum`；全仓 `.eventb`/`.b` 夹具 32 个。
5. `rossi prove` 是"检查已存证明"，不能当作从零证明新 PO（计划 §1 已警示），T12 仍须走 `rossi-prove` 的类型化 PO API。

## 5. 未验证项（`not_run`）

- `rossi validate` / `rossi export --build` 的真实输出与 `.bpo` 结构（T10）。
- `.bpo` 中 PO 的类别与数量（T11）、`rossi-prove::PoProject / PoFile / ProverSequent` API（T12）。
- cvc5 对 SMT-LIB 的 `sat` / `unsat` / `unknown` 输出与退出码（T09）。
- `strata connect` 官方连接方式（T08；引擎与端点已确认，CLI 不在 PATH）。

## 6. 下一步

P1 依赖已就绪：`rossi 0.2.3` + `cvc5 1.4.2` + `cargo 1.99.1`。建议顺序 **T09（cvc5 冒烟）→ T10（Rossi counter + `export --build`）→ T11（PO 清单）→ T12（类型化 PO API Spike）**；T08 需你确认 `strata` CLI 是否要加入 PATH。
