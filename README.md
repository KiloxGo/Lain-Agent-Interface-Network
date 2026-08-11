# Lain Agent Interface Network

一个处于早期原型阶段的本地优先、人格化对话 Agent。当前仓库已经具备 Rust/Axum 后端、React/Vite 前端、WebSocket 对话入口、DeepSeek 适配器，以及基于 Markdown 文件的实验性记忆；它还不是完整的通用 agent harness。项目目前由 KiloxGo 维护并保持私有。

项目目标不是复刻某个角色的台词，而是在可验证的性能、可靠性和安全边界内，构建可长期演化、可个性化的 harness：稳定身份、上下文组装、分层记忆、模型路由、工具策略、事件协议和评测闭环。

## 当前状态

- 后端：Rust 2024、Tokio、Axum，默认监听 `127.0.0.1:3000`。
- 前端：React 19、TypeScript、Vite。
- 模型：当前默认使用 DeepSeek V4 Flash Chat Completions（关闭 thinking 以保持现有非推理交互），尚无 provider 抽象或流式输出。
- 存储：当前为 `lain-kernel/memory/` 下的 Markdown/JSON 文件；目标方案是 SQLite + FTS5。
- 工具：当前仅有由模型文本标签触发的实验性记忆行为，尚无受约束的结构化工具层。
- 测试：已建立静态检查与评测样例骨架；运行时、协议和记忆测试仍按实施清单推进。

完整现状、目标架构、ADR 和 209 项实施任务见 [TECHNICAL_IMPLEMENTATION_PLAN.md](TECHNICAL_IMPLEMENTATION_PLAN.md)。

## 快速启动

需要 Rust 1.89.0、Node.js 24.1.0，以及有效的 DeepSeek API Key。

后端：

```powershell
Set-Location lain-kernel
Copy-Item ..\.env.example .env
# 编辑 .env，填入 DEEPSEEK_API_KEY
cargo run
```

前端（另开终端）：

```powershell
Set-Location frontend
npm ci
npm run dev
```

默认访问 Vite 输出的本地地址；健康检查为 `http://127.0.0.1:3000/api/health`，WebSocket 为 `ws://127.0.0.1:3000/ws`。

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
```

CI 在 Windows 与 Linux 验证 Rust，在 Linux 验证前端、评测数据格式与依赖策略。

## 仓库结构

```text
lain-kernel/     Rust 后端与系统提示词
frontend/        React Web UI
docs/            ADR、威胁模型、基线和知识产权策略
evals/           可版本化的人格与记忆评测样例
.github/         CI 与协作模板
```

## 配置与数据

- 不要提交 `.env`、API Key、聊天记录或真实用户资料。
- 当前后端依赖启动目录解析 `prompts/lain_system.md` 与 `memory/`，因此请从 `lain-kernel` 目录启动。
- 评测样例必须使用合成数据，不得复制真实会话。
- 当前仓库尚未选定开源许可证；代码和角色相关素材的授权边界见 [docs/IP_AND_ASSET_POLICY.md](docs/IP_AND_ASSET_POLICY.md)。

## 贡献约定

任何影响身份、记忆写入、模型路由、工具权限或客户端协议的变更，都必须同步：ADR 或决策记录、测试/评测样例、迁移/回滚说明、可观测指标。先保持核心层与具体模型/前端解耦，再扩展功能。
