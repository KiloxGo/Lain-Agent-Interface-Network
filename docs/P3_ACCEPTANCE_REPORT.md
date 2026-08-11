# P3 验收报告：事件存储、Session 与协议 v1

- 日期：2026-08-11
- 分支：`feat/p3-event-store`
- 维护者：KiloxGo
- 数据：仅临时目录和合成消息；临时数据库在测试后删除

## 结论

P3 完成门通过：页面可从 snapshot/replay 恢复 active session；同一个 `client_message_id` 重试不会创建第二条 user message、第二个 run 或第二次模型调用；`run.completed` 只在回复事务提交后发送，进程重启可从 SQLite 恢复最终正文。

## 自动化验证

- `cargo test --all-targets --all-features`：51 项通过。
- `cargo clippy --all-targets --all-features -- -D warnings`：通过。
- `npm run lint`：通过。
- `npm run build`：通过。
- 覆盖空库 migration、WAL 重开、foreign keys、并发 seq、消息幂等、run 状态机、重启中断恢复、完成事务、durable job、Markdown import/export 和协议类型生成。

## 真实进程、零 API 验证

临时启动 binary，实际创建 SQLite 文件并通过 `/v1/ws` 完成：

```text
client.hello -> server.hello(v1)
session.subscribe(after_seq=0) -> session.snapshot(last_seq=0)
client Close -> server Close -> NormalClosure
```

结果：数据库创建、默认 scope、snapshot 和关闭握手均通过。

## 真实 DeepSeek 纵向验证

使用 1 次模型调用，合成输入只要求返回固定标记 `P3_STORE_OK`。没有发送真实会话、记忆或第三方角色素材。

首次连接收到的持久事件：

```text
1 message.accepted
2 run.queued
3 run.started
4 text.completed
5 run.completed
```

模型最终文本精确为 `P3_STORE_OK`。断线重连并订阅 `after_seq=3` 时只重放 seq 4、5。随后以相同 `client_message_id` 重发，服务端返回既有 seq 1–5，模型调用计数仍为 1；最终 snapshot 为 `last_seq=5`、2 条消息，关闭状态为 `NormalClosure`。

## 数据与密钥

- API Key 只由本地 `.env` 加载，没有打印、写入数据库或提交。
- 所有验收数据库位于带 `lain-v1-*` 前缀的系统临时目录。
- 清理前校验解析后的绝对路径仍位于系统临时目录且名称前缀匹配，然后删除临时目录。

## 已知后续工作

- P4 将真正写入 `context_plans` 并把 legacy prompt/profile 拼接替换为可版本化 Context Compiler。
- P5 才实现 `memory_extract` durable job worker；P3 仅保证任务与回复同事务入队。
- 多 session 创建、归档和选择 UI 尚未进入本阶段产品范围。
