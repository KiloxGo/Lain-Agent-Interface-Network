# P7-lite SelfState 验收报告

- 日期：2026-08-29
- 分支：`feat/cli-mvp`
- 维护者：KiloxGo
- 结论：终端人格的第一层运行时闭环通过；这是 P7-lite，不宣称语音、gesture 或 Live2D 已完成。

## 已验收范围

- chat、task、maintenance 三种 host-owned signal 确定性归约为有限 SelfState。
- SelfState 所有连续值保持 0..=1，active intent 最多五个，TTL 固定十分钟。
- 每个 run 写入版本化 snapshot 与 `state.changed` 审计事件；事件不包含用户正文。
- 当前 run 的 Context Compiler 读取该状态，并注入固定 affect → voice directive。
- CLI 接收同一 `state.changed` 事件，以克制的 `state › curious` 形式显示当前 affect，并在 `/status` 中保留状态。
- 上下文明确规定状态只能改变 voice，不能改变权限、事实、工具风险或完成标准。
- 过期状态不会进入上下文；下一次 run 由新的 host signal 重置。

## 自动化结果

- Rust tests：95 passed，4 suites。
- Clippy：`--all-targets --all-features --locked -- -D warnings` 通过。
- rustfmt：`cargo fmt --all -- --check` 通过。
- reducer 定向测试与 state snapshot/event round-trip 定向测试均通过。

## 明确延期

- affect → gesture、终端头像变化、TTS 与 Live2D。
- 基于真实任务轨迹的 failure/success/cancellation 状态信号。
- relationship reducer、用户可见状态控制与人格 A/B 评测。

下一步应先让 CLI 可见但克制地呈现 state，并用人格评测证明表达差异不会降低任务成功率，再接语音或 Live2D。
