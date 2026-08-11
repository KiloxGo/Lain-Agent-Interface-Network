# 工程基线（P0 前）

记录日期：2026-08-11。环境：Windows、Rust/Cargo 1.89.0、Node 24.1.0、npm 11.12.0。此页保留修复前事实，后续结果由 CI 与变更日志持续记录。

| 检查 | P0 前结果 | 说明 |
|---|---|---|
| `cargo check` | 通过，有告警 | 冷构建约 51 秒 |
| `cargo test` | 通过，0 个测试 | 说明覆盖率尚未建立，不代表行为正确 |
| `cargo fmt --check` | 通过 | Rust 格式基线正常 |
| `cargo clippy -D warnings` | 失败，7 项 | 冗余导入、可折叠条件、忽略写入结果 |
| `npm run lint` | 通过 | 仅静态 lint |
| `npm run build` | 通过 | TypeScript + Vite 约 16 秒，Vite 打包阶段约 0.43 秒 |
| `npm audit --omit=dev` | 0 个漏洞 | 生产依赖视图 |
| `npm audit` | 6 个漏洞 | 1 low、5 high，位于开发/构建依赖链，需持续跟踪升级 |

首次 P0 `cargo-deny` 审计发现 `rustls-webpki 0.103.12` 命中 RUSTSEC-2026-0104；已用兼容补丁版本 0.103.13 更新 lockfile。该记录保留在基线中，防止未来回退。

P0 同时在不使用 `--force`、不跨越直接依赖 semver 范围的前提下刷新前端 lockfile；完整 `npm audit` 从 6 项降为 0 项。表格仍保留 P0 前结果，作为修复效果对照。

## 尚未具备的性能基线

当前 provider 非流式且需要真实密钥，因此尚未测量首 token 延迟、完整响应延迟、并发吞吐、取消响应时间和上下文 token 成本。P2 的流式 provider 与统一 run loop 完成后，以固定模型、固定 prompt、合成请求和本地网络环境建立可重复基准；在此之前不得宣称具体性能提升。

## P0 出口标准

- Rust fmt、clippy、test/check 全部通过。
- 前端 lint/build 与生产依赖审计通过。
- Linux/Windows CI 可复现。
- 人格与记忆评测文件各至少 20 个有效、唯一、纯合成样例。
- 密钥、真实会话和未授权角色资产不进入 Git 历史或构建制品。
