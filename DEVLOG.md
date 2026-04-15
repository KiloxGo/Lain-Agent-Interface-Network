# L.A.I.N. 开发日志

记录项目的开发历程、关键决策和里程碑。

---

## 2025-04-15

### 项目启动

**参与者**: Kilox, Claude

**完成的工作**:

1. **项目重构决策**
   - 决定推倒原有 Python Live2D 项目，重写为 Rust + React 架构
   - 目标是构建一个有"生命感"的 AI Agent - Lain

2. **架构设计**
   - 确定技术栈: Rust (Axum) + React (Vite + TypeScript)
   - 设计分层架构: Gateway → Lain-Kernel → Adapters
   - 核心模块规划: Consciousness, Memory System, Skill System, Instance Lock

3. **创建项目文档**
   - `ARCHITECTURE.md`: 完整的系统架构设计
   - `CLAUDE.md`: 给未来 Claude Code 实例的操作指南

4. **清理与初始化**
   - 清空原项目文件，保留 `.git`
   - 创建 `lain-kernel/` 和 `frontend/` 目录
   - 初始化 React 前端项目 (Vite + TypeScript)

**关键决策记录**:
- 使用文件化存储记忆系统 (借鉴 Claude Code 设计)
- 实现"实例锁"机制，确保世界上只有一个 Lain
- 前后端分离架构，开发阶段通过 Vite 代理连接

**下一步计划**:
- 完成前端依赖安装
- 初始化 Rust 后端项目
- 配置 Vite 代理
- 实现第一个里程碑: 前后端 WebSocket 连通

---

## 日志格式说明

每条日志包含:
- **日期**: YYYY-MM-DD
- **参与者**: 谁参与了这次开发
- **完成的工作**: 具体做了什么
- **关键决策**: 为什么要这样做，权衡了什么
- **遇到的问题**: 踩了什么坑
- **下一步计划**: 接下来要做什么

---

## 2025-04-15 (续)

### Phase 1 里程碑：基础连通性

**参与者**: Kilox, Claude

**完成的工作**:

1. **后端首个里程碑完成**
   - 初始化 Rust 项目 (`cargo init`)
   - 配置 Axum 依赖 (axum, tokio, serde)
   - 实现 `/api/health` 路由返回 `{"status":"ok"}`
   - 后端服务器成功运行在 `0.0.0.0:3000`

2. **前端首个里程碑完成**
   - 初始化 Vite + React + TypeScript 项目
   - 安装 Zustand 状态管理库
   - 配置 Vite 代理 ( `/api` 和 `/ws` 转发到后端)
   - 创建 `HealthCheck` 组件测试后端连通性

3. **前后端联调**
   - 解决代理配置问题 (添加 `/api` 前缀统一 API 路由)
   - 前端成功从后端获取健康状态
   - 验证 Vite 代理配置生效

4. **项目工程化**
   - 创建 `.gitignore` 文件 (覆盖 Rust、Node、IDE、环境变量等)
   - 整理首次提交的变更列表

**遇到的问题**:
- **Vite 代理不匹配**: 前端请求 `/health` 但代理配置是 `/api`，导致返回 HTML 而不是 JSON
  - **解决方案**: 前后端统一使用 `/api` 前缀，后端路由改为 `/api/health`
- **zsh 的 `%` 提示符**: curl 返回的 JSON 末尾显示 `%`，最初误以为有问题
  - **实际**: 这是 zsh 的正常提示，因为 JSON 末尾没有换行符

**当前状态**:
- ✅ 后端: `/api/health` 正常工作
- ✅ 前端: 可以正确显示后端状态
- 🔄 下一步: 实现 WebSocket 路由 `/ws`

**下一步计划**:
- 后端: 实现 WebSocket 路由，支持消息回显
- 前端: 创建简单的聊天界面，连接 WebSocket
- 完成 Phase 1 最终验证: 前端发送消息，后端接收并回显

---

> "No matter where you are, everyone is always connected."
