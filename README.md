# Lain Agent Interface Network

一个处于早期原型阶段的本地优先、人格化对话 Agent。当前仓库已经具备 Rust/Axum 后端、React/Vite 前端、可恢复的 WebSocket session 协议、DeepSeek 适配器、版本化 Identity Kernel 和确定性 Context Compiler；它还不是完整的通用 agent harness。项目目前由 KiloxGo 维护并保持私有。

项目目标不是复刻某个角色的台词，而是在可验证的性能、可靠性和安全边界内，构建可长期演化、可个性化的 harness：稳定身份、上下文组装、分层记忆、模型路由、工具策略、事件协议和评测闭环。

## 当前状态

- 内置工具：CLI workspace 的 chat mode 提供只读 `time_now` 与 `instance_status`；仅返回 UTC 时间和非敏感实例版本状态。
- 并行策略：同一模型 step 中的全只读工具批次可以并行执行；写入、检查、未知工具及混合批次保持串行。
- MCP：可选启动一个本地 stdio server，支持 MCP 2026-07-28 discovery 与旧版 initialize fallback；chat mode 会暴露有界、namespaced 的工具目录，但每次调用都按外部副作用处理并要求一次性 `y/N`，不采信 server annotations，审批超时或断线时拒绝执行。

- 后端：Rust 2024、Tokio、Axum，默认监听 `127.0.0.1:3000`。
- 前端：React 19、TypeScript、Vite。
- 模型：provider-neutral `ModelProvider` 事件流；默认路由到 DeepSeek V4 Flash，显式复杂推理可路由到 V4 Pro。
- 存储：`~/.lain/state/lain.db` 下的 SQLite WAL 保存 session、message、run 与事件；旧 Markdown 记忆暂时保留为兼容读取和迁移入口（根目录可通过 `LAIN_DATA_DIR` 覆盖）。
- 记忆：Memory v1 将模型抽取先写为候选，由 Rust 策略校验证据、敏感度和指令污染；只有已提交 item 会通过有来源的 FTS5 evidence 进入 Context Compiler。
- 身份与上下文：`identity/v1` 资产启动时校验 manifest/schema/hash；每个 v1 run 在模型调用前保存 identity version、ContextPlan、prompt hash、预算选择与 drop reason。
- 工具：CLI 自动启动的 kernel 提供只读 `memory_search`/`memory_get` 与 workspace `list_files`、`read_file`、`search_text`、`replace_text`、`create_file`、`run_check`；记忆工具继承 scope、状态和 `never_recall` 策略，精确写入、no-clobber 创建和八种预定义 Cargo/Node/npm 检查必须先展示 diff 或 command/cwd 并取得一次性 `y/N` 授权，尚不提供覆盖式 write 或任意 shell。
- 测试：配置/启动/关闭、SSE 分块、provider 错误/重试/超时/取消、SQLite 事务/迁移/并发 seq、协议 DTO 与 Markdown 迁移已有离线契约测试。

完整现状、目标架构、ADR 和 209 项实施任务见 [TECHNICAL_IMPLEMENTATION_PLAN.md](TECHNICAL_IMPLEMENTATION_PLAN.md)。

## 快速启动

需要 Rust 1.89.0 和 Node.js 24.1.0。没有 DeepSeek API Key 时后端也会启动，便于诊断，但不会进入 ready 状态或接受对话。

后端：

```powershell
Set-Location lain-kernel
Copy-Item ..\.env.example .env
# 要启用对话，请编辑 .env 并填入 DEEPSEEK_API_KEY
cargo run
```

前端（另开终端）：

```powershell
Set-Location frontend
npm ci
npm run dev
```

终端 CLI 会复用已运行的 kernel；如果默认地址不可用，则自动增量构建并启动本仓库 kernel：

```powershell
.\lain.cmd
.\lain.cmd -p "介绍一下你自己"
.\lain.cmd -c
```

可用 `LAIN_WS_URL` 覆盖默认的 `ws://127.0.0.1:3000/v1/ws`。CLI 自动启动 kernel 时将调用目录作为受限 workspace；可用 `LAIN_WORKSPACE` 显式覆盖。`-c` 会显示最近的持久消息并继续默认 session。CLI 支持流式回复、工具进度、精确编辑/项目检查审批、`/help`、`/history`、`/status`、`/quit` 和 `Ctrl+C` 取消当前 run。

默认访问 Vite 输出的本地地址。`/api/health` 只表示进程存活，`/api/ready` 检查 identity、SQLite、数据目录和 provider；`/api/provider` 返回不含 Secret 的能力诊断。前端使用 `/v1/ws`：协商版本后订阅默认 session，按 seq 重放持久事件并支持刷新恢复、退避重连和 Stop。`/ws/stream` 与裸文本 `/ws` 暂时作为弃用兼容入口。

Memory extractor 默认关闭，避免在不知情时产生额外 provider 调用。设置 `LAIN_MEMORY_EXTRACTION_ENABLED=true` 后，每个成功回复会异步消费一个 `memory_extract` job；失败只重试后台 job，不阻塞主对话。前端 Memory 面板通过本地 `/api/memory` 查看来源并确认、拒绝、pin、never-recall、dispute 或 retract。

只输出 provider 配置、readiness 与能力，不发起模型调用：

```powershell
cargo run -- --diagnose-provider
```

旧版 `memory/sessions/*.md` 可以先 dry-run，再幂等导入当前 SQLite；导出不会修改数据库：

```powershell
cargo run --bin lain-data -- import-markdown "$env:LAIN_DATA_DIR\memory\sessions" --dry-run
cargo run --bin lain-data -- import-markdown "$env:LAIN_DATA_DIR\memory\sessions"
cargo run --bin lain-data -- export-markdown ".\session-export.md"
cargo run --bin lain-data -- show-context-plan "<run-id>"
cargo run --bin lain-data -- identity-diff identity/v1 identity/v1
```

## 本地质量检查

```powershell
Set-Location lain-kernel
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features

Set-Location ..\frontend
npm ci
npm run lint
npm run build
npm audit --omit=dev

Set-Location ..
node evals/validate.mjs
node --test cli/lain.test.mjs
```

CI 在 Windows 与 Linux 验证 Rust，在 Linux 验证前端、评测数据格式与依赖策略。

## 仓库结构

```text
lain-kernel/     Rust 后端与系统提示词
frontend/        React Web UI
cli/             无额外依赖的终端客户端
docs/            ADR、威胁模型、基线和知识产权策略
evals/           可版本化的人格与记忆评测样例
.github/         CI 与协作模板
```

## 配置与数据

- 不要提交 `.env`、API Key、聊天记录或真实用户资料。
- `LAIN_IDENTITY_DIR`、`LAIN_SYSTEM_PROMPT`、`LAIN_DATA_DIR` 与 `LAIN_WORKSPACE` 可配置；相对路径会以启动目录为基准解析成绝对路径。SQLite 位于 `<LAIN_DATA_DIR>/state/lain.db`，不要在 WAL 活跃时只复制主数据库文件。默认值仍适合从 `lain-kernel` 目录启动。
- `LAIN_BIND_ADDR`、provider 地址、Flash/Pro 模型、超时、消息大小和关机超时均可通过 `.env.example` 中的变量覆盖；配置与诊断输出会隐藏 API Key。
- 评测样例必须使用合成数据，不得复制真实会话。
- 当前仓库尚未选定开源许可证；代码和角色相关素材的授权边界见 [docs/IP_AND_ASSET_POLICY.md](docs/IP_AND_ASSET_POLICY.md)。

## 贡献约定

任何影响身份、记忆写入、模型路由、工具权限或客户端协议的变更，都必须同步：ADR 或决策记录、测试/评测样例、迁移/回滚说明、可观测指标。先保持核心层与具体模型/前端解耦，再扩展功能。
