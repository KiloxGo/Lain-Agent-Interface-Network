# ADR-0010：只读运行时信息工具

- 状态：Accepted
- 日期：2026-08-27
- 决策者：KiloxGo

## 背景

模型处理时间相关请求或诊断当前实例时，不能依赖训练数据猜测当前时间、版本、身份或模型。让模型读取环境变量、文件路径或任意系统命令会扩大权限面，并可能泄露本地配置。

## 决策

1. 在配置 workspace 的 chat mode 提供无参数、只读的 `time_now` 和 `instance_status`。
2. `time_now` 返回 RFC 3339 UTC 时间和 Unix 毫秒时间戳，不猜测本地时区。
3. `instance_status` 仅返回 ready 状态、instance ID、kernel/protocol/identity 版本和当前模型；不返回路径、环境变量、密钥、用户或 session 标识。
4. 两个工具复用现有 invocation 审计、调用次数限制和 untrusted tool output 包装，无需审批。
5. 不增加系统信息采集层、第三方依赖或可配置字段；需要真实运行指标时另行设计受限诊断接口。

## 备选方案

- 让模型自行推断：结果可能过时或虚构，拒绝。
- 暴露环境变量或任意诊断命令：权限与泄露风险过高，拒绝。
- 新建通用 system-info registry：当前只有两个固定工具，暂不需要。

## 安全与隐私

- 两个 schema 拒绝未知参数，工具没有文件、网络或进程副作用。
- 输出使用固定字段白名单，避免意外序列化完整配置或 `AppState`。
- 工具结果仍按不可信数据处理，不改变系统指令或授权边界。

## 迁移与回滚

无 schema 迁移。回滚时移除两个 definition 和 dispatch；已有事件与数据库不受影响。

## 验证

- 单元测试覆盖 schema 名称、UTC/时间戳、状态白名单和未知参数拒绝。
- DeepSeek 烟测覆盖两个真实 tool call、无审批及最终成功标记。
