# P4 验收报告：Identity Kernel 与 Context Compiler

- 日期：2026-08-11
- 分支：`feat/p4-context-compiler`
- 维护者：KiloxGo
- 外部调用：2 次 DeepSeek，均为合成缓存验收输入

## 结论

P4 完成门通过：`/v1/ws` 的每个回复在 provider 请求前均持久化 identity version、compiler version、ContextPlan、prompt hash 和 stable-prefix hash；普通对话不再拼接整份 `profile.md`。人格不可变资产、动态个性化状态、非可信 memory evidence 和 recent history 已建立独立 schema、信任级别、顺序与 token 预算。

## 自动化验证

- `cargo test --all-targets --all-features --locked`：65 项通过。
- `cargo clippy --all-targets --all-features --locked -- -D warnings`：通过。
- `cargo fmt --all -- --check`：通过。
- `npm run lint`：通过。
- `npm run build`：通过。
- `node evals/validate.mjs`：persona 50 项/19 类，memory 20 项/20 类，共 70 个 fixtures，通过。
- `lain-data identity-diff identity/v1 identity/v1`：版本相同且 `changed_blocks=[]`。

覆盖内容包括 identity manifest/hash/schema/路径安全、激活幂等与版本冲突、SelfState TTL、RelationshipState 范围、确定性编译、预算失败、drop reasons、recent turn/tool exchange 原子性、memory trust label、ContextPlan 事务与 round-trip。

## Context compile 性能

本机 Windows、Rust debug profile，使用稳定 identity、79 条历史消息加当前消息，预热 20 次后测量 200 次：

```text
context compile p95: 1.490 ms
target: <= 50 ms
```

该测量只覆盖 compiler，不包含 SQLite、provider 网络或推理；P5 的 1 万 memory item 检索仍需单独达到 retrieval 门禁。P4 vector 关闭，compiler 接收的是已检索且有上限的 evidence，而不是扫描全库。

## 真实 DeepSeek 缓存验收

使用临时数据目录启动真实 binary，经 `/v1/ws` 在同一 session 连续发送两条合成消息。脚本只输出 usage 和 plan 元数据，不保存模型正文或密钥：

```json
{
  "callsUsed": 2,
  "identityVersion": "1.0.0",
  "compilerEstimated": [true, true],
  "estimatedInputTokens": [1968, 2017],
  "providerInputTokens": [1040, 1068],
  "cachedInputTokens": [0, 1024],
  "secondTurnCacheHit": true,
  "planIdsPresent": true,
  "promptHashesDiffer": true
}
```

结果符合预期：首轮建立前缀，第二轮复用 1024 cached input tokens；动态会话变化使完整 prompt hash 不同，但稳定 identity 前缀仍可命中。DeepSeek 官方说明缓存按相同前缀工作且是 best-effort，因此验收只证明当前 provider 路径能观测命中，不把命中作为正确性或 SLA。参考 [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache)。

验收后已停止进程，并在确认绝对路径位于系统临时目录且匹配 `lain-p4-cache-*` 前缀后删除临时数据库和日志。

## 可追溯性与调试

- `identity.activated` 记录 version 与 content hash；相同 version 不允许换内容。
- `context.compiled` 记录 plan ID、prompt hash、identity version、估算 token 和 drop 数量。
- `lain-data show-context-plan <run-id>` 输出完整选择/丢弃清单，不输出 API key。
- `lain-data identity-diff <old-dir> <new-dir>` 输出升级前 block hash 和行数变化。

## 已知后续工作

- P5 接入有 provenance 的长期 memory retrieval、更新/撤回、FTS 和 memory UI；当前 v1 不再使用旧 profile，但 memory evidence 为空。
- P6 增加原生 tool call/result ID 和 approval；P4 只保证完整 tool exchange 在 recent context 中不被截断。
- provider 返回的是实际 token usage，compiler 仍使用保守估算并明确标记 `estimated=true`；未来可接官方 tokenizer adapter。
