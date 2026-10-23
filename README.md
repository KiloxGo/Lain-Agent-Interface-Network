# Lain Agent Interface Network

> 常驻云端，与用户平等相处，并连接日常生活和本地工作的个人智能体。

**产品方向更新：2026-10-23。** 当前优先做出可以每天通过 QQ 相处的 Lain：
有持续身份和真实经历，能在授权范围内阅读、记忆、分享和主动联系。
本地终端与 Worker 管理继续发展，所有入口连接同一个云端 Lain。

**实现状态：** Phase 7.1 基础包括 Rust Kernel/CLI、Codex 工作监督、持久对话、
明确确认的工具操作、独立记忆和事实结果回传。新增 Q1 解耦 QQ 私聊实现，
真实 NapCat/模型对话和部署仍待验收。设备执行端、长期自主授权与新的
日常活动系统尚未实现。Web 保留且冻结。原有跨平台验收和
Phase 6.6/7.1 真实模型验收仍有缺口；详见验收文档，不以产品设计替代测试证据。

## 从这里阅读

| 文档 | 内容 |
| --- | --- |
| [产品定义与边界](docs/PRODUCT.md) | 用户确认的方向、平等关系、体验与开放问题 |
| [云端与设备设计](docs/RESIDENT_DESIGN.md) | QQ 双账号、数据归属、档案、授权、断线恢复 |
| [聊天与日常活动](docs/CHAT_DESIGN.md) | 人格、时机、自然回忆、纠正、主动联系 |
| [QQ 私聊操作指南](docs/QQ_CHANNEL.md) | 配置、授权、恢复、部署边界与真实联调 |
| [Q1 实现验收](docs/Q1_IMPLEMENTATION.md) | 本轮实现范围、离线证据与未完成验收 |
| [MaiBot 参考研究](docs/MAIBOT_REFERENCE.md) | 已阅读的机制、适合借鉴之处与未验证范围 |
| [架构](docs/ARCHITECTURE.md) | 新目标与当前本地实现的区别 |
| [领域模型](docs/DOMAIN_MODEL.md) | 对话、工作、记忆及拟议实体边界 |
| [运行模型](docs/RUNTIME_MODEL.md) | 生命周期、活动与恢复语义 |
| [记忆](docs/MEMORY.md) | 自主可纠正目标与当前 Phase 7.1 契约 |
| [路线图](ROADMAP.md) | QQ 优先的新顺序与旧阶段记录 |
| [ADR 009](docs/adr/009-cloud-resident-equal-companion.md) | 哪些旧决策被替代、哪些实现边界仍有效 |
| [开发记录](DEVLOG.md) | 实际推进与验证结果 |
| [开发约定](AGENTS.md) | 后续开发必须遵守的边界 |

## 想要的体验

早上可以问昨天群里有什么重要消息、昨天工作推进了什么。Lain 能回查授权保存的来源，
区分已经确认的安排、别人的说法和仍不确定的事情。

平时通过她自己的 QQ 与她聊天。她记得共同经历，气质冷静、略带电波感，可以主动分享
发现，也知道你上课或想安静的时候应暂缓联系。双方平等相处，没有主仆设定。

电脑上的终端提供 Chat/Work 和有 Lain 气质的 ASCII 表现。Worker 执行时仍能和她聊天；
可以进入 session 直接交互，也能回到 Lain 查看整体进度。这些是目标体验，不是现有
终端已经全部支持的功能。

## 系统边界

- 云端是统一身份、记忆和经历的权威；本地设备执行、采集和报告。
- 选定社交来源只读观察，Lain 账号与用户双向交流；第三方行动单独授权。
- 可归档 Lain 内外产生的可见 Worker 会话；阅读不等于接管。
- 原始档案、摘要、经历、语义记忆和技能模板各有用途。
- 自主行为需要范围、预算和结果，不靠持续模型推理制造常驻感。
- 复杂工程工作交给外部 harness；不重建通用编码 Agent。
- 当前审批检查不能直接删掉，必须由真实的 scoped authorization 设计和迁移替代。

## 当前源码

Rust workspace 有三个 crate：

- `lain-kernel`：Kernel 库与 `laind`，Axum/Tokio/SQLite，本机 Codex 监督和 Actor。
- `lain-client`：`lain` CLI/TUI，经 HTTP/SSE 访问 Kernel。
- `lain-wire`：公共强类型 ID 和数据结构。

`web/` 是保留的 React/TypeScript 客户端。终端开发不要求 npm。
`legacy/v0` 保存旧原型，仅供历史参考；本轮调整不恢复其架构。

以下开发操作仍描述当前**本地实现**。远端 API 客户端不等于已经具备远程 PC 执行服务。

## Development

The workspace contains `lain-kernel` (library and `laind`) and `lain-client`
(`lain`, HTTP/SSE only). It supervises real Codex workers with durable sessions,
detachable runtimes, turns, approvals and indexed operational history. Idle workers
release their process; follow-ups resume the same session lazily. Worker execution
remains owned by Codex.

Windows PowerShell and macOS zsh use the same Rust codebase and commands:

```text
cargo build --locked --workspace
cargo install --locked --path crates/lain-kernel --bin laind --force
cargo install --locked --path crates/lain-client --bin lain --force
lain setup --local
lain up
```

In another terminal: `lain doctor`, `lain status`, `lain chat`. Setup keeps the
local token in an owner-only per-user file; no token copying between terminals.
`up` launches the existing Kernel in the foreground; Ctrl-C gracefully stops it.
No npm, browser or Vite is required. See [CLI.md](docs/CLI.md) for PowerShell/zsh
binary paths, native credential permissions, PATH conflict diagnostics, remote
mode, existing work/profile/approval commands and platform acceptance entrypoints.

Chat requires its own explicit model configuration and namespaced SecretRef;
Codex login is separate. Copy [actor.example.json](config/actor.example.json) to
Git-ignored `config/actor.json` for local development, edit the model/endpoint,
and save the selected file plus a protected credential-source file as described in
[LAIN_ACTOR.md](docs/LAIN_ACTOR.md). That guide documents setup,
persona, persistent conversations, typed tools, user confirmation and recovery.
`lain --json ask` supports scripting; mutations remain pending until explicitly
approved. Existing workers run independently of chat/observers. Conversations are
operational continuity, separate from Phase 7.1 selected semantic memory.

Confirmed chat delegation now correlates the exact instruction with a worker
Turn. The Kernel delivers its actual outcome back to the conversation without
another user message or model call, while preserving in-progress terminal input.
Use `lain notices <conversation-id>` for retained factual results and
`lain delegations <conversation-id>` for exact dispatch inspection. Completed
worker Turns can include failed commands; notices distinguish those facts.
See [Phase 6.6 acceptance](docs/PHASE_6_6_ACCEPTANCE.md) for current evidence and
the opt-in real-provider procedure; no earlier real run proves this new path.

Chat displays sanitized incremental previews, then reconciles the completed reply
without repeating it. Use `lain chat --timing` or `lain ask --timing "你好"` for
measured latency diagnostics on stderr; JSON ask stdout remains a final snapshot.
`lain config actor set --file <actor.json>` and `config actor env-file --file
<actor.env>` let `lain up` reuse local settings without repeated exports. Doctor
reports observable startup differences; apply changes with an explicit graceful
stop/start after inspecting active work. Native macOS real Actor/Codex delegation,
worker completion and recovery were verified on 2026-10-14 across controlled
attempts; intermittent model tool omissions are documented. Native Windows
acceptance remains pending.
The current evidence and conditional memory-phase handoff are in
[Phase 6.5 acceptance](docs/PHASE_6_5_ACCEPTANCE.md); fixture checks do not establish
real provider compatibility.

The React/TypeScript Web client remains available through the existing optional
`LAIN_WEB_DIR` workflow, documented in [WEB.md](docs/WEB.md). Web development is
paused; Phase 6 has no browser chat or redesign. Public Internet/TLS/session-cookie
authentication remains unimplemented; remote clients need trusted transport.

Native macOS deterministic build/test/CLI smoke is validated. Windows and Linux
checks must run on their native hosts before acceptance is claimed. Real paid
Phase 6.6 Actor/Codex inference acceptance is opt-in and unverified. Exact evidence and
limitations are in [DEVLOG.md](DEVLOG.md).

Phase 7.1 adds independent SQLite memory with inspectable origin, scoped project
knowledge, explicit user review for Actor proposals and factual SessionSummaries.
Use `lain memory add/list/search/show/disable/enable/delete`, `lain memory candidate`
and `lain summary create/show/list` without a browser or inference. Memory survives
daemon restart; summaries and transcripts are never automatically promoted.
See [memory](docs/MEMORY.md) and [CLI commands](docs/CLI.md). Persona stays separate;
World/journals, reflection, scheduling and prompt compilation remain unimplemented;
their future order is governed by the new QQ-first roadmap, not the old deferrals.
Native macOS fixtures validate the path; real-provider Phase 7.1 retrieval and
Windows/Linux acceptance remain unverified.

Before architectural work, read `AGENTS.md` and `ROADMAP.md`.
