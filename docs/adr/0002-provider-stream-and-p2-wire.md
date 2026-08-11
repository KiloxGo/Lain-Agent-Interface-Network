# ADR-0002：Provider 事件流与 P2 过渡协议

- 状态：Accepted
- 日期：2026-08-11
- 决策人：KiloxGo
- 关联任务：P2-001—P2-018

## 背景

P1 的 `LlmClient` 同时持有 identity、用户 profile、DeepSeek DTO 和整段响应解析。业务层依赖供应商的 `choices` 结构，无法在首 token 到达时显示内容，也无法表达 usage、tool-call delta、部分断流或中途取消。旧 `/ws` 客户端又假设一个 frame 就是一条完整回复，直接改成逐 delta frame 会破坏兼容性。

## 决策

- 核心只依赖 `ModelRequest`、`ProviderCapabilities`、`ModelEventStream` 和稳定分类的 `ModelError`；DeepSeek JSON/SSE 结构只存在于 adapter。
- `ModelEvent` 表达 started、text/reasoning delta、tool-call 生命周期、usage、finish 与 warning。reasoning 只在 run 内消费，不发送前端或持久化。
- 前台请求最多两次尝试；仅在任何输出前对 rate limit、timeout、transport、overload 和 server error 使用指数退避与 jitter。部分输出后的断流标记 `partial/incomplete`，不从头重试。
- 默认 Flash 非 thinking；只有显式复杂推理短语走 Pro thinking；maintenance 走 Flash JSON。路由是代码规则，不增加分类模型调用。
- P2 新前端连接 `/ws/stream`，使用 `started/delta/finished/error` JSON 事件并可发送 `cancel`。旧 `/ws` 继续聚合后返回单个裸文本 frame。两者均标记 deprecated，P3 用正式版本化事件协议替换。
- 隐藏 memory/topic 标签通过跨 delta 状态过滤器抑制，不能因为流式输出提前泄漏给 UI。
- `--diagnose-provider` 与 `/api/provider` 只输出模型名、readiness 和静态能力，不输出或验证 API Key，也不产生模型费用。

## Provider 事实边界

截至本 ADR 日期，DeepSeek 官方模型页声明 V4 Flash/Pro 为 1M context、最大 384K output；thinking 通过 `thinking.type` 控制。Strict tool mode 仅在 `/beta` base URL 开启，因此默认正式 base URL 的能力声明为 `supports_strict_tools=false`。

- https://api-docs.deepseek.com/quick_start/pricing/
- https://api-docs.deepseek.com/guides/thinking_mode
- https://api-docs.deepseek.com/guides/tool_calls
- https://api-docs.deepseek.com/quick_start/error_codes/

这些数值属于 provider adapter 的可更新声明，不进入人格、记忆或业务规则。

## 备选方案

- 在应用层直接解析 DeepSeek SSE：文件更少，但继续让业务逻辑依赖供应商结构。
- 立即改变旧 `/ws` frame 语义：实现简单，但会把现有 UI 和任何脚本变成碎片消息。
- 在 P2 直接实现 P3 完整协议：会把 streaming 与 session/event persistence 两个风险面绑在一个变更中。
- 部分断流后自动重放请求：可能生成重复文本和重复费用，因此拒绝。

## 影响

首 token 可以立即显示，取消会停止当前 response body，usage/cache token 与 provider request ID 可关联记录；MockProvider 和 mock HTTP/SSE server 使 provider 行为无需真实 Key 即可验证。代价是 P2 存在一个明确临时的 wire schema，P3 迁移时需要同步替换前端 reducer。

## 安全与隐私

供应商错误正文只进入服务端日志，客户端只收到稳定 code。reasoning 不出服务端；hidden tags 在跨 chunk 情况下也被抑制；重试只发生在输出前；诊断不打印 Secret。Tool-call delta 仅解析和报告，不执行任何工具。

## 验证

离线契约测试覆盖正常 streaming、任意 byte/Unicode 分块、tool arguments、末帧 usage、空 choices、畸形 JSON、401/402/422、429 Retry-After、500/503、timeout/cancel、部分断流和 JSON empty content。Rust/前端门禁及 Windows/Linux CI 作为合并条件。

## 回滚

旧 `/ws` 载荷保持不变，可将前端临时切回它。此阶段没有数据库迁移；回滚代码即可，不需要处理持久数据格式。
