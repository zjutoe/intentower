# ADR-0002 — Rossi 库成为 `intentower-cli` 的生产依赖

- **Status**: accepted（2026-11-02，zjutoe 授权"加进 workspace"）
- **Task**: T14

## Context

T12 的 spike 只在 `[dev-dependencies]` 里用 `rossi` / `rossi-prove`。T14 要求实现 `Po` 内部结构并"从 T12 导入"，即产品代码 `src/po.rs` 必须自己读取类型化 PO。

可选路线：

1. 解析 `.bpo` XML（自研或第三方 XML crate）。
2. 调用 `rossi` CLI 导出后再解析文本。
3. 直接依赖 `rossi` / `rossi-prove` 库，用 `PoProject::load` 取 `ProverSequent`。

计划对 T12 明令"禁止通过正则表达式解析数学公式"，且 T11 已实测 `.bpo` 的谓词集引用链有隐蔽陷阱（同名集合、转义 `|`、跨文件链），路线 1 风险最高；路线 2 会把类型信息降级成文本。

## Decision

把 `rossi = "0.2.3"` 与 `rossi-prove = "0.2.3"` 移到 `crates/intentower-cli/Cargo.toml` 的 `[dependencies]`。版本与已安装的 Rossi CLI 一致（0.2.3），具体解析结果冻结在 `Cargo.lock`。不 vendor 源码：crates.io 的 0.2.3 与本地 tag `v0.2.3`（commit `888073efe336`）已实测一致（能编译，结论与 CLI 输出吻合）。

## Consequences

- 正面：PO 来自 Rossi 的同一套加载器，类型（`Type`）与假设链（含 WD 前提）由 Rossi 解析，Intentower 不重复实现也不曲解；SPEC-001 SP-G9"PO 清单只能来自 Rossi POG 产物"在代码结构上得到保证。
- 代价：冷编译时间增加（rossi + rossi-prove 及其依赖约 1–5 分钟）、二进制变大；我们从此绑定 Rossi 的 **库 API**，而不只是 CLI。
- 约束：升级 Rossi 版本必须重跑 `po_spike` 与 `po.rs` 的单测（它们把 64 条 PO 的假设清单钉死为基准），否则语义漂移不会被发现。
