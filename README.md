# Lain Agent Interface Network

> 常驻云端，与用户平等相处，并连接日常生活和本地工作的个人智能体。

**产品方向更新：2026-10-23。** 当前优先做出可以每天通过 QQ 相处的 Lain：
有持续身份和真实经历，能在授权范围内阅读、记忆、分享和主动联系。
本地终端与 Worker 管理继续发展，所有入口连接同一个云端 Lain。

**实现状态：** Phase 7.1 基础包括 Rust Kernel/CLI、Codex 工作监督、持久对话、
明确确认的工具操作、独立记忆和事实结果回传。新增 Q1 解耦 QQ 私聊实现，
基础真实 NapCat/模型私聊于 2026-10-24 由用户人工确认，完整可靠性与部署仍待验收。
Q1 连接诊断与 Q2-A 默认关闭的绑定私聊记忆授权、回忆、纠正和忘记已实现，
已有个人部署和明确范围的私聊记忆试用；它不授权来源归档或跨会话共享。
原生设备服务已有显式配置的 Codex 执行路径；
真实 Linux—Windows HTTPS 模拟 Worker 闭环及一个真实 Codex 三轮网页任务已分范围验收，
见[真实工作验收](docs/WORK_WEB_ACCEPTANCE.md)；这不证明通用 Worker 可靠性。通用自主授权与新的
日常活动系统尚未实现。Web 保留且冻结。原有跨平台验收和
Phase 6.6/7.1 真实模型验收仍有缺口；详见验收文档，不以产品设计替代测试证据。

C1 已接入自然私聊的表达分层、有限等待与收尾、生成中补充重考虑及旧草稿失效，
开发与本地确定性验收已完成；真实自然度未验收。机制参考、离线命令和人工多轮场景见
[C1 验收](docs/C1_ACCEPTANCE.md)。

W2b 新增设备命令、事件回执、取消请求与断线续传协议；提供分离的管理员/设备
CLI 和持久事件日志。W3a 已加入执行身份、有界结果和具体权限决定回执；
W3b 已加入显式启动的原生服务，支持实际进程、权限应用、取消和重启核对；
确定性演练使用模拟 Worker，真实付费／跨设备验收仍单独进行。
操作与验证范围见 [设备命令](docs/DEVICE_COMMANDS.md)、[执行与权限](docs/DEVICE_EXECUTION.md)
和[原生服务](docs/DEVICE_HOST.md)。

## 每日消息 Demo 的当前状态 — 2026-11-07

云端已部署来源管理、昨日简报、原文追问及选中事项的记忆／更正／忘记流程，终端连接
同一个云端身份。隔离来源、实际模型和模拟 QQ 已分范围验证；真实私人来源仍未启用，
因此当前不能查询真实的昨日群消息，也不能据此宣称已经日常可用。

最新云端版本b9a217fa171a在原文回查、整理中短反馈与有界上下文基础上，统一解释收取时间
和来源声称的补发，并保留不含正文的提取拒绝原因；保留原数据库、身份和配置。
真实来源启用仍是独立授权步骤。

云端已通过独立保护配置维护采用 `medium`，8192输出／32768单轮限制保持。当前配置副本
在原生Windows、合成消息和实际模型上完成16轮简报／原文／保存／更正／忘记／重连流程；
这不是生产数据或真实私人QQ验收。手工读回仍发现把来源日误当成接收整理日的表述，回复
也偏长，完整目标保持未完成。正常记忆查询未命中的中性提示另有Windows预览入口；
范围见交付验收和终端说明。

从现有终端的 `/sources` 查看接入状态。N/E/S/G 分别审阅来源登记、导入保存、有限同步和
当前会话阅读；没有授权时保持关闭。完整操作见[消息简报体验步骤](docs/DAILY_BRIEFING.md)，
现状与验证限制见[交付验收](docs/DAILY_BRIEFING_ACCEPTANCE.md#current-usable-deployment--verified-2026-11-07)。

## 从这里阅读

统一终端：`lain` / `lain desk --conversation <id>`，可用 `--portrait <本地字符画文件>`
载入选定头像。包含可编辑 Chat、Worker Dashboard、对话关联的远程派工、权限检查与事实
回传。详见[终端设计与操作](docs/TERMINAL_DESIGN.md)和[设备共同工作](docs/DEVICE_WORK.md)。
派工确认可核对原始请求、实际发送文本和 Changed/Unchanged；本机可显式运行
`lain-device-agent --check --directory <私有设备目录>` 检查前置条件，不会启动服务，
也不把登记或旧报告当成在线。任务详情 F6 可向已完成／失败的原 Worker 会话追加指令，
F8 可检查取消目标；两者都须明确确认，聊天草稿独立保留。权限状态区分云端决定、设备
报告收到与 Worker 报告确认。本地 [W4 验收](docs/W4_ACCEPTANCE.md)已完成，用户已确认
真实 Windows Terminal 的视觉效果。终端会提示已知缺失的聊天配置并保留草稿；配置恢复后
需明确发送。云端常驻、作用域记忆、启动与运行时同步及真实 HTTPS 两机确定性闭环已交付，
见[云端阶段验收](docs/CLOUD_ACCEPTANCE.md)和[运维指南](docs/CLOUD_OPERATIONS.md)。
云端现已显式启用 DeepSeek Flash，并通过一轮真实短对话；本机有独立云端入口，
未覆盖原本地实例或迁移个人数据。真实 QQ 来源、Worker 故障恢复及多日相处仍单独验收。
启动方式见[云端终端入口](docs/CLI.md#cloud-chat-entry--2026-11-05)。

| 文档 | 内容 |
| --- | --- |
| [产品定义与边界](docs/PRODUCT.md) | 用户确认的方向、平等关系、体验与开放问题 |
| [云端与设备设计](docs/RESIDENT_DESIGN.md) | QQ 双账号、数据归属、档案、授权、断线恢复 |
| [云端阶段验收](docs/CLOUD_ACCEPTANCE.md) | 已实现／部署／验证与真实日常使用剩余条件 |
| [云端运维](docs/CLOUD_OPERATIONS.md) | 资源归属、有限保留、升级、备份恢复与卸载 |
| [聊天与日常活动](docs/CHAT_DESIGN.md) | 人格、时机、自然回忆、纠正、主动联系 |
| [QQ 私聊操作指南](docs/QQ_CHANNEL.md) | 配置、授权、恢复、部署边界与真实联调 |
| [Q1 实现验收](docs/Q1_IMPLEMENTATION.md) | 本轮实现范围、离线证据与未完成验收 |
| [Q2-A 验收与实测](docs/Q2_A_ACCEPTANCE.md) | 连接诊断修补、私聊记忆、权限边界和分阶段人工验证 |
| [MaiBot 参考研究](docs/MAIBOT_REFERENCE.md) | 已阅读的机制、适合借鉴之处与未验证范围 |
| [架构](docs/ARCHITECTURE.md) | 新目标与当前本地实现的区别 |
| [领域模型](docs/DOMAIN_MODEL.md) | 对话、工作、记忆及拟议实体边界 |
| [运行模型](docs/RUNTIME_MODEL.md) | 生命周期、活动与恢复语义 |
| [记忆](docs/MEMORY.md) | Phase 7.1 契约与 Q2-A 受限私聊记忆 |
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

Rust workspace 有四个 crate：

- `lain-kernel`：Kernel 库、`laind` 和独立 `lain-device-agent`；服务端 Actor 与原生 Worker 监督分开启动。
- `lain-client`：`lain` CLI/TUI，经 HTTP/SSE 访问 Kernel。
- `lain-wire`：公共强类型 ID 和数据结构。
- `lain-local`：CLI 与设备服务共用的本机私有文件／权限工具。

`web/` 是保留的 React/TypeScript 客户端。终端开发不要求 npm。
`legacy/v0` 保存旧原型，仅供历史参考；本轮调整不恢复其架构。

原生服务需要独立配置并显式启动；远端 API 连接本身不启用设备执行。

## Development

The workspace contains `lain-kernel` (library, `laind`, `lain-device-agent`),
`lain-client` (`lain`, HTTP/SSE), shared `lain-wire` DTOs and `lain-local` private
native file helpers. It supervises Codex workers with durable sessions,
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

Q2-B1 adds separately authorized, bounded private-history source search/inspect over
existing channel records, with actual source IDs, grant revision/window, honest
delivery state and content-free read audits. It defaults off and cannot approve
Work or restore forgotten memory. See [Q2-B1 acceptance](docs/Q2_B1_ACCEPTANCE.md).
