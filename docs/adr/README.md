# Architecture Decision Records

ADR 用于记录难以逆转、跨模块或影响数据/协议兼容性的技术选择。目标架构中的首批决策候选见根目录技术实施计划；它们在实现对应模块时拆成独立 ADR。

## 状态

- Proposed：仍可讨论，不能作为稳定兼容承诺。
- Accepted：已作为实现约束。
- Superseded：由后续 ADR 替代，原记录保留。
- Deprecated：不再建议使用，但可能仍需兼容。

## 命名

使用 `NNNN-short-title.md`，编号只增不改。每个 ADR 至少包含：背景、决策、备选方案、影响、安全/隐私、迁移与回滚、验证方式。

## 已接受决策

- [ADR-0001：Kernel application boundary](0001-kernel-application-boundary.md)
- [ADR-0002：Provider stream and P2 wire](0002-provider-stream-and-p2-wire.md)
- [ADR-0003：SQLite 事件存储与 session 协议 v1](0003-sqlite-event-store-and-session-v1.md)
- [ADR-0004：版本化 Identity Kernel 与确定性 Context Compiler](0004-versioned-identity-and-deterministic-context.md)
- [ADR-0005：有来源、可确认、可重建检索的 Memory v1](0005-provenanced-memory-v1.md)
- [ADR-0006：一次性审批的精确 workspace 编辑](0006-one-shot-approved-workspace-edit.md)
