# ADR-0003：SQLite 事件存储与 session 协议 v1

- 状态：Accepted
- 日期：2026-08-11
- 维护者：KiloxGo

## 背景

P2 WebSocket 能把 provider 的流式输出送到浏览器，但对话历史仍依赖连接内存和 Markdown 追加。刷新会丢失当前 UI，重复提交可能再次调用模型，run 的完成状态也无法审计。人格化 harness 后续还需要 ContextPlan、记忆候选和后台任务，它们不能建立在无事务的文本日志上。

## 决策

采用本地 SQLite 作为结构化事实与事件主存储，默认文件为 `<LAIN_DATA_DIR>/state/lain.db`。连接使用 `foreign_keys=ON`、WAL、`synchronous=FULL`、5 秒 busy timeout、内存临时表和最多 4 个连接；schema 只通过提交到仓库的 sqlx forward migration 演进。

单用户原型启动时幂等创建一个 active instance、默认 user 与 active session。外部稳定 ID 和 event ID 使用 UUIDv7；SQLite rowid 不进入协议。

用户输入事务同时完成：

1. 按 `(session_id, client_message_id)` 尝试插入 user message；
2. 创建 queued run；
3. 原子递增 `sessions.last_seq` 并写 `message.accepted`；
4. 再次递增并写 `run.queued`；
5. commit 后才向客户端发送事件。

相同 `client_message_id` 的重试返回既有 message、run 和事件，不启动第二次模型调用。回复完成事务同时插入 assistant message、更新 run usage/state、写 `text.completed` 与 `run.completed`，并排入 `memory_extract` durable job。客户端只会在 commit 成功后看到 `run.completed`。

新增 `/v1/ws`。客户端必须在 5 秒内发送：

```json
{"type":"client.hello","versions":[1]}
```

服务端选择共同版本并返回默认 scope。客户端随后用 `session.subscribe` 和 `after_seq` 恢复。初次订阅或重放量超过 500 个事件时返回 `session.snapshot`；其余情况按 seq 升序重放。

持久事件使用共同 envelope。`message.accepted`、`run.queued`、`run.started`、`text.completed` 和终态 run 事件拥有 session seq。为了不把 token chunk 写入永久事件日志，`model.started` 与 `text.delta` 使用同一 envelope 但 `seq=null`；前端仅对非空 seq 执行缺口与重复检测，最终正文以 `text.completed` 为准。

每个连接使用容量 64 的有界出站队列和 2 秒等待上限，20 秒 WebSocket heartbeat，同时支持应用层 ping/pong。Origin、消息大小和 frame 大小继续由 upgrade 边界校验。客户端游标保存于 localStorage，采用带 jitter 的 250 ms 到 10 s 指数退避；检测到 seq gap 时用协议错误关闭并从最后确认的 seq 重连。

Rust DTO 通过 ts-rs 在测试期间导出到 `frontend/src/protocol/generated`。这些文件不得手工修改。

## Markdown 兼容

Markdown 不再是 session 的写入主路径。`lain-data import-markdown <dir> [--dry-run]` 只读解析现有每日会话文件，并用稳定的 legacy client message ID 幂等导入；`export-markdown <path>` 从 SQLite snapshot 生成可读副本。导入不会删除或改写源文件。

## 后果与边界

- 刷新、重连和已完成 run 的进程重启恢复不再依赖 provider；启动时遗留的 queued/running/cancelling run 会原子转为 `run.failed(kernel_restarted)`，不会永久卡住 UI。
- 断开浏览器不会中止正在进行的 provider stream；kernel 会继续消费并提交最终回复，重连后再重放。
- 当前只有默认 active session，创建/归档多个 session 的 UI 留给后续产品阶段。
- `context_plans` 在 P3 只建立 schema；P4 Context Compiler 才填充内容。
- `memory_extract` job 在 P3 只可靠入队；P5 worker 才会认领。
- 备份必须使用 SQLite Backup API 或一致性 checkpoint，不能在 WAL 活跃时只复制 `lain.db`。

## 回滚

不回退或改写 `0001_p3_event_store.sql`。应用可以临时切回已弃用的 `/ws/stream`，但已写数据库保留。若新版本无法启动，应先一致性备份整个 state 目录，再修复 forward migration；禁止删除用户数据库来“回滚”。

## 验证

- 空库 migration、文件库 WAL 重开和 foreign key 测试；
- 并发输入产生唯一连续 seq；
- 重复 client ID 不增加 message/run；
- queued → completed 非法跳转不写事件；
- 回复、终态事件和 durable job 同事务；
- v1 DTO/版本协商测试及 Rust → TypeScript 生成；
- 真实临时进程执行 hello、snapshot、持久回复、after-seq replay、幂等重试和正常关闭。
