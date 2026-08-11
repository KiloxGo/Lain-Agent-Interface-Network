# ADR-0004：版本化 Identity Kernel 与确定性 Context Compiler

- 状态：Accepted
- 日期：2026-08-11
- 维护者：KiloxGo

## 背景

P2 的普通 WebSocket 路径把单体 `lain_system.md` 与整份 `profile.md` 直接拼成 system message。它能验证 provider 连通性，却无法回答某条回复使用了哪个身份版本、哪些动态信息被选中、为什么某项被丢弃，也无法稳定控制 token、注入边界和 prefix cache。P3 已提供 `runs`、`context_plans` 与有序事件，为编译结果的持久化和回放诊断建立了事务边界。

本项目的产品差异来自“高性能且可个性化的人格化 harness”，因此身份资产、动态关系/状态和会话历史必须解耦；个性化不能以无界自由文本覆盖 canon、权限或事实身份。

## 决策

### Identity Kernel

身份资产进入版本目录 `lain-kernel/identity/v1`。`manifest.toml` 固定 identity ID、语种、语义版本、compiler 兼容版本、block 顺序、schema 版本、changelog 和每个文件的 SHA-256。当前 block 顺序为：

1. canon；
2. boundaries；
3. voice；
4. behavior；
5. examples。

启动时必须完整读取并验证 manifest、相对路径、block 非空、文件 hash、examples JSONL schema 和 ID 唯一性。校验失败时 identity readiness 失败，不允许 `/v1/ws` 接收 run。相同语义版本不得对应不同 content hash；成功切换版本时，SQLite 事务更新 active instance 并写 `identity.activated`。仓库内资产是项目自有的通用化 interface 描述，不包含或分发第三方角色素材。

稳定的首个 system message 只包含 harness contract 与 canon/boundaries/voice/behavior。examples 按 mode 确定性选择，动态 self/relationship/memory 不进入稳定前缀。DeepSeek 默认启用基于相同输入前缀的上下文缓存，且缓存是 best-effort；此排序既提高复用概率，也不把 cache hit 当正确性依赖。参考官方 [Context Caching](https://api-docs.deepseek.com/guides/kv_cache) 与 [Models & Pricing](https://api-docs.deepseek.com/quick_start/pricing)。

### Context Compiler

`ContextCompiler` 的输入为 identity、run/mode、当前完整 user message、recent complete messages、结构化 SelfState、只读 RelationshipState、带来源的 memory evidence、provider capability 和 `ContextBudget`。编译规则如下：

- canon 与当前用户消息是强制项，预算不足则显式失败；
- SelfState 仅允许枚举 affect、0..1 数值、最多 5 个 intent ID 和明确 TTL，过期即丢弃；
- RelationshipState 使用固定 0..1 字段和引用列表，P4 默认全零且只读；
- memory 必须带 item/version/source event IDs，以 `untrusted evidence` 注入，不能成为指令；
- recent history 从新到旧按完整 run unit 选择，不拆 user/assistant turn，也不拆 assistant/tool/final-assistant exchange；
- 相同输入数据除 UUIDv7 plan ID 外，消息顺序、选择、drop reason 和 prompt hash 必须一致。

DeepSeek 当前 context window 很大，但普通前台请求的输入预算仍上限为 32,768 estimated tokens，并预留 output、tool result 和安全边际，避免人格化演变成无界 prompt。没有官方 tokenizer 时使用保守估算器并记录 `estimated=true`；`TokenEstimator` trait 保留 provider-specific tokenizer 的替换点。

运行时并行读取 recent、SelfState 和 RelationshipState。recent 或当前消息失败会终止 run；可选状态源失败则记录降级并使用空值。编译成功后，必须在 provider 请求前原子写入 `context_plans`、关联 `runs.context_plan_id`，并写有 session seq 的 `context.compiled`。provider request metadata 同时携带 plan ID 和 prompt hash。

`lain-data show-context-plan <run-id>` 用于查看选择、版本、位置和 drop reason；`lain-data identity-diff <old-dir> <new-dir>` 在版本升级前生成 block hash/行数差异。原始敏感 prompt 不复制进事件 payload。

## 备选方案

- 继续拼接单体 prompt/profile：实现简单，但无法限制动态内容的权限、大小和来源，拒绝。
- 直接采用通用 agent framework 的 memory/prompt abstraction：会隐藏本产品最重要的顺序、预算、版本和人格边界，拒绝。
- 常驻全部 examples 和完整历史：人格可能更强，但 token/延迟成本不可控且 cache prefix 更易变化，拒绝。
- 为 DeepSeek 引入非官方 tokenizer：可能更接近实际 token，但增加供应链和版本漂移；P4 先使用明确标记的保守估算。

## 后果与边界

- `/v1/ws` 不再读取或拼接 `profile.md`；旧 `/ws/stream` 暂时保留 P2 兼容路径。
- 每个 P4 run 可从 run → context plan → identity version/hash 追踪；plan 支持诊断和同版本数据重建，但不会在事件里泄露完整 prompt。
- P5 才实现结构化 memory retrieval 和 snapshot provenance；P4 的 compiler 已建立 trust/budget 接口，但运行路径传入空 memory evidence。
- P6 才产生真实 tool calls；P4 已测试 tool exchange 作为完整 recent unit 保留，P6 需要补充原生 tool call ID/schema。
- identity block 内容变更必须提升 version、更新 manifest hash、运行 persona regression 与 diff，不能原地静默改写已激活版本。

## 迁移与回滚

`0002_identity_context.sql` 是 forward-only migration：新增 `identity_versions`、`state_snapshots`，并给 `context_plans` 增加 stable prefix hash 与 estimated 标记。不得回退或改写已应用 migration。

应用回滚时可临时切回 P3 binary；数据库中的新表和列保留。身份回滚使用一个仍可通过 hash/schema 校验的旧版本目录重新激活，不删除旧 run 或旧 identity version。

## 验证

- manifest、路径穿越、hash、compiler compatibility、examples schema 和 identity diff 测试；
- ContextBudget、状态范围/TTL、确定性 hash、recent turn 与 tool exchange 原子性、untrusted memory cap 测试；
- identity activation 幂等/冲突、typed snapshot、context plan/event 事务和 round-trip 测试；
- 50 个 persona fixtures 的 schema 与唯一性校验；
- 80 条 recent message、200 次 debug compile 的本机 p95 门禁（目标 ≤ 50 ms）；
- 临时进程两轮真实 DeepSeek 调用，核对 identity/plan/hash 与第二轮 cache usage。
