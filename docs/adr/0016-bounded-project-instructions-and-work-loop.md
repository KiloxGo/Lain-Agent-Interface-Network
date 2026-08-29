# ADR-0016：有界项目指令与外部项目工作闭环

- 状态：Accepted
- 日期：2026-08-29
- 决策者：KiloxGo

## 背景

全局 `lain` 已能在任意 workspace 中读取、编辑和运行检查，但模型此前必须自行发现项目约定，且 4 轮/8 次工具上限只够一次直线修改，没有检查失败后的恢复余量。这使它具备工具，却尚未形成可靠的外部项目工作闭环。

## 决策

1. 只识别 workspace 根目录的标准 `AGENTS.md`，不增加 `LAIN.md`、别名或递归继承规则。
2. 每个 run 在编译上下文前重新读取该文件：canonical path 必须仍位于 workspace，内容必须是 UTF-8，最大 32 KiB，换行归一化。文件不存在等价于无项目指令；存在但不可安全读取时 run 明确失败。
3. Context Compiler v2 为 project instructions 保留 8192 tokens，将内容哈希与选择记录写入 ContextPlan，不保存本地路径。项目指令属于 user-provided guidance，不能授权工具、覆盖隐私边界或要求泄露 Secret。
4. Harness Contract 固定要求 workspace 任务先检查再修改，实际变更后尽可能运行相关检查，最终报告 outcome、changed files、checks 和 blockers。
5. 工具循环保持硬上限但扩至 8 轮/16 次调用；chat route 在 provider 支持工具时使用现有工具。complex-task 在 reasoning replay 完成前继续不注入工具。

## 备选方案

- 自定义 `LAIN.md` 或 `.lain/`：品牌化更强，但破坏通用约定且当前没有必要。
- 向父目录递归并支持目录级覆盖：适合 monorepo，但需要明确优先级、边界和缓存失效规则，留待真实需求出现。
- 无界 agent loop：任务覆盖率更高，但成本、卡死和副作用面不可接受。
- 新建 planner/task 服务：当前工具回执和固定完成契约已足以覆盖小型修复。

## 影响

- 外部项目无需在 prompt 中重复仓库规则；同一 session 的每个新 run 都能看到最新根级规则。
- Context Compiler 兼容版本从 1 升至 2，稳定前缀会变化一次；旧 ContextPlan 仍可审计，不会被重写。
- 单个 run 最多可产生 9 次模型请求（初始请求加 8 个后续轮次），但通常在检查成功后提前结束。

## 安全与隐私

`AGENTS.md` 只提供项目操作指导，不构成授权。所有写入、检查、shell 与 MCP 调用继续使用原有一次性审批、action hash 和审计事件；排除路径与 Secret 环境清理不变。ContextPlan 只记录内容哈希、token 估计与选择状态。

## 迁移与回滚

没有数据库迁移。仓库 identity manifest 将 compatible compiler version 更新为 2；回滚时同时恢复 compiler version、manifest 与 CompileInput 字段即可。新增在线基线只含合成项目和 token usage。

## 验证方式

- WorkspaceTools 单测覆盖缺失文件、CRLF 归一化和 32 KiB 上限。
- Context Compiler 单测覆盖 project instructions 的 system message、来源类型、trust level 与确定性 hash。
- 真实 `coding-work-loop-001` 必须完成目录检查、并行读取、精确修改、`npm test` 和受规则约束的最终报告。
- Rust 全量测试、Clippy、rustfmt、CLI 测试和离线 eval 校验必须通过。
