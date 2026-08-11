# ADR-0001：Kernel 应用边界与降级启动

- 状态：Accepted
- 日期：2026-08-11
- 决策人：KiloxGo
- 关联任务：P1-001—P1-020

## 背景

原型把配置读取、依赖构造、路由、WebSocket 会话和后台任务集中在 `main.rs`。API Key、prompt 或用户目录异常会导致启动 panic；后台任务缺少所有权和关机语义；健康检查也无法区分“进程存活”和“可以接受对话”。这使 harness 难以测试、诊断和逐步替换 provider、记忆与协议。

## 决策

- `main.rs` 只负责加载 `.env`、初始化 tracing、加载配置并调用 library composition root。
- `Config` 将环境覆盖集中验证，路径在启动时绝对化，Secret 不进入 Debug/日志。
- `AppState` 显式持有配置、readiness、可选 provider、`CancellationToken` 与 `TaskTracker`。
- `/api/health` 仅证明 event loop/HTTP 存活；`/api/ready` 脱敏报告 identity、数据目录和 provider 状态。
- 缺少 API Key 或 identity 时服务降级启动，但拒绝 `/ws` upgrade。
- 所有自有后台 I/O 经统一 supervisor 入口进入 `TaskTracker`；HTTP 排空和后台任务排空共享关机超时。
- 旧 `/ws` 保持裸文本兼容，并在成功握手中返回 `Deprecation: true`；版本化协议在 P3 替换它。

## 备选方案

- 配置错误立即终止进程：对无效 bind/URL/数值仍采用；对 provider/identity 不采用，因为会破坏本地诊断与健康探针。
- 将所有逻辑保留在 `main.rs`：实现简单，但无法做无端口 router 测试或替换依赖。
- 立即删除旧 `/ws`：会破坏当前 UI，留到版本化协议与前端同步迁移。

## 影响

正面影响是启动可诊断、错误稳定、任务有所有权、关机有上界，且配置/路由可以独立测试。代价是状态结构和生命周期代码增加；provider 仍是具体 DeepSeek adapter，真正的 provider-neutral stream 留给 P2。

## 安全与隐私

默认只监听 loopback；CORS 只允许本地 Vite origin，WebSocket 在 upgrade 前独立拒绝不可信浏览器 Origin；请求体、frame 和消息均有限制；就绪响应不返回 Secret、文件路径、prompt 或数据正文；provider 错误细节仅记录在服务端。

## 验证

- 配置单元测试覆盖默认值、绝对路径、无效 bind/URL 和 Secret 脱敏。
- router 测试覆盖 health/ready 和请求 ID。
- 进程集成测试覆盖无 Key 降级启动。
- TaskTracker 测试覆盖限时排空。
- `cargo fmt`、`cargo check`、Clippy `-D warnings` 与全量测试作为合并门禁。

## 回滚

此变更不迁移持久数据。回滚代码即可恢复旧启动路径；旧 `/ws` 的请求和响应载荷未改变。
