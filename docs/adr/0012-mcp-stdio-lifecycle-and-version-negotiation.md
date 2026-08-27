# ADR-0012：MCP stdio 生命周期与双时代版本协商

- 状态：Accepted
- 日期：2026-08-27
- 决策者：KiloxGo

## 背景

MCP 2026-07-28 已移除连接级 `initialize` 生命周期，改为每个请求携带协议元数据，并以 `server/discover` 提供能力发现；2025-11-25 及更早 server 仍要求 `initialize`/`notifications/initialized`。本项目需要接入本地 MCP 工具，同时不能让可选 server 故障阻止普通聊天。

## 决策

1. 第一版只支持一个显式配置的本地 stdio server；未配置时不启动任何子进程。
2. 客户端先以 `2026-07-28` 元数据调用 `server/discover`。成功且版本相容时采用现代无状态协议；普通未知方法错误或超时才回退旧版 `initialize`，明确的 unsupported-version 错误不回退。
3. 旧版请求 `2025-11-25`，接受四个已知历史版本；成功后发送 `notifications/initialized`。
4. stdio 使用单行 JSON-RPC 2.0 framing，输入和输出各限制 64 KiB，握手 10 秒超时，最多跳过 32 条无关消息。
5. 子进程默认继承非敏感环境；复用现有环境变量过滤器移除 provider、云、GitHub、token、password 与 credential 类变量。stderr 丢弃，避免阻塞和未经处理的日志泄露。
6. kernel shutdown 时先关闭 child stdin 并等待 2 秒，仍未退出才强制终止。连接或协商失败只记录警告并禁用 MCP，不改变 readiness。
7. server 声明 tools capability 时，连接阶段在 10 秒总预算内分页调用 `tools/list`，最多接收 32 页、128 个工具；目录不合法则禁用该 MCP server。
8. 模型侧工具名使用配置 namespace，而不使用不唯一的 server self-reported name。格式为 `<namespace>__<tool>`；下划线与点号分别转义为 `_u` 与 `_d`，最终名称必须满足 DeepSeek Chat 的 64 字符限制。
9. `tools/call` 只接受 JSON object 参数和已发现的 namespaced 工具名；现代请求逐次携带 `_meta`，旧版保持原 wire shape。在 P6-019 host approval policy 完成前不向模型暴露。
10. server 提供的工具说明统一带 `mcp:<namespace>` 不可信来源标签并限制为 2 KiB；输入 schema 递归移除 `description`、`title`、`$comment`、`examples` 和 `default` 等展示型字段，目录原始 metadata 总计不得超过 256 KiB。工具结果不公开原始旁路，只输出带来源标签且限制为 64 KiB 的模型文本。

## 备选方案

- 只实现旧 `initialize`：无法正确支持当前协议，拒绝。
- 只实现 2026-07-28：会立即排除大量现有 server，拒绝。
- 引入完整 MCP SDK：当前只需要 stdio、协商和进程生命周期，依赖与抽象成本过高，暂不采用。
- 首版支持多个 server、自动重启与动态重载：尚无第二个真实 server 或故障数据，推迟。

## 安全与隐私

- server self-reported identity 和 capabilities 仅用于诊断，不参与授权决策。
- server annotations、工具说明、schema 展示字段和工具结果均不作为权限依据；授权只由 host policy 决定。
- stdout 只能包含有界 JSON-RPC；畸形、超长、关闭或版本不相容均 fail closed。
- 不把 DeepSeek/API/cloud 凭证隐式转交给 MCP 子进程；需要 server 专用凭证时再增加显式最小 env allowlist。

## 迁移与回滚

没有数据库迁移。移除 MCP 环境变量即可禁用；代码回滚只需删除 client 字段、生命周期任务和配置解析，不影响会话、记忆或工具审计数据。

## 验证

- 合成 Node stdio server 覆盖现代 discovery、旧版 initialize fallback、capability/identity 解析、分页工具目录、命名空间、现代请求 `_meta`、新旧 `tools/call` 与 EOF shutdown。
- 配置测试覆盖 opt-in、namespace 字符约束、JSON args 和 Debug 参数脱敏。
- 全量 Rust、CLI、评测与跨平台 CI 必须保持通过。

## 官方参考

- [MCP 2026-07-28 stdio transport](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/transports/stdio.mdx)
- [MCP 2026-07-28 server discovery](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/server/discover.mdx)
- [MCP 2026-07-28 tools](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/server/tools.mdx)
- [DeepSeek Chat Completions tools](https://api-docs.deepseek.com/api/create-chat-completion/)
