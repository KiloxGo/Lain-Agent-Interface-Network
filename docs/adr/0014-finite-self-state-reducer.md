# ADR-0014：有限、可过期的 SelfState reducer

- 状态：Accepted
- 日期：2026-08-29
- 决策者：KiloxGo

## 背景

版本化 Identity Kernel 已提供稳定人格，Context Compiler 也能注入 SelfState，但运行时此前从不生成状态，因此人格仍主要是静态提示词。让模型自行输出隐藏情绪标签会把人格状态交给不可验证文本，并可能污染权限与事实判断。

## 决策

1. 使用六个 host-owned 输入信号：`Conversation`、`Task`、`Maintenance` 来自现有 route 分类；`Succeeded`、`Failed`、`Cancelled` 来自持久化 run 终态。不从模型输出或工具内容推断状态。
2. reducer 确定性映射到有限 `Affect`、activation、closeness expression、uncertainty 和最多五个 active intent ID；所有数值继续限制在 0..=1。
3. 每次 run 开始及进入成功、失败或取消终态后生成一个十分钟 TTL 的 SelfState，并在同一 SQLite 事务中写入版本化 `state_snapshots` 与 `state.changed` 事件。事件不保存用户正文。
4. 当前 run 在编译上下文时读取刚写入的状态。每种 Affect 映射为一条固定 voice directive；directive 只能改变表达方式，不能改变权限、事实、工具风险或任务完成标准。
5. 终态状态供 CLI 和审计呈现；下一次 run 会先根据新的 route signal 重置。过期状态由 Context Compiler 丢弃，不让旧情绪无限延续。

## 备选方案

- 只依赖静态 identity：稳定但没有运行时行为差异。
- 让模型输出 mood/hidden tag：表达更自由，但状态不可验证且容易被 prompt injection 污染。
- 先建设完整情绪模型：缺少真实评测数据，当前没有必要。

## 影响

- chat、task 和 maintenance 会得到不同但有界的表达倾向；工具和任务能力保持同一执行链。
- 状态是 instance-level 短期快照，不自动修改 relationship 或长期记忆。
- 当前只完成 voice 映射；gesture、TTS 和 Live2D 仍等待对应消费者。

## 安全与隐私

状态完全由 host 枚举产生，不记录原始用户文本，不接受模型提供的 affect。`state.changed` 可审计，且上下文明确禁止状态影响授权与事实。

## 迁移与回滚

复用已有 `state_snapshots`，没有数据库迁移。旧数据库首次运行时直接追加新版本；回滚 reducer 和写入调用后，已有快照会按 TTL 过期。

## 验证方式

- reducer 单测覆盖 task、success、failure、cancellation 映射、数值校验、voice directive 和十分钟 TTL。
- storage 单测覆盖 snapshot 与 `state.changed` 原子写入及 round-trip。
- Context Compiler 现有确定性、预算与过期测试继续通过。
- Rust 全量测试、Clippy 和 rustfmt 必须通过。
