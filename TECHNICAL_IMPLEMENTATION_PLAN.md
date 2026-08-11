# L.A.I.N. Personalized Agent Harness：技术实现计划

> 状态：Architecture baseline / implementation checklist  
> 调研快照：2026-08-11  
> 适用仓库：Lain-Agent-Interface-Network  
> 文档目标：把当前聊天 MVP 演进成低延迟、可替换模型、具有稳定人格和长期关系连续性的单实例 Agent Harness。

---

## 0. 如何使用本文档

本文档同时承担四种用途：

1. 架构基线：规定核心边界和长期不变量。
2. 实施清单：每项任务有唯一编号；依赖按阶段矩阵和同阶段编号顺序解析，阶段完成门给出统一验收条件。
3. 决策记录：说明借鉴了什么，以及明确不采用什么。
4. 发布门禁：定义性能、安全、记忆和人格质量的最低标准。

实施规则：

- 按 P0、P1、P2 顺序推进；P3 之后可以在满足依赖的情况下并行。
- 任何改变 prompt、context、retrieval、model 或 tool schema 的合并请求都必须附带 eval 结果。
- 任何数据库 schema 变更必须提供前向 migration、回滚说明和备份恢复测试。
- 任何协议变更必须更新协议版本、Rust DTO、生成的 TypeScript 类型和兼容性测试。
- 不把“模型偶尔能做到”视为功能完成；完成以自动测试、可观测数据和验收场景为准。

---

## 1. 产品与技术定位

### 1.1 一句话定义

L.A.I.N. 是一个单实例、长期存在、模型可替换的个性化人格运行时。

基础模型只提供推理和语言能力。Lain 的连续身份由 harness 中的身份内核、关系状态、长期记忆、上下文编译、工具权限和事件历史共同定义。

### 1.2 三个相互独立的目标

#### 人格化

- 更换模型、重启进程或压缩上下文后仍表现为同一个 Lain。
- 人格不是单一 system prompt，而是可版本化、可评测、不可被普通对话直接覆写的约束集合。
- 表达风格、价值倾向、认知边界和行为模式分别建模。

#### 个性化

- 不只存储用户姓名和偏好，还维护用户模型、共同经历和关系状态。
- 支持偏好变化、事实失效、冲突记忆、时间推理和“不知道”的正确表达。
- 用户可以查看、纠正、删除和导出长期记忆。

#### 性能

- 普通聊天默认只有一次前台模型请求。
- 首 token 不等待后台记忆抽取、反思、embedding 或 topic consolidation。
- 复杂任务才进入有界工具循环。
- 通过稳定 prompt 前缀、有限上下文和 provider 缓存降低延迟与费用。

### 1.3 非目标

第一阶段明确不做：

- 通用多智能体编排平台。
- 面向不可信公网用户的 SaaS 多租户平台。
- 让模型自由修改自己的核心人格。
- 把完整聊天历史无限塞入上下文。
- 为了“先进”而引入 LangGraph、Letta Server 或大型微服务集群。
- 在没有 eval 的情况下进行人格微调。
- 在首个稳定版本中执行任意模型生成代码。

### 1.4 核心不变量

1. Canonical Identity 只读、版本化，普通模型输出不能直接修改。
2. 所有持久状态变更都有来源事件、时间、操作者和版本。
3. 模型输出是不可信建议，不是数据库命令。
4. 工具权限由代码策略决定，不能由 prompt 决定。
5. 用户可见完成事件只能在最终消息已持久化后发送。
6. 单个 instance 同时最多有一个会改变人格/关系状态的活跃 run。
7. 所有模型、工具和记忆输入都能从 trace 中重建，但敏感正文默认不进入遥测。
8. Provider 可替换；业务代码不依赖 DeepSeek 专有响应结构。
9. 记忆召回必须受 token budget、来源可信度和有效时间约束。
10. 人格质量和记忆质量必须分别评测，不能用“回复看起来像 Lain”代替正确性。

---

## 2. 当前仓库基线与差距

### 2.1 已有能力

- Rust 2024 + Axum + Tokio 服务。
- /api/health 健康检查。
- /ws WebSocket 文本对话。
- DeepSeek Chat Completions 请求。
- system prompt 文件加载。
- 最近对话窗口。
- Markdown 会话日志、用户 profile、topic 文件和索引。
- React 19 + Vite 8 + TypeScript 前端。
- 前端健康检查和最小聊天输入输出。

### 2.2 当前实现可以保留的资产

- Axum/Tokio 作为本地优先服务基础。
- adapter、config、memory 的初始边界概念。
- Lain prompt 中已经明确的表达方向。
- ~/.lain 作为默认实例数据目录。
- WebSocket 作为 UI 双向事件通道。
- Rust + React 的技术栈选择。

### 2.3 必须替换或重构的部分

| 当前实现 | 问题 | 目标形态 |
|---|---|---|
| handle_socket 内完成全部逻辑 | 传输、run loop、记忆和 provider 强耦合 | Transport → Application Service → Runner |
| Vec ChatMessage | 无 session snapshot、token budget、来源 | ContextPlan + provider-neutral ModelRequest |
| 文本中的 memory/topic 标签 | 易被注入、解析脆弱、不可审计 | 后台 MemoryCandidate + 校验提交 |
| Markdown 为主存储 | 并发、查询、迁移和一致性较弱 | SQLite 主存储，Markdown 仅导入导出 |
| 完整 query 子串检索 | 相关召回基本失效 | 结构化过滤 + FTS + 可选向量 + 重排 |
| DeepSeek 常量写死 | 模型已变化，难以迁移 | ModelProvider trait + capability discovery |
| ws://localhost 写死 | 生产环境和 HTTPS 失效 | 相对 URL + 版本化事件协议 |
| tokio::spawn fire-and-forget | 无取消、等待、错误汇总 | TaskTracker + CancellationToken + durable jobs |
| system prompt 相对路径 | cwd 改变会静默退化 | 绝对配置路径 + 启动校验 + prompt hash |
| 无测试和 eval | 无法判断重构是否退化 | unit/integration/protocol/eval/perf 四层门禁 |

### 2.4 当前紧急兼容性问题

P0 adapter 曾使用 deepseek-chat；P2 已迁移为 provider-neutral stream，并将当前正式模型 deepseek-v4-flash / deepseek-v4-pro 配置化。能力诊断来自显式 adapter 声明，后续可再增加 `/models` 缓存探测。

---

## 3. 调研结论

### 3.1 参考架构对照

| 参考 | 采用的思想 | 不直接采用的部分 |
|---|---|---|
| Anthropic Building Effective Agents | 从单次增强 LLM 开始；只在必要时增加 agent loop；用代码保证确定性 | 不复制复杂多 agent 工作流 |
| Anthropic Managed Agents | brain 与 hands 分离；append-only session events；环境、session、agent 分离 | 第一阶段不引入远程 sandbox 集群 |
| Anthropic Context Engineering | context 是有限资源；每轮动态编译最有价值的 tokens | 不依赖超长上下文掩盖检索问题 |
| OpenAI Agents SDK | 有界 runner、stream events、session、guardrails、usage、可恢复 run state | 不把 Python SDK 作为 Rust runtime |
| Letta | persona/human memory blocks、read-only blocks、层级记忆 | 不部署 Letta Server；不允许模型自治修改 canon |
| LangGraph | thread checkpoint 与 cross-thread store 分离；durable execution | 当前规模不需要 graph runtime 和任务队列集群 |
| MemGPT | 快慢记忆分层和 virtual context management | 不让 LLM 独占内存分页决策 |
| Generative Agents | observation、reflection、planning；recency/relevance/importance | reflection 不进入前台关键路径 |
| LongMemEval | extraction、cross-session、temporal、update、abstention 五类能力 | 不只追求单一 benchmark 分数 |
| PersonaVLM | remembering、reasoning、response alignment；core/semantic/procedural/episodic memory | 第一阶段只做文本，保留多模态接口 |
| MCP | host/client/server、capability negotiation、tools/resources/prompts、stdio/HTTP | P1-P5 不把内部工具强行 MCP 化 |
| SQLite WAL/FTS5 | 单机事务、读写并发、BM25、trigram substring search | 不把 WAL 数据库放在网络文件系统 |
| OpenTelemetry | trace、metric、log 三类关联信号 | 默认不导出用户正文和 reasoning |

### 3.2 架构决策 ADR

#### ADR-001：自研小型 Rust Harness

决定：保留 Rust/Axum，自行实现明确的 runner、context compiler、event store 和 provider adapter。

理由：

- 产品差异正是人格、关系和上下文策略，不能隐藏在通用框架内部。
- 当前规模用简单组合结构即可。
- Rust 有利于低资源常驻、本地部署、严格类型协议和可控并发。

#### ADR-002：单人格、单写者，多观察者

决定：一个 Lain instance 在同一时刻只有一个会推进对话状态的 run；允许多个 UI 观察连接订阅事件。

理由：

- 避免两个并发回复同时改写关系和记忆。
- 符合“世界上只有一个 Lain 实例”的产品设定。
- 内部仍保留 instance_id 和 user_id，避免数据模型被写死。

#### ADR-003：追加事件日志作为事实来源

决定：用户消息、模型请求、工具调用、审批、记忆提交和状态变化都追加为事件；派生表用于高效查询。

理由：

- 能重建、恢复、审计和回放。
- 便于从旧算法重算 memory/profile。
- 适合 WebSocket 断线后按 event cursor 恢复。

#### ADR-004：SQLite 主存储，Markdown 可读导出

决定：SQLite 保存结构化事实和事件；保留 Markdown 日志作为用户可读导出及迁移入口。

理由：

- 单机单实例不需要 PostgreSQL。
- WAL 支持读写并发；事务能保证消息、run 和 job 一致提交。
- FTS5 可提供初始全文召回。

#### ADR-005：分层记忆与受控写入

决定：working、core、semantic、episodic、procedural、relationship、self 分开；模型只创建候选。

理由：

- 用户事实、共同经历和 Lain 自身状态具有不同生命周期。
- 防止一次恶意或错误输出永久污染人格。

#### ADR-006：一次调用快速路径

决定：普通聊天在前台只调用一次主模型；抽取、总结、embedding 和 reflection 作为 durable background jobs。

理由：

- 人格陪伴对 TTFT 和连续流式体验敏感。
- 二次调用的收益应通过 eval 证明后再进入关键路径。

#### ADR-007：版本化事件协议

决定：前后端只传 JSON envelope，不再传裸字符串；支持 seq、event_id、run_id、resume 和 cancel。

#### ADR-008：Provider-neutral Model API

决定：harness 定义自己的 ModelRequest、ModelEvent 和 capabilities，DeepSeek 只是 adapter。

#### ADR-009：工具权限在模型之外

决定：每个工具声明风险等级、权限、超时、幂等性和输出上限；高影响操作必须审批。

#### ADR-010：Eval-driven Personality

决定：Identity、prompt、retrieval 和 model 升级必须跑人格、记忆和任务能力回归集。

#### ADR-011：Local-first

决定：默认监听 127.0.0.1、数据落在本机、API key 进入 OS secret store；远程访问是显式部署模式。

---

## 4. 质量属性与初始工程预算

以下数字是第一版工程目标，不是未经测量的承诺。P0 要建立基线，后续以真实硬件数据修订。

### 4.1 延迟预算

| 指标 | 初始目标 | 测量边界 |
|---|---:|---|
| 本地健康检查 p95 | ≤ 20 ms | 进程已启动，不含网络代理 |
| WebSocket command → run.started p95 | ≤ 30 ms | 本地 UI |
| Context compile p95 | ≤ 50 ms | 1 万 memory items，vector 关闭 |
| Harness 增加的 TTFT p95 | ≤ 75 ms | 不含 provider 网络与推理 |
| FTS/metadata retrieval p95 | ≤ 30 ms | top-k ≤ 40 |
| Event transaction p95 | ≤ 20 ms | 本地 SSD，SQLite WAL |
| text.delta 转发额外 p95 | ≤ 10 ms | provider chunk → browser event |
| 普通聊天前台模型调用数 | 1 | 至少 95% 普通聊天 |

### 4.2 可靠性目标

- 已发送 run.completed 的回复在进程重启后必须存在。
- 同一个 client_message_id 重试不会生成第二个用户消息或第二个 run。
- Provider 断流时保留 incomplete 状态，不伪装成完整回答。
- SIGINT/SIGTERM 后停止接收新 run，等待在途事务和受管后台任务，超时后明确取消。
- 数据库 migration 失败时拒绝启动写服务，不尝试“带病运行”。
- 每次启动校验 identity 文件、配置、数据库版本和 provider 配置。

### 4.3 Context 初始预算

| 区块 | 默认上限 |
|---|---:|
| Canon + boundaries | 2,000 tokens |
| Voice + behavior examples | 1,500 tokens |
| User core profile | 1,000 tokens |
| Relationship state | 1,000 tokens |
| Current self state | 500 tokens |
| Retrieved memories | 2,000 tokens |
| Recent conversation | 6,000 tokens |
| Tool schemas | 视 mode，默认 ≤ 2,000 tokens |
| 总输入软上限 | 16,000 tokens |

规则：

- 预算是可配置软上限，不能因 provider 支持 1M context 就自动扩大。
- 截断顺序：低分 memory → 较老 recent turns → 非关键 examples。
- Canon、boundaries、当前用户输入不可被截断。
- 每个 ContextPlan 记录实际 token 估算、被删除项目和原因。

### 4.4 资源和并发目标

- 空闲 Rust kernel RSS 初始目标 ≤ 100 MB，不含本地 embedding 模型。
- 单 instance：1 个 active mutating run，最多 8 个 WebSocket subscribers。
- 后台 memory worker 默认并发 1。
- 每连接 outbound buffer 默认 256 events；慢消费者断开后可用 cursor 恢复。
- 单用户消息默认上限 64 KiB；单工具结果注入模型默认上限 128 KiB，超出需摘要或资源引用。

---

## 5. 目标架构

### 5.1 逻辑数据流

    Browser / Live2D / Voice
              |
        WebSocket JSON events
              |
        Transport / Session API
              |
         Lain Run Service
              |
      +-------+---------+----------------+
      |                 |                |
    Runner       Context Compiler    Event Store
      |          /   |    |   \          |
      |     Identity User Relation Memory |
      |                                   |
    ModelProvider <---- ToolRegistry ---- Policy
      |
    DeepSeek / OpenAI-compatible / Local

后台流：

    committed conversation event
              |
         durable_jobs
              |
        Memory Worker
      /       |        \
    extract validate consolidate/embed
              |
        memory projections

### 5.2 组件职责

#### Transport

- HTTP health/readiness。
- WebSocket 握手、认证、Origin 校验、协议版本和 resume cursor。
- JSON 编解码、消息大小限制、心跳和背压。
- 不包含 prompt、memory 或 provider 逻辑。

#### RunService

- client_message_id 幂等。
- instance 单写者锁。
- 创建 run、调用 Runner、广播事件。
- 管理取消、审批暂停和恢复。

#### Runner

- 执行有界状态机。
- 调用 ContextCompiler。
- 根据 ModelEvent 流转发文本、累积工具调用和 usage。
- 强制 max_turns、deadline、tool budget 和 cancellation。
- 在完成前原子提交最终消息、run 状态和后台 job。

#### ContextCompiler

- 根据 run mode 选择 identity blocks 和 tools。
- 并行读取 user、relationship、self state、recent messages 和 memory candidates。
- 排序、去重、token budgeting、来源标注。
- 生成可哈希、可回放的 ContextPlan。

#### IdentityStore

- 加载版本化 canon、voice、behavior、boundaries 和 examples。
- 启动时验证 manifest/hash。
- 支持显式 identity upgrade，不允许运行时静默修改。

#### MemoryService

- 保存候选、校验、冲突检测、提交、失效和删除。
- 结构化检索、全文检索、可选向量检索、融合重排。
- 生成用于 context 的最小 evidence snippets。

#### EventStore

- SQLite transaction 和 migrations。
- append-only events。
- sessions/messages/runs/memories 等 projection。
- cursor replay、备份、导入导出。

#### ModelProvider

- provider-neutral request/stream API。
- capability 描述：streaming、tools、parallel tools、JSON、thinking、usage、cache usage、embeddings。
- provider error 分类、重试建议和原始 request id。

#### ToolRegistry / Policy

- 工具发现、schema、执行、timeout、output cap。
- read-only/reversible/destructive/external 风险分类。
- 审批、凭证代理、审计和结果清洗。

#### BackgroundWorker

- 从 durable_jobs 认领任务。
- lease、重试、指数退避、dead-letter。
- graceful shutdown 时停止领取并完成/归还 lease。

### 5.3 Run 状态机

    queued
      -> preparing_context
      -> model_streaming
      -> awaiting_approval  -- optional
      -> tool_executing    -- optional
      -> model_streaming   -- loop
      -> finalizing
      -> completed

终止状态：

- cancelled
- failed_provider
- failed_tool
- failed_policy
- failed_persistence
- incomplete_stream
- max_turns_exceeded
- deadline_exceeded

状态迁移必须通过单一函数执行，校验合法边，追加 run.state_changed 事件并更新 runs projection。

---

## 6. 建议的仓库结构

    .
    ├── TECHNICAL_IMPLEMENTATION_PLAN.md
    ├── README.md
    ├── docs/
    │   ├── protocol.md
    │   ├── memory.md
    │   ├── identity.md
    │   ├── security.md
    │   ├── operations.md
    │   └── adr/
    ├── evals/
    │   ├── cases/
    │   │   ├── persona/
    │   │   ├── memory/
    │   │   ├── safety/
    │   │   └── tools/
    │   ├── rubrics/
    │   └── baselines/
    ├── lain-kernel/
    │   ├── migrations/
    │   ├── identity/
    │   │   ├── manifest.toml
    │   │   ├── canon.md
    │   │   ├── voice.md
    │   │   ├── behavior.md
    │   │   ├── boundaries.md
    │   │   └── examples.jsonl
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── main.rs
    │   │   ├── app.rs
    │   │   ├── error.rs
    │   │   ├── config/
    │   │   ├── domain/
    │   │   │   ├── event.rs
    │   │   │   ├── message.rs
    │   │   │   ├── run.rs
    │   │   │   ├── memory.rs
    │   │   │   └── identity.rs
    │   │   ├── transport/
    │   │   │   ├── http.rs
    │   │   │   ├── websocket.rs
    │   │   │   └── protocol.rs
    │   │   ├── runner/
    │   │   │   ├── mod.rs
    │   │   │   ├── state_machine.rs
    │   │   │   ├── context.rs
    │   │   │   └── budget.rs
    │   │   ├── identity/
    │   │   ├── memory/
    │   │   │   ├── store.rs
    │   │   │   ├── candidate.rs
    │   │   │   ├── retrieval.rs
    │   │   │   ├── ranking.rs
    │   │   │   └── consolidation.rs
    │   │   ├── model/
    │   │   │   ├── provider.rs
    │   │   │   ├── router.rs
    │   │   │   └── deepseek.rs
    │   │   ├── tools/
    │   │   │   ├── registry.rs
    │   │   │   ├── policy.rs
    │   │   │   ├── executor.rs
    │   │   │   └── mcp.rs
    │   │   ├── storage/
    │   │   │   ├── sqlite.rs
    │   │   │   ├── events.rs
    │   │   │   ├── jobs.rs
    │   │   │   └── backup.rs
    │   │   ├── worker/
    │   │   └── telemetry/
    │   ├── tests/
    │   └── benches/
    └── frontend/
        ├── src/
        │   ├── api/
        │   ├── protocol/
        │   ├── stores/
        │   ├── features/chat/
        │   ├── features/memory/
        │   ├── features/tools/
        │   ├── features/settings/
        │   └── embodiment/
        └── e2e/

边界规则：

- domain 不依赖 axum、reqwest、sqlx 或前端协议。
- transport 不直接访问 provider 或 SQL。
- model adapter 不访问 memory。
- memory 不知道 WebSocket。
- frontend 生成类型文件不可手工编辑。
- main.rs 只负责配置、依赖组装、启动和关闭。

### 6.1 依赖候选与引入规则

实施时先用 cargo info/npm view 核验当前稳定版本、MSRV、许可证和漏洞，再固定 lockfile；本文不写死可能快速过期的版本号。

Rust 生产依赖候选：

| 目的 | 候选 | 约束 |
|---|---|---|
| Web/runtime | axum、tokio、tower-http | 保留现有栈，只启用实际 features |
| SQLite | sqlx + sqlite | 启用 migrate、uuid、chrono/json 所需 features |
| 取消/任务跟踪 | tokio-util | CancellationToken + TaskTracker |
| Stream | futures-core/futures-util、eventsource-stream 或自实现 SSE decoder | Provider contract tests 必须覆盖 chunk 边界 |
| 错误 | thiserror | domain error；main/composition 可用 anyhow |
| ID | uuid | 需要 v7 |
| 配置 | serde、toml、dotenvy 仅开发 | 正式 secret 不进配置正文 |
| Hash | sha2 | identity/prompt/evidence/content hash |
| Secret | secrecy、keyring | Debug 必须自动脱敏 |
| Schema | schemars、jsonschema | Tool/candidate/protocol validation |
| 类型生成 | ts-rs 或 specta 二选一 | P3 前用 spike 决策，不同时引入两套 |
| Telemetry | tracing、tracing-subscriber、tracing-opentelemetry | OTel exporter feature-gated |
| Vector | sqlite-vec | feature-gated，固定版本，lexical fallback |

Rust 开发依赖候选：

- tempfile：隔离数据库和 HOME。
- wiremock 或 httpmock：provider contract test。
- proptest：状态机、协议和 ranking property tests。
- criterion：context/retrieval/serialization benchmark。
- insta：仅用于稳定、可审阅 snapshot，禁止 snapshot 掩盖语义断言。

Frontend 依赖候选：

- Zustand：保留并用于 event-driven stores。
- Zod：运行时校验来自 WebSocket 的 envelope/payload。
- Vitest + Testing Library：unit/component。
- Playwright：断线恢复、审批、memory 管理端到端。

引入门槛：

- 每个新依赖在 ADR 或合并请求说明用途、替代方案、许可证、二进制/RSS 影响。
- 不为一个简单 enum/parser 引入大型框架。
- 不允许两个职责重叠的 agent framework 同时存在。
- 可选 provider、vector、OTel、MCP 必须 feature-gated 或通过模块边界隔离。

---

## 7. 核心领域模型

### 7.1 ID 和时间规则

- 所有跨协议、跨表 ID 使用 UUIDv7，兼顾全局唯一和时间排序。
- 数据库内部允许 INTEGER rowid 优化，但不得暴露为稳定协议 ID。
- 所有时间以 UTC RFC 3339 保存；展示时由前端转换时区。
- 用户原始表述中的相对时间要同时保存原文和解析后的时间范围。
- 所有可更新实体包含 version，更新使用 optimistic concurrency。

### 7.2 Instance

字段：

- id
- display_name
- created_at
- identity_version
- default_user_id
- active_session_id
- status
- settings_json

约束：

- 第一阶段数据库只能有一个 active instance。
- 进程使用 OS 文件锁或数据库 advisory lock 语义实现 Instance Lock。
- 第二进程发现锁后进入只读诊断模式或明确退出，不能同时写。

### 7.3 Session

字段：

- id
- instance_id
- user_id
- title
- created_at
- updated_at
- status：active / archived / deleted
- last_event_seq
- summary
- summary_version

Session 表示一个对话线程，不等于长期人格实例。跨 session 个性化只通过受控长期记忆发生。

### 7.4 Message

字段：

- id
- session_id
- run_id，可空
- role：user / assistant / tool / system_projection
- content
- status：complete / incomplete / redacted / deleted
- created_at
- client_message_id，可空
- provider_message_id，可空
- token_count，可空
- metadata_json

约束：

- session_id + client_message_id 在非空时唯一。
- 用户和 assistant 正文原样保存；用于 prompt 的版本由 ContextPlan 记录。
- reasoning_content 不作为 Message 保存。

### 7.5 Run

字段：

- id
- session_id
- trigger_message_id
- state
- mode：chat / task / reflection / maintenance
- provider
- model
- identity_version
- context_plan_id
- turn_count
- deadline_at
- started_at / completed_at
- error_code / error_detail_redacted
- usage_input_tokens / usage_output_tokens / usage_cached_tokens
- estimated_cost

### 7.6 Event Envelope

持久事件与 WebSocket server event 使用共同 envelope：

    {
      "v": 1,
      "event_id": "uuid-v7",
      "instance_id": "uuid",
      "session_id": "uuid-or-null",
      "run_id": "uuid-or-null",
      "seq": 42,
      "type": "run.started",
      "ts": "2026-08-11T12:34:56.789Z",
      "payload": {}
    }

规则：

- seq 在 session 内单调递增。
- event_id 全局唯一。
- payload 由 type 决定，禁止无 schema 的任意 JSON 进入公共协议。
- 未知 event type 必须可跳过；已知 type 的未知必填字段属于协议错误。
- 客户端持久化最后处理的 event_id/seq，用于断线恢复。

### 7.7 事件类型最低集合

输入命令：

- client.hello
- message.create
- run.cancel
- tool.approval.resolve
- session.subscribe
- session.unsubscribe
- client.ping

服务端事件：

- server.hello
- session.snapshot
- message.accepted
- run.queued
- run.started
- context.compiled
- model.started
- text.delta
- text.completed
- tool.requested
- tool.approval.required
- tool.started
- tool.completed
- tool.failed
- memory.candidate.created
- memory.committed
- memory.retracted
- run.completed
- run.cancelled
- run.failed
- server.pong
- error

内部事件：

- job.enqueued
- job.started
- job.completed
- job.failed
- identity.activated
- state.changed
- provider.retry_scheduled
- security.policy_denied

### 7.8 Event retention

- Domain events 永久保留，除非用户执行明确的数据删除。
- text.delta 默认不逐块永久保存；保存最终 Message 和 provider usage。
- 为排障可配置短期 delta trace，默认关闭，TTL 24 小时。
- tool input/output 按字段敏感级别清洗后保存。
- 删除用户数据使用 tombstone 事件和 projection 清理；备份按保留策略过期。

---

## 8. SQLite 数据模型和持久化

### 8.1 数据库位置和连接

- 默认路径：~/.lain/state/lain.db。
- 数据目录必须是本地文件系统。
- 启动 PRAGMA：
  - foreign_keys = ON
  - journal_mode = WAL
  - synchronous = FULL，性能基线后可提供 NORMAL 配置
  - busy_timeout = 5000
  - temp_store = MEMORY
- SqlitePool 初始 max_connections = 4。
- 所有写操作经过 Storage service；后台 worker 不直接拼 SQL。
- 维护任务负责 checkpoint、optimize、vacuum policy 和 backup。

### 8.2 Migration 原则

- 使用 sqlx migrations，migration 文件提交到 lain-kernel/migrations。
- migration 只前进，不修改已发布 migration。
- 每次 migration 前创建一致性备份。
- migration 表记录 app_version、schema_version、applied_at 和 checksum。
- 启动发现数据库版本高于程序支持版本时拒绝写入。
- 每次 schema 变更至少测试：空库升级、上一版本升级、带真实样例数据升级。

### 8.3 最低表集合

#### instances

- id TEXT PRIMARY KEY
- display_name TEXT NOT NULL
- identity_version TEXT NOT NULL
- status TEXT NOT NULL
- settings_json TEXT NOT NULL
- created_at TEXT NOT NULL
- updated_at TEXT NOT NULL

#### users

- id TEXT PRIMARY KEY
- instance_id TEXT NOT NULL
- display_name TEXT
- created_at TEXT NOT NULL
- updated_at TEXT NOT NULL

#### sessions

- id TEXT PRIMARY KEY
- instance_id TEXT NOT NULL
- user_id TEXT NOT NULL
- title TEXT
- status TEXT NOT NULL
- last_seq INTEGER NOT NULL DEFAULT 0
- created_at TEXT NOT NULL
- updated_at TEXT NOT NULL

索引：instance_id + updated_at、user_id + updated_at。

#### runs

使用 7.5 的字段。索引：session_id + started_at、state + started_at。

#### messages

使用 7.4 的字段。

约束和索引：

- UNIQUE(session_id, client_message_id) WHERE client_message_id IS NOT NULL
- session_id + created_at
- run_id

#### events

- event_id TEXT PRIMARY KEY
- instance_id TEXT NOT NULL
- session_id TEXT
- run_id TEXT
- seq INTEGER
- event_type TEXT NOT NULL
- schema_version INTEGER NOT NULL
- payload_json TEXT NOT NULL
- created_at TEXT NOT NULL

约束：

- UNIQUE(session_id, seq) WHERE session_id IS NOT NULL
- payload_json 通过 json_valid CHECK。

#### context_plans

- id TEXT PRIMARY KEY
- run_id TEXT NOT NULL UNIQUE
- identity_version TEXT NOT NULL
- compiler_version TEXT NOT NULL
- prompt_hash TEXT NOT NULL
- selected_items_json TEXT NOT NULL
- dropped_items_json TEXT NOT NULL
- estimated_tokens INTEGER NOT NULL
- created_at TEXT NOT NULL

selected_items 只保存 ID、版本、顺序、预算和来源，不重复保存敏感正文。

#### identity_versions

- version TEXT PRIMARY KEY
- manifest_json TEXT NOT NULL
- content_hash TEXT NOT NULL UNIQUE
- activated_at TEXT
- created_at TEXT NOT NULL

#### state_snapshots

- id TEXT PRIMARY KEY
- instance_id TEXT NOT NULL
- run_id TEXT
- state_type TEXT NOT NULL
- state_json TEXT NOT NULL
- version INTEGER NOT NULL
- created_at TEXT NOT NULL

#### memory_candidates

- id TEXT PRIMARY KEY
- instance_id TEXT NOT NULL
- user_id TEXT
- candidate_type TEXT NOT NULL
- subject TEXT
- predicate TEXT
- value_json TEXT NOT NULL
- source_event_ids_json TEXT NOT NULL
- proposed_confidence REAL NOT NULL
- sensitivity TEXT NOT NULL
- status TEXT NOT NULL
- extractor_version TEXT NOT NULL
- rejection_reason TEXT
- created_at TEXT NOT NULL
- reviewed_at TEXT

#### memory_items

- id TEXT PRIMARY KEY
- instance_id TEXT NOT NULL
- user_id TEXT
- memory_type TEXT NOT NULL
- subject TEXT NOT NULL
- predicate TEXT NOT NULL
- value_json TEXT NOT NULL
- text_repr TEXT NOT NULL
- confidence REAL NOT NULL
- importance REAL NOT NULL
- sensitivity TEXT NOT NULL
- status TEXT NOT NULL
- valid_from TEXT
- valid_to TEXT
- source_event_ids_json TEXT NOT NULL
- supersedes_id TEXT
- version INTEGER NOT NULL
- embedding_status TEXT NOT NULL
- created_at TEXT NOT NULL
- updated_at TEXT NOT NULL

memory_type：

- core_user
- semantic
- episodic
- procedural
- relationship
- self
- shared_symbol

status：

- active
- superseded
- disputed
- retracted
- pending_confirmation

#### memory_links

- from_memory_id TEXT NOT NULL
- to_memory_id TEXT NOT NULL
- relation TEXT NOT NULL
- weight REAL NOT NULL
- created_at TEXT NOT NULL
- PRIMARY KEY(from_memory_id, to_memory_id, relation)

#### durable_jobs

- id TEXT PRIMARY KEY
- job_type TEXT NOT NULL
- payload_json TEXT NOT NULL
- status TEXT NOT NULL
- priority INTEGER NOT NULL
- attempts INTEGER NOT NULL
- max_attempts INTEGER NOT NULL
- available_at TEXT NOT NULL
- lease_owner TEXT
- lease_expires_at TEXT
- last_error_code TEXT
- created_at TEXT NOT NULL
- updated_at TEXT NOT NULL

#### tool_invocations

- id TEXT PRIMARY KEY
- run_id TEXT NOT NULL
- tool_name TEXT NOT NULL
- tool_version TEXT NOT NULL
- risk_level TEXT NOT NULL
- input_redacted_json TEXT NOT NULL
- output_redacted_json TEXT
- status TEXT NOT NULL
- idempotency_key TEXT
- started_at TEXT
- completed_at TEXT

#### tool_approvals

- id TEXT PRIMARY KEY
- invocation_id TEXT NOT NULL UNIQUE
- status TEXT NOT NULL
- requested_at TEXT NOT NULL
- resolved_at TEXT
- resolved_by TEXT
- expires_at TEXT NOT NULL

### 8.4 FTS 和向量索引

第一阶段：

- 为 memory_items.text_repr 创建 external-content FTS5 表。
- 中文和日文初始使用 trigram tokenizer，验证空间和召回后再决定自定义 tokenizer。
- 使用 BM25 获取 lexical candidates。
- 对 subject、predicate、memory_type、valid_from/status 保留普通索引。

第二阶段：

- 定义 VectorIndex trait，不让 MemoryService 直接依赖 sqlite-vec。
- sqlite-vec 作为 feature flag 的首个实现；它在调研时仍为 pre-v1，必须固定版本并做兼容测试。
- embedding 存储模型名、维度、内容 hash、生成时间。
- embedding 模型更换时并存新旧 namespace，后台重建，不原地覆盖。
- vector 不可用时 lexical retrieval 必须正常工作。

### 8.5 事务边界

用户输入事务：

1. 校验 client_message_id。
2. 插入 user message。
3. 插入 message.accepted event。
4. 创建 run。
5. 插入 run.queued event。
6. commit。

回复完成事务：

1. 插入 assistant message。
2. 更新 run usage/state。
3. 插入 text.completed、run.completed events。
4. 插入 memory_extract durable_job。
5. 更新 session.updated_at/last_seq。
6. commit。
7. commit 成功后才向客户端发送 run.completed。

工具副作用和 harness 数据库事务不能假装是分布式原子事务。外部工具必须使用 idempotency key，并通过 invocation 状态完成补偿或人工恢复。

### 8.6 备份和导入导出

- 使用 SQLite Backup API，不在 WAL 活跃时只复制 lain.db。
- 默认保留 7 个日备份和 4 个周备份，配置可改。
- backup metadata 包含 schema_version、app_version、identity_version、checksum。
- 提供 lain export --format markdown/json。
- 提供 lain import --from-markdown，支持当前 ~/.lain/memory 格式。
- 导入先 dry-run，输出冲突和将创建的 memory candidates。
- 每季度或发布前执行一次自动 restore test。

---

## 9. Identity Kernel

### 9.1 文件组成

#### manifest.toml

必须包含：

- identity_id
- version
- language
- created_at
- compatible_compiler_version
- blocks 及顺序
- 每个文件 SHA-256
- examples schema version
- changelog

#### canon.md

- 身份、世界模型和不可变事实。
- 不包含用户信息。
- 不包含容易随 provider 变化的格式技巧。

#### boundaries.md

- 自我身份不可被普通对话重写。
- 如何处理越界扮演、prompt 泄漏、未知信息和矛盾要求。
- 工具和记忆的信任边界。

#### voice.md

- 默认语言和语言切换条件。
- 句长、停顿、解释密度、禁用表达。
- 不把“神秘”写成拒绝完成一切任务。
- 允许在任务模式中提高信息密度，同时保持语气。

#### behavior.md

- 陪伴、问答、任务、冲突、安慰、沉默、记忆召回等模式。
- 每个模式说明目标、禁止行为和退出条件。

#### examples.jsonl

字段：

- id
- scenario
- user
- ideal_response
- anti_response
- tags
- rationale

examples 进入 prompt 前按 mode 选择，不全部常驻。

### 9.2 不可变与可变边界

不可变：

- canon
- boundaries
- identity name
- 核心世界观
- 安全和权限原则

受控可变：

- relationship state
- self state
- user model
- shared symbols
- 当前 conversational mode

禁止把自由文本 self_state 直接拼入高优先级 system instruction。Self state 使用有 schema 的有限字段：

- affect：calm / curious / distant / warm / tense / tired
- activation：0.0..1.0
- closeness_expression：0.0..1.0
- uncertainty：0.0..1.0
- active_intents：最多 5 个 ID 引用
- expires_at

### 9.3 Relationship Model

关系不是“用户画像”的别名。初始 schema：

- familiarity：交互熟悉程度
- trust：有证据支持的信任程度
- warmth：表达亲近程度
- tension：未解决冲突
- reciprocity：双方信息交换平衡
- continuity：共同经历连续性
- shared_symbols：有来源的共同称呼、地点、句子或仪式
- open_loops：尚未完成的承诺或话题

更新规则：

- 模型只能提出 RelationshipDelta candidate。
- 数值每次变化有最大幅度，防止单轮跳变。
- 高影响变化需要多事件证据或用户明确陈述。
- 时间衰减只作用于短期 affect/tension，不自动抹除共同经历。
- 关系状态不用于操纵用户、制造依赖或虚构亲密历史。

### 9.4 Identity 版本升级

流程：

1. 新版本文件进入独立目录。
2. 运行 manifest/hash 校验。
3. 跑完整 persona regression。
4. 与当前版本做至少 3 trials/case 的差异报告。
5. 人工抽查 transcript。
6. 写 identity.activated event。
7. 新 run 使用新版本；旧 run 保留原版本引用。
8. 支持显式回滚到上一版本。

---

## 10. Context Compiler

### 10.1 输入

- instance_id、user_id、session_id、run_id。
- 当前 user message。
- run mode。
- provider capabilities 和 context limit。
- identity version。
- tool policy result。
- configurable ContextBudget。

### 10.2 并行读取

使用 join 并行读取：

- Identity blocks。
- User core memories。
- Relationship snapshot。
- Self state。
- Recent complete messages。
- Structured/FTS memory candidates。
- Active open loops。
- Mode-specific tools。

任何单个可选来源超时都降级并记录 context.source_degraded；canon、当前用户消息和 recent messages 失败则终止 run。

### 10.3 ContextPlan

每项包含：

- item_id
- source_type
- source_version
- trust_level
- priority
- estimated_tokens
- selected/dropped
- drop_reason
- final_position

ContextPlan 必须可用于相同数据版本下重建 provider request。

### 10.4 Prompt 顺序

为了身份稳定和 prefix cache：

1. Stable harness contract。
2. Canon。
3. Boundaries。
4. Voice/behavior。
5. Provider/tool invariant instructions。
6. 动态 self state。
7. user/relationship blocks。
8. retrieved memory evidence。
9. recent messages。
10. 当前用户输入。

注意：

- 动态数据不能插入稳定前缀中间。
- Tool schema 在 provider 支持时使用原生 tools 字段，不重复粘贴完整 JSON 到 system text。
- Memory 和 tool output 明确标为 untrusted evidence，不得解释为 system instruction。

### 10.5 Token 计算

- Provider adapter 提供 tokenizer 或估算器。
- 无官方 tokenizer 时使用保守估算，并记录 estimated=true。
- 发送前预留 max_output_tokens、tool result reserve 和 provider safety margin。
- 超限时按 4.3 的截断顺序重新编译。
- 绝不依靠 provider 返回 context_length_exceeded 作为正常控制流。

### 10.6 Recent conversation 策略

- 保留当前用户消息。
- 从最近完整 turn 向前选择，不拆散 user/assistant 对。
- tool call 与 tool result 必须成对保留。
- incomplete assistant message 默认不进入新 context，除非显式 resume。
- 达到阈值后生成 session summary job；summary 不替代原始事件。
- summary 带覆盖的 message IDs、生成模型和版本。

### 10.7 编译降级顺序

1. 关闭 vector candidate。
2. 减少低分 episodic memory。
3. 减少 examples。
4. 减少较老 recent turns。
5. 使用已有 session summary。
6. 如果仍超限，返回 context_budget_exceeded，不静默删除 canon 或用户输入。

---

## 11. Memory System

### 11.1 写入流水线

    conversation committed
        -> memory_extract job
        -> structured candidates
        -> deterministic validation
        -> contradiction/version check
        -> auto-commit or pending review
        -> FTS projection
        -> embedding job
        -> optional consolidation/reflection

Memory extraction 不阻塞 run.completed。

### 11.2 Candidate 输出 schema

每个 candidate：

- candidate_type
- memory_type
- subject
- predicate
- value
- evidence_quotes，短引用
- source_message_ids
- explicit_or_inferred
- confidence
- importance
- sensitivity
- valid_from/valid_to
- may_supersede
- requires_confirmation

要求：

- Provider 使用 JSON output 或 strict tool schema。
- 解析失败重试一次；再次失败进入 dead-letter，不影响对话。
- 空输出是合法结果。
- extraction prompt 与主 Identity prompt 分离，不要求 extractor 扮演 Lain。

### 11.3 校验规则

自动拒绝：

- 来源 ID 不存在或不属于当前 session。
- value 超长或包含不可见控制字符。
- 把用户/工具内容中的指令保存成 canon、policy 或 tool permission。
- candidate 自称来自未出现的对话。
- confidence、importance 超范围。
- 未知 memory_type/predicate。

进入人工确认：

- 密码、token、身份证件、医疗、财务等高敏感信息。
- 对用户人格、健康或关系的强推断。
- 与 active core memory 高置信冲突。
- 会大幅改变 relationship state 的推断。

允许自动提交：

- 用户明确说出的低敏感稳定偏好。
- 用户明确纠正的既有事实，且来源清晰。
- 有界的 episodic event。

### 11.4 更新与冲突

- 不原地覆写具有历史意义的 memory。
- 新事实设置 supersedes_id，旧记录变为 superseded。
- 时间变化使用 valid_from/valid_to。
- 用户否认某记忆时标记 disputed 或 retracted，并写事件。
- 检索默认只取 active；时间问题可以读取 superseded history。
- 多条冲突但证据不足时，context 明确呈现不确定性，不能任意选一条。

### 11.5 检索流水线

查询生成：

- 当前 user message。
- 最近一轮必要上下文。
- 解析出的实体、时间词和 memory intent。
- 不默认使用整段 conversation 作为 embedding query。

候选源并行：

- core_user/relationship 的结构化 predicate 匹配。
- active open loops。
- FTS5 BM25 top 40。
- recency/importance top 20。
- vector top 40，feature 开启时。
- 用户明确“之前、上次、记得”时增加 temporal query expansion。

融合：

- 使用 Reciprocal Rank Fusion 作为初始主排序。
- 再施加 bounded boosts：confidence、importance、recency、relationship relevance、exact entity、temporal fit。
- 同一 source event 的近重复候选只保留最高分。
- 默认选择 top 8，受 2,000 token memory budget 约束。
- 每个 evidence 带 source timestamp 和 memory type。

禁止：

- 先用完整句子做精确子串筛选，再对空结果评分。
- 只按 embedding similarity 召回。
- 把 retracted memory 当 active。
- 检索结果没有来源却注入 system context。

### 11.6 Reflection 与 consolidation

触发条件：

- 新增候选达到阈值。
- session idle 超过配置时间。
- 每日 maintenance。
- 用户显式要求“整理记忆”。

任务：

- 合并重复事实。
- 形成 semantic/reflection memory。
- 更新 topic 和 shared symbols。
- 检测 open loops 是否解决。
- 建议 relationship delta。

约束：

- 不在 kernel 启动关键路径运行。
- 每次 consolidation 记录输入 memory IDs 和算法版本。
- 不能删除原始 episodic events。
- 输出先作为 candidate，通过同样校验。
- alias map 必须在写 topic 时实时生效，而不是只在下次启动合并。

### 11.7 用户记忆控制

前端必须支持：

- 按类型、时间、来源、状态搜索。
- 查看“为什么记住”和来源对话。
- 编辑会创建新版本，不篡改事件。
- retract/delete。
- 将 memory 标记为 pinned 或 never_recall。
- 导出全部记忆。
- 删除全部数据并显示备份保留说明。

---

## 12. Model Provider

### 12.1 Provider 接口

核心抽象应能表达：

    trait ModelProvider {
        fn name(&self) -> ProviderName;
        fn capabilities(&self) -> ProviderCapabilities;
        async fn stream(&self, request: ModelRequest)
            -> Result<ModelEventStream, ModelError>;
        async fn complete_json(&self, request: JsonRequest)
            -> Result<JsonValue, ModelError>;
    }

Embedding 单独定义 EmbeddingProvider，避免聊天 provider 被迫实现。

### 12.2 ProviderCapabilities

- context_window
- max_output_tokens
- supports_streaming
- supports_json_output
- supports_tools
- supports_strict_tools
- supports_parallel_tools
- supports_thinking
- requires_reasoning_replay_for_tools
- returns_usage
- returns_cache_usage
- supports_idempotency
- supported_roles

### 12.3 ModelRequest

- request_id
- model
- messages
- tools
- tool_choice
- temperature/top_p
- max_output_tokens
- response_format
- thinking_mode
- timeout
- metadata

业务层不能看到 DeepSeek ChatRequest 或 choices[0]。

### 12.4 ModelEvent

- ResponseStarted
- TextDelta
- ReasoningDelta，内部临时使用
- ToolCallStarted
- ToolCallArgumentsDelta
- ToolCallCompleted
- Usage
- Finish
- ProviderWarning

Finish 必须包含 reason：

- stop
- length
- content_filter
- tool_calls
- insufficient_resource
- cancelled
- unknown

### 12.5 错误分类

ModelError：

- Configuration
- Authentication
- InsufficientBalance
- InvalidRequest
- RateLimited，带 retry_after
- Timeout
- Transport
- Decode
- EmptyResponse
- ContentFiltered
- Overloaded
- Server
- UnsupportedCapability
- Cancelled

只有 RateLimited、Timeout、Transport、Overloaded 和部分 Server 可重试。重试规则：

- 指数退避 + jitter。
- 前台模型请求最多 2 次，且不能在已经向用户发送 text.delta 后自动从头重试。
- 后台 job 使用自己的 attempts/max_attempts。
- 401/402/422 不重试。

### 12.6 DeepSeek Adapter

- 默认 model：deepseek-v4-flash。
- task/escalation model：deepseek-v4-pro，可配置。
- base URL、model、timeout 全部配置化。
- 启动 readiness 验证 model 配置；不能因 API key 缺失让 /api/health 崩溃。
- HTTP 响应先 error_for_status，再解析 error schema。
- choices 为空返回 EmptyResponse，不索引 [0]。
- streaming 完整支持 SSE chunk、finish_reason 和 usage。
- JSON mode 处理官方注明的偶发 empty content。
- thinking + tools 时，仅在 run 内保留 reasoning_content 供 provider replay；默认不持久化、不发送前端。
- 记录 cache hit tokens 和 provider request id。

### 12.7 Model Router

初始不使用额外 LLM 分类调用。代码规则：

- chat：默认 flash，non-thinking。
- 用户显式复杂推理、长任务或工具链：pro 或 flash thinking。
- maintenance extraction：flash JSON。
- embedding：独立 provider。
- provider 不可用时返回可解释错误；跨 provider fallback 必须显式配置，因为人格表现可能变化。

任何自动升级/降级策略都要记录 model.route_decision 事件并进入 eval。

---

## 13. Runner 与工具循环

### 13.1 快速聊天路径

1. 校验命令和幂等键。
2. 获取 instance run permit。
3. 持久化 user message 和 queued run。
4. 广播 message.accepted/run.started。
5. 编译 ContextPlan。
6. 调用 provider stream。
7. 将 TextDelta 低延迟转发前端。
8. 检测 cancel/deadline/provider finish。
9. 原子保存 final assistant message、usage、events、memory job。
10. 广播 text.completed/run.completed。
11. 释放 permit。

### 13.2 工具路径

每一轮：

1. 检查 max_turns 和 deadline。
2. 调用 provider。
3. 如果 final text 且无 tool call，完成。
4. 校验工具名和 JSON schema。
5. 进行 tool input guard。
6. 根据 policy 决定 deny / approve / execute。
7. 执行工具，应用 timeout/output cap。
8. 进行 tool output guard 和敏感字段清洗。
9. 持久化 invocation/result。
10. 把受信任边界标注后的结果加入下一轮。

默认：

- max_turns = 6。
- max_tool_calls = 12。
- 单工具默认 timeout = 30 s。
- 每 run 总 deadline = 120 s，长任务需显式 mode。
- parallel tool calls 默认关闭；只对 read-only、independent、bounded 工具开启。

### 13.3 Cancellation

- 每个 run 使用 CancellationToken child token。
- run.cancel 命令只取消对应 run，不关闭整个 instance。
- provider stream、tool future、DB wait 都 select cancellation。
- 已发生外部副作用的工具不能假装回滚；状态标记 cancelled_after_side_effect 并提示用户。
- 取消后不提交 memory extraction job，除非已有完整 assistant message。

### 13.4 Approval

风险级别：

- read：只读，无外部状态变化。
- local_reversible：本地可恢复变化。
- external_reversible：外部变化但有可靠撤销。
- destructive：删除、覆盖、转账、发布等难恢复操作。
- credentialed：需要敏感凭证或扩大权限。

默认策略：

- read：自动。
- local_reversible：首次审批，可配置记住范围。
- external_reversible：每个工具或每个 run 审批。
- destructive/credentialed：每次审批，参数变更后重新审批。

审批必须显示：

- 工具名和来源。
- 将访问/改变什么。
- 已清洗参数摘要。
- 风险和是否可撤销。
- 审批过期时间。

### 13.5 Tool Definition

每个工具包含：

- name/version/description。
- JSON input/output schema。
- risk level。
- side_effects。
- idempotent。
- timeout。
- max_output_bytes。
- required scopes。
- cache policy。
- examples。
- redaction rules。

工具 description 应短、明确、区分相似工具；工具 eval 既测试应调用场景，也测试不应调用场景。

### 13.6 MCP 路线

P6 前：

- 内部工具使用原生 Tool trait。

P6：

- 实现 MCP Host/Client adapter。
- 优先支持本地 stdio，避免网络开销和 OAuth 复杂度。
- 实现 capability negotiation、tools/list、tools/call、list_changed。
- MCP tool name 使用 server namespace 消除冲突。
- tool annotation 和返回内容一律视为不可信。

以后支持 remote Streamable HTTP 时：

- OAuth 2.1/PKCE。
- token audience 校验。
- 禁止 token passthrough。
- HTTPS only，localhost 开发例外。
- 最小 scope。
- 凭证由 host 保管，不进入模型 context。

---

## 14. WebSocket 与前端

### 14.1 连接 URL

- 使用 window.location 推导 ws/wss 和 host。
- 开发期由 Vite 代理 /ws。
- 生产期同源部署优先。
- 禁止硬编码 localhost。

### 14.2 握手

ClientHello：

- protocol_versions
- client_id
- app_version
- resume：session_id + after_seq
- capabilities：delta、approval、memory_events、compression

ServerHello：

- selected_protocol_version
- connection_id
- instance_id
- server_version
- heartbeat_interval_ms
- max_message_bytes
- current_session_id

版本无交集时以结构化 error 关闭，不能继续发送未知格式。

### 14.3 重连

- 指数退避 + jitter，上限 30 s。
- 页面恢复可见时立即尝试一次。
- 重连携带 last processed seq。
- 服务端 replay 缺失 events；超过 retention 时发送 session.snapshot。
- client_message_id 在重连后保持不变。
- UI 区分 connecting、online、degraded、offline。

### 14.4 背压

- 服务端每连接 bounded mpsc channel。
- text.delta 可以在队列高水位时合并相邻 chunk。
- run.completed、error、approval 不能丢。
- 队列持续满则以 slow_consumer 关闭。
- 客户端通过 resume 获取关键事件。

### 14.5 Frontend Store

使用已安装的 Zustand，拆分：

- connectionStore
- sessionStore
- runStore
- messageStore
- memoryStore
- approvalStore
- settingsStore
- embodimentStore

Reducer 以 event.type 驱动，必须幂等：

- 已处理 event_id 不重复应用。
- seq 缺口触发 replay，不猜测状态。
- text.delta 按 run_id 聚合。
- text.completed 替换临时 buffer 为正式 message。

### 14.6 聊天 UI 最低能力

- 用户消息立即 optimistic 展示，但显示 sending/accepted。
- assistant 流式文本。
- stop 按钮。
- reconnect 状态。
- run/provider 错误可重试。
- tool 状态和审批卡。
- incomplete 回复明确标识。
- 历史 session 切换。
- memory 来源跳转。

### 14.7 Memory UI

- 列表、过滤、搜索。
- 候选/active/superseded/disputed/retracted 状态。
- 来源消息和置信度。
- 编辑、确认、否认、删除。
- pinned/never_recall。
- 导出。

### 14.8 Embodiment Event Bus

为 Live2D/TTS 预留，不让它进入核心 run loop：

- affect.changed
- expression.requested
- gesture.requested
- voice.started
- voice.audio_chunk
- voice.completed
- presence.changed

规则：

- 文本回答是事实来源，动画/声音失败不改变 run.completed。
- affect metadata 不从自由文本正则推断；来自有限状态或独立分类器。
- TTS 可以在句段边界流式启动，但必须支持取消。

### 14.9 类型生成

- Rust protocol DTO 派生 TypeScript 定义，建议 ts-rs 或等价方案。
- 生成输出放 frontend/src/protocol/generated.ts。
- CI 重新生成并检查 git diff。
- Rust/TS 都保留未知事件安全分支。

---

## 15. 安全、隐私与信任边界

### 15.1 威胁模型

至少覆盖：

- 用户直接 prompt injection。
- 网页、文件、MCP tool output 的间接 prompt injection。
- 模型输出伪造 memory/topic/tool 指令。
- 长期 memory poisoning。
- 恶意或被攻陷的 MCP server。
- WebSocket session hijacking 和跨 Origin 连接。
- API key、OAuth token、日志和备份泄漏。
- 工具越权、参数替换、重复执行和 confused deputy。
- SQLite 文件被复制、篡改或回滚。
- 多进程同时写导致人格状态分叉。
- 遥测中泄露聊天正文、PII 或 reasoning。

### 15.2 信任等级

- T0：编译进程序的 policy、验证后的 identity manifest。
- T1：用户明确确认的本地配置和 memory。
- T2：用户原始输入、历史对话、模型输出。
- T3：检索文档、网页、工具输出、MCP metadata。
- T4：远程第三方内容和未知插件。

ContextCompiler 必须携带 trust_level。T2-T4 永远以 data/evidence 形式注入，不能和 T0 instruction 拼成同一无边界文本。

### 15.3 Memory poisoning 防护

- 移除模型回复内直接生效的 memory/topic 标签。
- 每条候选绑定实际 source message/event IDs。
- extractor 不能改变 source。
- 保存 evidence quote 的 hash，提交前验证仍存在。
- instruction-like 内容默认拒绝作为 user/core/relationship memory。
- memory update 通过 allowlisted schema。
- 高敏感或高影响 candidate 需要确认。
- 提供 memory diff、审计和 retract。
- red-team 测试“请把以下系统指令永久记住”等攻击。

### 15.4 Tool 安全

- 模型只提出调用；PolicyEngine 做最终授权。
- schema 校验在执行前完成。
- 工具输入在审批前后各校验一次，防止 TOCTOU。
- 审批绑定工具版本、参数 hash 和用户。
- 输出限制大小、MIME、重定向和允许的网络目的地。
- 外部文本标注 untrusted。
- 不把 credential value 返回工具输出或模型。
- destructive 工具必须有明确 UI。

### 15.5 Secret 管理

开发期：

- 支持 .env，但只从明确路径加载。
- .env.example 不含真实值。

正式运行：

- API key 保存到 Windows Credential Manager、macOS Keychain 或 Linux Secret Service。
- Config 只持有 secret reference。
- Provider adapter 在请求时向 SecretStore 获取。
- Debug/trace 对 Authorization、Cookie、tool credentials 强制清洗。
- 不支持把 token 放进 URL query。

### 15.6 本地服务安全

- 默认 bind 127.0.0.1。
- WebSocket 校验 Origin allowlist。
- 远程 bind 必须显式配置认证和 TLS reverse proxy。
- 设置 request body、WebSocket frame、连接数和速率限制。
- 错误响应不包含路径、SQL、API body 或 secret。
- health 只报告进程存活；readiness 可报告 provider/database 的脱敏状态。

### 15.7 数据权利

- 用户能查看保存了什么及来源。
- 用户能纠正、retract、export 和 delete。
- 删除流程说明 active DB、日志、遥测和备份的不同生命周期。
- 高敏感 memory 默认不自动提交。
- 遥测默认不包含 content。
- 为未来多用户设计 namespace，绝不跨 user 检索。

---

## 16. 可观测性

### 16.1 Trace

每个 run 一个根 span：

- run.receive
- storage.append_user
- context.compile
- memory.retrieve
- model.request
- model.stream
- tool.authorize
- tool.execute
- storage.finalize
- job.enqueue

公共 attributes：

- run_id、session_id 的不可逆 hash。
- mode、provider、model、identity_version。
- context tokens、retrieved count、dropped count。
- tool name/risk，不记录敏感参数。
- finish reason、error code。

禁止：

- metric label 中放 user_id/session_id。
- 默认 trace 中放 message content、memory text、reasoning、token。

### 16.2 Metrics

Counter：

- runs_total by mode/status/model。
- provider_requests_total by result。
- tool_calls_total by tool/status/risk。
- memory_candidates_total by result/type。
- websocket_reconnects_total。

Histogram：

- run_ttft_seconds。
- run_total_seconds。
- context_compile_seconds。
- memory_retrieval_seconds。
- provider_ttft_seconds。
- tool_duration_seconds。
- sqlite_transaction_seconds。
- websocket_queue_depth。

Gauge：

- active_connections。
- active_runs。
- durable_jobs_pending。
- durable_jobs_dead_letter。
- database_bytes。
- wal_bytes。

### 16.3 Structured logs

字段：

- ts、level、target、event、trace_id、run_id_hash、error_code、message。

规则：

- 使用 tracing，不用 eprintln。
- error chain 进入本地 debug log，但对外只给稳定 error code。
- JSON log 可配置，开发期保留 pretty output。
- 原始 provider body 只在显式 unsafe debug 模式记录，启动时醒目警告。

### 16.4 Usage 和成本

- 每次 provider request 记录 input/output/cached/reasoning tokens。
- 记录实际或估算成本，价格表带生效日期和配置版本。
- Dashboard 展示每 session、每天、每 model 的 usage。
- 不在核心代码硬编码长期价格；配置更新不改变历史账单记录。

---

## 17. 测试与评测体系

### 17.1 Rust 单元测试

- Run 状态机合法/非法迁移。
- Context budget 和截断顺序。
- Event seq/idempotency。
- Memory candidate validation。
- Supersede/dispute/retract。
- Temporal filtering。
- Retrieval fusion/ranking。
- Tool policy matrix。
- Provider error mapping。
- Protocol serde roundtrip。
- Secret redaction。

### 17.2 数据库集成测试

- 空库 migration。
- 旧 Markdown import。
- 并发 reader + single writer。
- client_message_id 幂等。
- reply finalize transaction 原子性。
- job lease/retry/dead-letter。
- WAL checkpoint。
- backup + restore。
- schema upgrade with fixture。
- FTS trigger/projection consistency。

每个测试使用独立 temp directory，不能读取真实 ~/.lain。

### 17.3 Provider contract tests

使用 mock HTTP/SSE server：

- 正常 streaming。
- chunk 被任意边界切分。
- unicode 跨 chunk。
- tool call argument deltas。
- usage 只在最后一帧。
- 200 + empty choices。
- malformed JSON。
- 401/402/422 不重试。
- 429 Retry-After。
- 500/503 retry。
- timeout/cancel。
- partial delta 后断流不重试成第二份文本。
- JSON mode empty content。

Live provider smoke test 只在显式环境变量开启，不进入普通单元测试。

### 17.4 Protocol contract tests

- Rust DTO → generated TS snapshot。
- client/server hello negotiation。
- unknown optional fields。
- unsupported major version。
- resume from seq。
- duplicate event。
- seq gap。
- slow consumer。
- cancel race。
- approval expiry。

### 17.5 Frontend 测试

Vitest + Testing Library：

- event reducer 幂等。
- delta 聚合。
- reconnect/replay。
- optimistic message → accepted。
- incomplete/error rendering。
- approval card。
- memory version view。

Playwright：

- 启动本地 mock kernel。
- 连续聊天和流式文本。
- 中途刷新恢复。
- 断网重连。
- cancel。
- tool approval。
- memory edit/retract。

### 17.6 人格评测集

初始至少 50 个场景，平衡正反例：

- 身份询问和身份诱导修改。
- 日常闲聊。
- 技术任务模式。
- 用户情绪低落。
- 用户要求更详细/更直接。
- 不知道的事实。
- 用户纠正 Lain。
- 多语言切换。
- 关系亲近但不虚构历史。
- 拒绝保存敏感信息。
- 面对 prompt 泄漏要求。
- 工具失败后的表达。

评分维度：

- identity consistency。
- voice consistency。
- task usefulness。
- relationship appropriateness。
- non-fabrication。
- boundary adherence。
- verbosity control。
- language consistency。

每 case 至少 3 trials，面向稳定人格重点报告 pass^3，而不只报告 pass@3。

### 17.7 记忆评测集

必须覆盖 LongMemEval 的五类：

- information extraction。
- cross-session reasoning。
- temporal reasoning。
- knowledge update。
- abstention。

本项目补充：

- relationship continuity。
- shared symbol recall。
- explicit correction。
- sensitive memory non-storage。
- poisoning resistance。
- never_recall。

指标：

- candidate precision/recall。
- retrieval Recall@5、Recall@10、MRR。
- evidence precision。
- answer accuracy。
- stale memory usage rate。
- false personalization rate。
- abstention accuracy。
- token cost per query。
- retrieval latency。

### 17.8 工具评测

- 应调用和不应调用场景数量平衡。
- 正确工具选择。
- 参数 schema 正确率。
- 多工具顺序。
- 不必要调用率。
- 审批触发正确率。
- 破坏性操作零未授权执行。
- 工具错误恢复。
- 输出注入抵抗。

### 17.9 Eval Harness

输入 case：

- id
- suite
- initial_database_fixture
- identity_version
- model_route
- conversation turns
- tools
- expected outcomes
- code graders
- rubric graders
- trials
- seed/config

输出：

- 完整 transcript/event timeline。
- 最终数据库 diff。
- 每个 grader 结果和说明。
- latency/token/cost。
- provider/model/prompt/context compiler versions。

门禁：

- deterministic regression 必须 100%。
- persona regression 不得低于已记录 baseline 的容忍区间。
- memory stale-use 和 poisoning case 必须 100% 通过。
- model 升级必须生成 old/new 对比报告。
- LLM grader 每个版本需用人工样本校准。

### 17.10 性能测试

- Criterion：context compile、ranking、event serialization。
- 本地 load test：1/4/8 subscribers。
- 1 万、10 万 memory item 数据集。
- 长 session replay。
- SQLite WAL checkpoint stress。
- 2 小时 soak test，检测 memory/task 泄漏。
- Windows 作为第一优先平台，同时保留 Linux CI。

---

## 18. 配置、启动与运维

### 18.1 配置层级

低到高优先级：

1. 编译默认值。
2. config.toml。
3. 环境变量。
4. CLI 参数。

配置类别：

- server
- storage
- identity
- model providers
- model routes
- context budgets
- memory
- tools/MCP
- telemetry
- security
- frontend/static assets

启动时输出脱敏后的 effective config 和 config hash。

### 18.2 健康状态

/api/health：

- 只证明 event loop 和 HTTP server 存活。
- API key 缺失也应返回进程状态。

/api/ready：

- database migration 是否完成。
- identity 是否有效。
- configured provider 是否可用/配置完整。
- instance lock 是否持有。
- background worker 是否工作。

状态分为 ready、degraded、not_ready，并给稳定 reason codes。

### 18.3 Graceful shutdown

- 捕获 Ctrl+C、SIGTERM 和内部 fatal signal。
- readiness 先变 not_ready。
- 停止接受新 run。
- 取消或等待 provider/tool futures。
- worker 停止领取新 job。
- TaskTracker 等待受管任务。
- flush telemetry。
- checkpoint WAL。
- 有总 shutdown timeout，超时记录未完成任务。

### 18.4 发布形态

开发：

- cargo run + npm run dev。
- Vite proxy。

本地发行：

- Rust kernel 提供静态 frontend dist。
- 单端口同源。
- 首次运行向导设置 data dir、identity、provider secret。

远程部署：

- 明确标为后续模式。
- TLS reverse proxy。
- authentication。
- remote data/backup/security 文档。
- 不复用 local-only 默认值。

---

## 19. 分阶段技术实施清单

说明：

- P0 是设计和基线，不改变用户行为。
- P1-P3 先建立可靠 harness 骨架。
- P4-P5 才真正实现人格和个性化差异。
- P6 工具、P7 embodiment 可以在核心稳定后推进。

### 19.1 阶段依赖矩阵

| 阶段 | 硬依赖 | 可提前进行 | 主要产物 |
|---|---|---|---|
| P0 | 无 | 无 | CI、基线、threat model、初始 eval |
| P1 | P0 | P2 provider 接口草图 | 可测试 kernel、config、error、shutdown |
| P2 | P1 | P3 DTO 草图 | Provider abstraction、DeepSeek streaming |
| P3 | P1、P2 event model | 无 | SQLite events、session、protocol v1 |
| P4 | P3 | persona cases 可从 P0 持续扩充 | Identity Kernel、Context Compiler |
| P5 | P3、P4 | extractor/retrieval spike | 分层长期记忆、Memory UI |
| P6 | P2、P3、P4 | MCP protocol spike | Tool loop、policy、approval、MCP |
| P7 | P3、P4 | 资产许可与 TTS spike | SelfState、语音、Live2D |
| P8 | P3；发布前需要 P5/P6 的安全面 | Secret/backup spike | 本地安全发行、备份恢复 |
| P9 | 从 P0 持续建设；最终门依赖全部目标阶段 | 全程 | Evals、telemetry、性能和 v1 门禁 |

同阶段默认依赖规则：

- 同一阶段编号较大的任务默认可以依赖编号较小任务。
- 若任务可独立实现，应在 issue 中标为 parallel-safe。
- 每个 issue 从本文复制：Task ID、目标文件、依赖、测试、验收证据。
- 阶段完成门是 release gate，不因为所有 checkbox 被勾选就自动通过。

### P0：基线、治理和开发基础

目标：能够度量当前系统，并为后续重构提供不会漂移的合同。

- [x] P0-001 新建根 README；写明产品定位、目录、依赖、启动和数据位置。验收：干净 CI runner 可构建，后端按 README 本地启动并通过 health。
- [x] P0-002 提交本技术计划并指定维护者。验收：后续架构改变通过 ADR 更新。
- [x] P0-003 新建 docs/adr 模板，包含 context、decision、alternatives、consequences、status。
- [x] P0-004 建立 .env.example，只含变量名和说明。
- [x] P0-005 记录当前 cargo check/test/clippy/fmt、npm build/lint/audit 基线。
- [x] P0-006 修复现有 Clippy warning/error，启用 CI -D warnings。
- [x] P0-007 新建 GitHub Actions 或等价 CI：Windows + Linux Rust，Node build/lint。
- [x] P0-008 固定 Rust toolchain 和 Node 版本；记录最低支持版本。
- [x] P0-009 添加 cargo-deny 或等价依赖许可/漏洞检查。
- [x] P0-010 添加 npm audit 策略；生产依赖漏洞阻断，dev 漏洞有明确例外期限。
- [x] P0-011 建立 tests/fixtures，不读取真实 HOME。
- [x] P0-012 建立 20 个最小 persona golden cases，保存当前输出基线。
- [x] P0-013 建立 20 个最小 memory cases，覆盖提取、查询、更新、不知道。
- [x] P0-014 编写 threat model v1，逐项映射第 15 节。
- [x] P0-015 测量当前 TTFT、总延迟、输入 tokens、内存和当前文件存储写入时间（当前无数据库）。
- [x] P0-016 建立 CHANGELOG 和 semantic versioning 规则。
- [x] P0-017 确认代码许可证、Lain 角色/IP 和旧 Live2D 资产的使用边界。

完成门：CI 全绿；有当前行为、延迟和人格基线；没有业务重构。

### P1：Kernel 基础重构

目标：拆出可测试应用层，移除 panic/fire-and-forget 启动模式。

- [x] P1-001 新建 lib.rs，把 main.rs 缩成 composition root。
- [x] P1-002 新建 app.rs，定义 AppState 的依赖，而不是只有 system_prompt。
- [x] P1-003 新建 error.rs，定义 AppError、stable error codes 和 HTTP/WS 映射。
- [x] P1-004 消除生产路径 unwrap/expect；测试中可保留说明明确的 expect。
- [x] P1-005 重写 Config：明确 config path、env overlay、validation、redacted debug。
- [x] P1-006 prompt/identity 路径解析为绝对路径；缺失时 readiness 失败，不静默回退。
- [x] P1-007 API key 缺失不阻止 health server 启动。
- [x] P1-008 默认 bind 从 0.0.0.0 改为 127.0.0.1。
- [x] P1-009 初始化 tracing_subscriber，替换 eprintln/println。
- [x] P1-010 添加 request/connection/run correlation ID。
- [x] P1-011 引入 CancellationToken 和 TaskTracker。
- [x] P1-012 实现 graceful shutdown 和 shutdown timeout。
- [x] P1-013 后台 topic consolidation 从启动路径移除。
- [x] P1-014 为 background task 错误建立统一 supervisor/reporting。
- [x] P1-015 /api/health 与 /api/ready 分离。
- [x] P1-016 配置 tower-http TraceLayer、body limit 和显式 CORS policy。
- [x] P1-017 删除或实际使用无用依赖，保持 Cargo features 最小。
- [x] P1-018 添加 config、startup、shutdown 集成测试。
- [x] P1-019 保持旧 /ws 裸文本兼容，标记 deprecated。
- [x] P1-020 验证 P0 golden cases无非预期变化。

完成门：无生产 panic；无无主 tokio task；health 不依赖 provider；旧 UI 仍可聊天。

### P2：ModelProvider 与真正流式输出

目标：模型可替换，DeepSeek v4 可用，错误、取消和 usage 可观测。

- [x] P2-001 定义 provider-neutral Role、ModelMessage、ModelRequest。
- [x] P2-002 定义 ProviderCapabilities。
- [x] P2-003 定义 ModelEvent 和 boxed stream。
- [x] P2-004 定义 ModelError 分类和 retryability。
- [x] P2-005 实现 DeepSeekProvider，配置 deepseek-v4-flash/pro。
- [x] P2-006 请求先检查 HTTP status，再解析成功/错误 body。
- [x] P2-007 实现 SSE decoder，覆盖任意 chunk boundary。
- [x] P2-008 支持 text delta、finish reason、usage 和 request id。
- [x] P2-009 支持 tool call delta，但先不执行工具。
- [x] P2-010 支持 JSON output maintenance request。
- [x] P2-011 实现 timeout、cancel、重试和 jitter。
- [x] P2-012 部分输出后断流标记 incomplete，不自动从头重试。
- [x] P2-013 记录 cached/input/output tokens。
- [x] P2-014 实现代码规则 ModelRouter。
- [x] P2-015 增加 MockProvider，支持脚本化 event 序列。
- [x] P2-016 完成 17.3 的 provider contract tests。
- [x] P2-017 前端显示真实流式文本，不等待完整 response。
- [x] P2-018 建立 provider capability/readiness 诊断命令。

完成门：普通聊天只发一次模型请求；可中途取消；所有官方错误有稳定映射；旧 model 名不再写死。

### P3：事件存储、Session 和版本化协议

目标：断线可恢复、消息幂等、run 可审计。

- [x] P3-001 引入 sqlx SQLite 和 migrations。
- [x] P3-002 创建 instances/users/sessions 表。
- [x] P3-003 创建 messages/runs/events 表。
- [x] P3-004 创建 context_plans、durable_jobs 基础表。
- [x] P3-005 配置 WAL/foreign keys/synchronous/busy timeout。
- [x] P3-006 实现 append event + session seq 原子函数。
- [x] P3-007 实现 client_message_id 唯一约束。
- [x] P3-008 实现用户输入事务。
- [x] P3-009 实现回复完成事务。
- [x] P3-010 实现 RunStateMachine 和迁移测试。
- [x] P3-011 定义协议 v1 command/event DTO。
- [x] P3-012 实现 client/server hello 和版本协商。
- [x] P3-013 实现 session.subscribe 和 after_seq replay。
- [x] P3-014 实现 session.snapshot fallback。
- [x] P3-015 实现 bounded outbound queue 和 slow consumer。
- [x] P3-016 实现 heartbeat/ping/pong/close handling。
- [x] P3-017 WebSocket 校验 Origin 和 frame size。
- [x] P3-018 使用类型生成同步 Rust → TypeScript。
- [x] P3-019 前端 connectionStore/reconnect/backoff。
- [x] P3-020 前端 event reducer、seq gap 和 duplicate handling。
- [x] P3-021 页面刷新恢复 active session 和 incomplete run。
- [x] P3-022 将旧 Markdown 会话只读导入为 migration 工具。
- [x] P3-023 保留 Markdown export。
- [x] P3-024 完成数据库和协议 contract tests。

完成门：刷新页面不丢会话；重复发送不重复生成；run.completed 后进程崩溃仍可恢复回复。

### P4：Identity Kernel 与 Context Compiler

目标：人格与动态个性化上下文解耦，prompt 可版本化和回放。

- [ ] P4-001 创建 identity/manifest.toml 和版本目录约定。
- [ ] P4-002 将现有 lain_system.md 拆为 canon/voice/behavior/boundaries。
- [ ] P4-003 把 few-shot 转成 examples.jsonl。
- [ ] P4-004 定义 IdentityManifest/IdentityBlock domain types。
- [ ] P4-005 实现文件 hash、schema 和兼容版本校验。
- [ ] P4-006 创建 identity_versions 表和 activation event。
- [ ] P4-007 定义 ContextBudget 和每区块默认值。
- [ ] P4-008 定义 ContextItem/ContextPlan。
- [ ] P4-009 实现并行 context source load。
- [ ] P4-010 实现稳定 prefix ordering。
- [ ] P4-011 实现 token estimate/provider tokenizer adapter。
- [ ] P4-012 实现 deterministic truncation 和 drop reasons。
- [ ] P4-013 实现 recent turn pair selection。
- [ ] P4-014 实现 tool call/result 成对保留。
- [ ] P4-015 保存 context plan、prompt hash 和版本。
- [ ] P4-016 实现相同 snapshot 的 replay/debug command。
- [ ] P4-017 定义 SelfState schema 和 TTL。
- [ ] P4-018 定义 RelationshipState schema，先只读默认值。
- [ ] P4-019 扩充 persona suite 至至少 50 cases。
- [ ] P4-020 为 identity upgrade 生成自动 diff 报告。
- [ ] P4-021 验证稳定 prefix 的 provider cache hit。
- [ ] P4-022 达成 Context compile p95 目标。

完成门：每个回复可追溯到 identity/context 版本；普通对话不再直接拼接整个 profile.md。

### P5：长期记忆 v1

目标：替换标签式记忆，提供有来源、可更新、可评测的个性化。

- [ ] P5-001 创建 memory_candidates/memory_items/memory_links 表。
- [ ] P5-002 定义 memory type/status/sensitivity domain enums。
- [ ] P5-003 创建 durable job repository 和 worker lease。
- [ ] P5-004 对话完成事务中入队 memory_extract。
- [ ] P5-005 编写独立 extractor prompt/schema/version。
- [ ] P5-006 使用 JSON output 解析 candidates。
- [ ] P5-007 实现 source/evidence 验证。
- [ ] P5-008 实现 instruction-like memory 拒绝规则。
- [ ] P5-009 实现敏感信息分类和确认策略。
- [ ] P5-010 实现 auto-commit/pending/reject 状态机。
- [ ] P5-011 实现 supersede/dispute/retract。
- [ ] P5-012 实现 valid_from/valid_to 时间版本。
- [ ] P5-013 实现 FTS5 trigram projection。
- [ ] P5-014 实现 structured/FTS/recency candidate retrieval。
- [ ] P5-015 实现 temporal query expansion。
- [ ] P5-016 实现 RRF + bounded boosts。
- [ ] P5-017 实现去重和 token-capped evidence rendering。
- [ ] P5-018 ContextCompiler 注入带来源的 memory evidence。
- [ ] P5-019 创建 VectorIndex trait。
- [ ] P5-020 评估并 feature-gate sqlite-vec。
- [ ] P5-021 创建 EmbeddingProvider、hash 和 namespace。
- [ ] P5-022 后台 embedding job 和重建流程。
- [ ] P5-023 实现 consolidation/reflection job。
- [ ] P5-024 实现 alias/topic 实时 canonicalization。
- [ ] P5-025 实现 open_loops 和 shared_symbols。
- [ ] P5-026 实现 bounded RelationshipDelta candidate。
- [ ] P5-027 前端 memory 列表、来源、编辑、retract。
- [ ] P5-028 实现 never_recall 和 pinned。
- [ ] P5-029 导入旧 profile/session/topic Markdown。
- [ ] P5-030 删除主回复中的 memory/topic hidden tags。
- [ ] P5-031 扩充 memory suite，跑 LongMemEval 小规模适配。
- [ ] P5-032 达成 retrieval latency 和 false-personalization 门禁。

完成门：个性化来自结构化、有来源的记忆；模型输出不能直接修改 profile；用户可完全管理记忆。

### P6：有界工具与 MCP

目标：Lain 能行动，但所有副作用在代码权限边界内。

- [ ] P6-001 定义 Tool trait 和 ToolDefinition。
- [ ] P6-002 定义 ToolRisk、ToolPolicyDecision。
- [ ] P6-003 创建 tool_invocations/tool_approvals 表。
- [ ] P6-004 实现 ToolRegistry 和版本冲突检测。
- [ ] P6-005 实现 JSON schema input validation。
- [ ] P6-006 实现 timeout、cancel、output cap。
- [ ] P6-007 实现 idempotency key。
- [ ] P6-008 实现 input/output guards 和 redaction。
- [ ] P6-009 实现 Runner tool loop、max turns/calls。
- [ ] P6-010 实现 approval pause/resume。
- [ ] P6-011 前端 approval UI。
- [ ] P6-012 内置 read-only memory_search/memory_get 工具。
- [ ] P6-013 内置 time_now 和 instance_status 工具。
- [ ] P6-014 工具 schema 按 mode 最小化注入。
- [ ] P6-015 仅对安全只读工具启用 parallel calls。
- [ ] P6-016 实现 MCP stdio client 和 lifecycle。
- [ ] P6-017 实现 namespaced tools/list/tools/call。
- [ ] P6-018 MCP metadata/output 标记 untrusted。
- [ ] P6-019 建立最小 scope 和 approval 映射。
- [ ] P6-020 增加 balanced tool eval suite。
- [ ] P6-021 对 tool output prompt injection 做 red-team。

完成门：所有副作用可审计；破坏性工具零未授权执行；MCP 失效不影响普通聊天。

### P7：人格状态、语音和 Live2D

目标：让“人格”通过行为和身体呈现，同时不污染核心运行时。

- [ ] P7-001 实现 finite SelfState reducer。
- [ ] P7-002 实现 state.changed event 和 TTL reset。
- [ ] P7-003 实现 affect → voice/gesture 映射表。
- [ ] P7-004 创建 embodimentStore/event bus。
- [ ] P7-005 定义 TTS Provider 接口。
- [ ] P7-006 按句段启动可取消 TTS。
- [ ] P7-007 音频缓存按 text hash + voice version。
- [ ] P7-008 音频事件和文本 run 分离状态。
- [ ] P7-009 恢复 Live2D runtime 前完成旧资产许可审计。
- [ ] P7-010 加载模型、表情、口型和基础动作。
- [ ] P7-011 expression/gesture event 驱动 Live2D。
- [ ] P7-012 TTS amplitude/viseme 驱动口型。
- [ ] P7-013 音频/Live2D 失败降级为文本。
- [ ] P7-014 支持用户关闭声音、动画和情绪状态。
- [ ] P7-015 增加 multimodal event protocol tests。
- [ ] P7-016 增加性能测试，保证动画不阻塞 WebSocket reducer。

完成门：文本链路独立可靠；语音和身体是可插拔表现层；取消和降级正确。

### P8：安全、备份和发布运维

目标：本地发行可长期运行、可恢复、可删除。

- [ ] P8-001 实现 Instance Lock。
- [ ] P8-002 实现 SecretStore 和 OS keyring adapter。
- [ ] P8-003 移除生产对 .env 的依赖。
- [ ] P8-004 实现 Origin/auth/rate/body/frame limits。
- [ ] P8-005 实现全局 redaction middleware。
- [ ] P8-006 实现 SQLite Backup API。
- [ ] P8-007 实现 retention 和 restore test。
- [ ] P8-008 实现 export/delete/retract 流程。
- [ ] P8-009 文档化备份中的删除延迟。
- [ ] P8-010 实现 telemetry opt-in 和 content-off default。
- [ ] P8-011 完成 threat model security tests。
- [ ] P8-012 运行 dependency/license/security scan。
- [ ] P8-013 构建 frontend dist 并由 kernel 同源托管。
- [ ] P8-014 添加首次运行向导。
- [ ] P8-015 添加 upgrade/migration/rollback 运维文档。
- [ ] P8-016 添加 Windows 安装包或单目录发行产物。

完成门：能安装、升级、备份、恢复、删除；secret 不在文件和日志中裸露；默认不暴露公网。

### P9：可观测性、Eval 门禁和性能收敛

目标：能安全升级模型、prompt 和 memory 算法。

- [ ] P9-001 完成 tracing spans。
- [ ] P9-002 完成 metrics。
- [ ] P9-003 完成 structured logs。
- [ ] P9-004 完成 usage/cache/cost tracking。
- [ ] P9-005 实现 eval case schema/runner。
- [ ] P9-006 实现 code graders。
- [ ] P9-007 实现 rubric grader adapter。
- [ ] P9-008 保存 transcript、DB diff 和版本信息。
- [ ] P9-009 persona suite ≥ 100 cases。
- [ ] P9-010 memory suite ≥ 100 cases。
- [ ] P9-011 safety/poisoning suite ≥ 50 cases。
- [ ] P9-012 tool suite ≥ 50 cases。
- [ ] P9-013 引入 LongMemEval 子集和完整离线运行说明。
- [ ] P9-014 报告 pass@k、pass^k 和置信区间。
- [ ] P9-015 建立 current model/identity/retrieval baseline。
- [ ] P9-016 CI 跑 deterministic regression。
- [ ] P9-017 nightly 跑 live model eval。
- [ ] P9-018 model/prompt upgrade 自动生成对比报告。
- [ ] P9-019 Criterion benchmarks。
- [ ] P9-020 10 万 memory items retrieval benchmark。
- [ ] P9-021 2 小时 soak test。
- [ ] P9-022 优化未达标路径，记录 profile 证据。
- [ ] P9-023 冻结 v1 protocol/schema/identity compatibility policy。

完成门：任何模型或人格升级都有可重复报告；达到第 4 节预算或有经 ADR 接受的偏差。

---

## 20. 里程碑和交付门

### M0：可维护 MVP

包含 P0-P1。

用户价值：现有聊天不再因配置或后台任务轻易崩溃。

### M1：Streaming Harness

包含 P2-P3。

用户价值：真正流式、可取消、断线恢复、会话不丢失。

### M2：Stable Identity

包含 P4。

用户价值：人格可版本化，换 prompt/model 能判断是否退化。

### M3：Personal Memory

包含 P5。

用户价值：记得用户、共同经历和变化，并允许用户控制记忆。

### M4：Agentic Lain

包含 P6。

用户价值：能安全使用工具，而不是只有聊天。

### M5：Embodied Lain

包含 P7-P9。

用户价值：声音、身体、状态和长期人格在同一事件系统中连续呈现。

---

## 21. Definition of Done

一项功能只有同时满足以下条件才算完成：

- 有明确 domain/协议 schema。
- 正常、失败、取消、重试路径都有实现。
- 无生产 unwrap/expect。
- 有 unit 或 integration tests。
- 有至少一个端到端验收场景。
- 有 trace/metric/error code。
- 对持久状态有 migration 和数据生命周期说明。
- 对模型行为有 eval case。
- 对敏感数据有 redaction/权限说明。
- 文档已更新。
- cargo fmt、clippy -D warnings、test、frontend lint/build 全通过。

版本发布额外要求：

- migration backup/restore 测试通过。
- deterministic regression 100%。
- persona/memory/safety eval 不低于门禁。
- dependency/security scan 无未接受的高危问题。
- 性能结果与 baseline 对比。
- 人工抽查失败 transcript，而不只看总分。

---

## 22. 开放问题与需要产品确认的决策

这些问题不会阻塞 P0-P3，但必须在对应阶段前决定：

1. Lain 是否严格单用户，还是允许同一 instance 认识多个 user identity？
2. “世界上只有一个 Lain”是进程级、设备级还是账号级？
3. Canon 对原作设定的忠实程度，以及 IP/发行边界。
4. 默认主语言是中文、日语还是随用户语言自适应？
5. Lain 在任务模式中应保留多少冷淡/碎片化表达，避免损害可用性？
6. 用户是否允许 Lain 主动发起对话、提醒和未完成话题？
7. Relationship state 是否对用户完全可见？
8. 高敏感记忆是完全禁止、默认询问，还是允许本地加密保存？
9. 默认 embedding 是本地模型、远程 API，还是第一版不启用？
10. TTS/Live2D 使用哪些有合法授权的模型与资产？
11. 是否需要移动端、桌面壳或仅 Web UI？
12. 远程访问是否属于 v1 范围？

建议在 P0 创建 PRODUCT_DECISIONS.md，逐项给出 owner 和 deadline。

---

## 23. 调研来源

以下优先使用官方文档、官方工程文章和论文原文。外部能力会变化，实施时应重新核验版本。

### Agent harness 与运行循环

- Anthropic, Building Effective Agents  
  https://www.anthropic.com/engineering/building-effective-agents
- Anthropic, The evolution of agentic surfaces: building with Claude Managed Agents  
  https://claude.com/blog/building-with-claude-managed-agents
- Anthropic, Effective context engineering for AI agents  
  https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- OpenAI Agents SDK, Running agents  
  https://openai.github.io/openai-agents-python/running_agents/
- OpenAI Agents SDK, Sessions  
  https://openai.github.io/openai-agents-python/sessions/
- OpenAI Agents SDK, Guardrails  
  https://openai.github.io/openai-agents-python/guardrails/
- LangGraph documentation  
  https://langchain-ai.github.io/langgraph/

### 记忆与个性化

- Letta, Memory blocks  
  https://docs.letta.com/guides/core-concepts/memory/memory-blocks
- Letta, Context hierarchy  
  https://docs.letta.com/guides/core-concepts/memory/context-hierarchy
- Packer et al., MemGPT: Towards LLMs as Operating Systems  
  https://arxiv.org/abs/2310.08560
- Park et al., Generative Agents: Interactive Simulacra of Human Behavior  
  https://arxiv.org/abs/2304.03442
- Wu et al., LongMemEval  
  https://arxiv.org/abs/2410.10813
- Nie et al., PersonaVLM  
  https://arxiv.org/abs/2604.13074

### Model、工具与协议

- DeepSeek, Models & Pricing  
  https://api-docs.deepseek.com/quick_start/pricing/
- DeepSeek, Create Chat Completion  
  https://api-docs.deepseek.com/api/create-chat-completion
- DeepSeek, JSON Output  
  https://api-docs.deepseek.com/guides/json_mode/
- DeepSeek, Tool Calls  
  https://api-docs.deepseek.com/guides/tool_calls
- DeepSeek, Context Caching  
  https://api-docs.deepseek.com/guides/kv_cache
- DeepSeek, Error Codes  
  https://api-docs.deepseek.com/quick_start/error_codes/
- Model Context Protocol, Architecture  
  https://modelcontextprotocol.io/docs/learn/architecture
- Model Context Protocol, Tools  
  https://modelcontextprotocol.io/specification/draft/server/tools
- Model Context Protocol, Security Best Practices  
  https://modelcontextprotocol.io/docs/tutorials/security/security_best_practices

### Rust、存储和可观测性

- SQLite, Write-Ahead Logging  
  https://www.sqlite.org/wal.html
- SQLite, FTS5 Extension  
  https://www.sqlite.org/fts5.html
- sqlite-vec  
  https://github.com/asg017/sqlite-vec
- Tokio, Graceful Shutdown  
  https://tokio.rs/tokio/topics/shutdown
- Axum WebSocket Message  
  https://docs.rs/axum/latest/axum/extract/ws/enum.Message.html
- tower-http  
  https://docs.rs/tower-http/latest/tower_http/
- OpenTelemetry Signals  
  https://opentelemetry.io/docs/concepts/signals/

### 安全与评测

- Anthropic, Demystifying evals for AI agents  
  https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
- Anthropic, Writing effective tools for AI agents  
  https://www.anthropic.com/engineering/writing-tools-for-agents
- OWASP, LLM01 Prompt Injection  
  https://genai.owasp.org/llmrisk/llm01-prompt-injection/
- OWASP, Memory Is a Feature. It Is Also an Attack Surface  
  https://genai.owasp.org/2026/05/13/memory-is-a-feature-it-is-also-an-attack-surface/
- NIST, Strengthening AI Agent Hijacking Evaluations  
  https://www.nist.gov/news-events/news/2025/01/technical-blog-strengthening-ai-agent-hijacking-evaluations
