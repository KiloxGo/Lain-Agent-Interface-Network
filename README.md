<img width="2547" height="1360" alt="image" src="https://github.com/user-attachments/assets/400222a8-2fa5-43df-bd9d-cf0ad9f37f69" />

# 烂尾了。。。

# Lain Agent Interface Network

## 当前源码

Rust workspace 有四个 crate：

- `lain-kernel`：Kernel 库、`laind` 和独立 `lain-device-agent`；服务端 Actor 与原生 Worker 监督分开启动。
- `lain-client`：`lain` CLI/TUI，经 HTTP/SSE 访问 Kernel。
- `lain-wire`：公共强类型 ID 和数据结构。
- `lain-local`：CLI 与设备服务共用的本机私有文件／权限工具。

`web/` 是保留的 React/TypeScript 客户端。
`legacy/v0` 保存旧原型，仅供历史参考；本轮调整不恢复其架构。

原生服务需要独立配置并显式启动；远端 API 连接本身不启用设备执行。
