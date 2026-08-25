# Changelog

本文件记录用户可感知行为、协议、数据格式和工程基线的变化。格式遵循 Keep a Changelog 的分区习惯；版本策略会在首次可发布里程碑前确定。

## Unreleased

### Added

- 首个有界只读 Tool Loop：模型可在 CLI workspace 内列文件、读取 UTF-8 文本和字面搜索；调用与结果状态写入 SQLite 审计，CLI 显示执行进度。
- 无额外运行时依赖的终端 CLI：复用 v1 协议的持久会话与流式输出，支持单次 prompt、最近历史、token/cache usage、取消和自动启动本地 kernel。
- 根级项目说明、环境变量示例与固定工具链版本。
- 目标架构、技术调研、ADR 与分阶段实施清单。
- CI、依赖策略、威胁模型、知识产权策略和基线记录。
- 人格与记忆最小评测数据集及格式校验器。
- 可测试的 Kernel 应用层、结构化配置/错误、存活与就绪探针、请求关联 ID。
- 受跟踪后台任务、取消传播、限时优雅停机、请求/消息大小限制和显式本地 CORS。
- Provider-neutral 模型请求/事件/错误接口、DeepSeek SSE adapter、代码模型路由与可脚本化 MockProvider。
- 增量 WebSocket UI、运行中取消、token/cache usage 日志和脱敏 provider capability 诊断。
- 有来源的 Memory v1：候选/事实/关系表、耐久抽取 worker、确定性敏感策略、版本替换、FTS5 检索和 ContextPlan 注入。
- 本地记忆控制面：pending 确认/拒绝、来源查看、pin、never-recall、dispute 和 retract。
- DeepSeek Harness 官方架构参考调研与 Memory v1 ADR。

### Changed

- 修复现有 Rust Clippy 告警，并传播记忆文件写入错误。
- 将 `rustls-webpki` 从 0.103.12 锁定更新到 0.103.13，修复 RUSTSEC-2026-0104。
- 将传递依赖 `h2` 从 0.4.13 锁定更新到 0.4.16，修复 RUSTSEC-2026-0258。
- 在现有 semver 范围内刷新前端 lockfile，消除 6 个开发依赖审计告警。
- 将后端默认监听地址从全部网卡收紧为 `127.0.0.1:3000`，落实本机优先的 P0 信任边界。
- 确认推荐产品默认值、KiloxGo 维护责任、私有发布策略和第三方角色素材禁分发边界。
- 将已停用的 `deepseek-chat` 默认模型迁移到 `deepseek-v4-flash`，使用官方基础地址并显式关闭 thinking；允许通过环境变量覆盖模型和地址。
- 重写系统提示词的优先级和边界：安全、诚实、任务完成、隐私与用户控制优先于人格氛围；保留克制的 Lain interface 风格。
- 建立 49 次受控 DeepSeek 调用的人格、token、延迟、RSS 与文件存储 P0 在线基线，并保留旧 prompt 的失败结果。
- 完成私有 GitHub 分支的 Windows/Linux/前端/eval/依赖策略远端 CI 验收。
- 保持旧 `/ws` 裸文本协议兼容，同时以标准 `Deprecation` 响应头标记迁移边界。
- 将业务层从 DeepSeek DTO 解耦；普通聊天改为单请求流式消费，部分输出后断流不自动重试。
