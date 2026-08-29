# ADR-0015：按 workspace 选择稳定 CLI session

- 状态：Accepted
- 日期：2026-08-29
- 决策者：KiloxGo

## 背景

全局 `lain` 会从任意项目目录启动本地 kernel。此前 SQLite 总是复用最近更新的 active session，导致不同项目可能看到彼此的消息与编译上下文；为每个项目拆分整个数据目录又会割裂人格和长期记忆。

## 决策

1. CLI 自动启动 kernel 时继续把调用目录作为 workspace。
2. kernel 规范化并尽量 canonicalize workspace 绝对路径；Windows 额外转为小写，再计算 SHA-256。
3. 使用 `workspace:<sha256>` 作为现有 `sessions.title`，在同一 active instance/user 下精确查找或创建 session。同一路径稳定复用，不同路径分离。
4. 没有 workspace 的兼容入口继续使用 `Main`；不新增数据库列、服务层或协议字段。

## 备选方案

- 每个项目使用独立 `LAIN_DATA_DIR`：隔离更彻底，但会拆散人格、记忆和设置。
- 新增 session key 列与迁移：当前单用户本地 CLI 不需要额外结构。
- 继续选择最近 session：实现最少，但跨项目上下文泄漏不可接受。

## 影响

- 全局 CLI 在项目之间不再串消息、run 与 context plan；instance-level SelfState 和 user memory 仍可共享。
- 旧 `Main` session 保留且可由无 workspace 的兼容入口访问，不自动搬迁历史消息。

## 安全与隐私

SQLite 仅保存 workspace 路径哈希，不保存原始路径或项目名。哈希用于本地分区标识，不作为安全凭据；工具访问仍由 canonical workspace 边界与审批策略约束。

## 迁移与回滚

复用现有 `sessions.title`，无需数据库迁移。回滚为默认初始化后，新建的 workspace session 仍保留在库中，但不会被自动选择。

## 验证方式

- 同一数据目录与同一 workspace key 重启后必须得到相同 scope。
- 不同 workspace key 必须共享 instance/user，但得到不同 session。
- 标题稳定性测试必须确认不包含原始路径。
- Rust 全量测试、Clippy 和 rustfmt 必须通过。
