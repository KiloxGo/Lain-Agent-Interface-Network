# MaiBot：Lain Chat 的参考与取舍

**调研日期：** 2026-10-23

**来源：** 用户指定 [Mai-with-u/MaiBot](https://github.com/Mai-with-u/MaiBot)。

**范围：** 官方公开文档及部分源码阅读；未安装、运行或完成真实体验测试。

**版本说明：** 2026-10-23 的笔记基于未固定版本的 main 页面和在线文档；
2026-10-25 C1 已补充固定源码复核，见下一节。历史笔记不作为当前源码事实。

这份文件是聊天设计的长期依据与取舍记录，不是上游完整审计或性能报告。
不因上游已有模块就默认在 Lain 引入同样的框架、服务或模型调用数量。

## 2026-10-25 C1 固定源码复核

Git 远端连接重置后，官方 GitHub API 成功返回 main 的固定 revision：
[`062d45cf8bd4272ac57d7ec03fcf405e5a458211`](https://github.com/Mai-with-u/MaiBot/commit/062d45cf8bd4272ac57d7ec03fcf405e5a458211)，
提交时间 2026-10-23T17:46:39Z。以下是该版本的源码事实，不是在线文档默认值的推断。
本轮没有安装或运行 MaiBot，也没有比较真实模型的体验。只参考机制，未复制源码或提示词；
上游 LICENSE 与下载文件哈希留在忽略目录 `target/q2a/c1-maibot/`。

| 机制 | 固定版本源码事实 | Lain 的选择与差异 |
|---|---|---|
| 人格、行为、表达 | [official_configs.py:148–198](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/config/official_configs.py#L148-L198) 分别配置 personality、behavior_style、reply_style；[generator:105–141](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/chat/replyer/maisaka_generator_base.py#L105-L141) 分别装配身份和表达，并可随机选择备用表达。 | 拆开 Lain 身份、私聊行为和表达文件，Kernel 规则始终附加；不采用上游身份、随机人格或额外 Replyer 调用。 |
| 连发等待 | [controller.py:18–104](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/chat/debounce/controller.py#L18-L104) 用消息事件及有限 deadline；不同模式、作者与活跃情况有不同等待，批次上限 8 秒，已到期窗口不延长。 | 仅单人文字私聊，采用末条后 750ms 安静窗口、首条后最多 3 秒；不引入群聊活跃评分、图片等待或额外模型。 |
| 私聊参与 | [gates.py:64–77](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/maisaka/turn_trigger/gates.py#L64-L77) 明确跳过群聊动态门控；[runtime.py:1060–1096](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/maisaka/runtime.py#L1060-L1096) 私聊不采用群聊随机/动态频率倍数。 | 新私聊输入仍进入有界 Actor，不移植群聊概率来随机丢弃消息。模型可提议等待或结束，由 Kernel 校验状态、计数和保护条件。 |
| 等待与结束 | [wait.py](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/maisaka/builtin_tool/wait.py) 与 [stop.py](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/maisaka/builtin_tool/stop.py) 区分限时暂停和结束本轮思考；[runtime.py:1714–1791](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/maisaka/runtime.py#L1714-L1791) 管理计数、期限和新消息唤醒，计数提示并非硬强停。 | `chat.wait` 一轮最多一次、1–5 秒，可被新输入唤醒；`chat.finish` 只接受社交收尾解释，明确请求保护、来源 ID 和推断性质进入回执。限制由代码执行。 |
| 新消息转向 | [runtime.py:1007–1057](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/maisaka/runtime.py#L1007-L1057) 在可选 steering 开关下请求打断，每次在途请求只发一次；[reasoning_engine.py:608–674](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/maisaka/reasoning_engine.py#L608-L674) 保留非执行性的部分思考并在剩余轮数内接入新输入。[配置:530–541](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/config/official_configs.py#L530-L541) 默认关闭，属可选快速反应路径。 | 不假定提供方支持可靠的中途续接：等待在途响应、累计实际消耗，丢弃过时文本/调用，最多重新考虑两次。发送前再次检查；不撤回已发送，不重放 Unknown，不取消 Worker。 |
| 私聊上下文与回忆 | [chat_loop_service.py:818–909](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/src/maisaka/chat_loop_service.py#L818-L909) 分别装配人格、行为、私聊/群聊提示及记忆指引；[maisaka_replyer.prompt](https://github.com/Mai-with-u/MaiBot/blob/062d45cf8bd4272ac57d7ec03fcf405e5a458211/prompts/zh-CN/maisaka_replyer.prompt) 将身份与表达风格提供给回复器。 | 继续使用已授权、可纠正的 Q2-A/Q2-B1；每次重新考虑刷新相关证据，普通回忆使用自然措辞，内部来源留在管理回执。未确认发送的草稿不作为已说出口的历史。 |

上述 Lain 选择属于 [ADR 012](adr/012-bounded-private-conversation.md) 的实现范围。
源码对照能证明参考依据，不能证明 Lain 已获得相同自然度；实际完成和验收结果另记。

## 2026-10-25 聊天体验优先级与文档复核

以下保留为取得固定 revision 前的调研记录；当前源码复核结果以上一节为准。

用户要求聊天部分务必借鉴 MaiBot，目标是非常像活人的日常交流。
C1 已作为下一阶段优先级写入 ROADMAP/CHAT_DESIGN，仍属于待实现范围。

本次重新阅读 [官方 Bot 配置](https://docs.mai-mai.org/manual/configuration/bot-config)
（访问日 2026-10-25；页面标注更新日 2026-10-20）。在线文档把身份气质、参与行为与
语言风格分开，并描述连发等待、私聊上下文和回复期间新消息处理；其中若干能力属
实验设置。其动态触发门控明确只针对群聊，不能宣称同样适用于私聊。

Lain 的选择是优先实现可观察的私聊节奏和自然表达，再用对话场景评价；保留自身身份、
现有预算、记忆纠正和 Kernel 授权。具体等待门限、是否拆分推理阶段及不回复协议尚待
实施设计，不直接采用上游默认常数。C1 的等待/不回复范围是 Lain 的目标，非本次已验证
的 MaiBot 私聊源码行为。文档阅读也不证明任何实际聊天效果。

本次 Git 远端 main 元数据查询因连接重置未成功，未获得固定源码 revision；因此没有
声称完成当前源码复核、移植或运行验收。实施相关机制前须取得并阅读固定版本。
2026-10-23 的 main 源码笔记保留为历史参考，不充当本次固定版本验证。

## 1. 参考价值

MaiBot 将日常交流的自然程度、参与时机和逐渐了解对方作为产品重点。
这与 Lain 此次 QQ 优先和平等相处的方向相近。
参考目标是把自然相处落实为可观察的行为，而不是复制另一角色的身份。
来源：[项目介绍](https://github.com/Mai-with-u/MaiBot)。

## 2. 已阅读的机制及 Lain 设计选择

### 2.1 人格、行为与表达分离

官方配置分别描述 personality、behavior_style 和 reply_style；消息合并等待与表达学习
也有独立设置。后台子任务返回结果的能力在该文档中标为实验性。
来源：[Bot 配置](https://docs.mai-mai.org/manual/configuration/bot-config)。

Lain 的设计选择：稳定气质、何时参与和怎样表达应分别讨论，不以一大段角色提示词
承担所有职责。连发补充应合并理解。允许学习共同的梗，但不随机更换人格或模仿所有群友。
后台结果回到交流的体验可参考；工程 Worker 的执行仍归 Lain Supervisor 和外部 harness。
是否使用独立 Planner/Replyer 以及模型配置不在本轮决定。

### 2.2 回复必要性与节奏

源码中的回复评分考虑提及、问题/请求、短反应、积压消息及近期自身发言占比。
来源：[reply_necessity.py](https://github.com/Mai-with-u/MaiBot/blob/main/src/maisaka/reply_necessity.py)。

Lain 的设计选择：接收不等于必答，归档不等于立即推理。
私聊、显式工作请求和只读群观察需要不同策略。群聊打分不能直接控制任务授权，
也不能因一句文字包含“帮我”就把观察资料升级为指令。
具体权重与等待窗口通过私聊体验确定，不复制上游常数作为已确认需求。

### 2.3 不行动时的退避

源码的 IdleBackoffController 对连续群聊空闲周期进行有上限的退避，
并有焦点/待处理消息等条件；私聊不是同一条退避路径。
来源：[idle_backoff.py](https://github.com/Mai-with-u/MaiBot/blob/main/src/maisaka/idle_backoff.py)。

Lain 的设计选择：无值得处理事件时不持续调用模型；生活日程与自主阅读仍应使用
可归因的事件或定时安排。不能把群聊退避代码称为已经解决课表感知和主动联系。

### 2.4 来源记录与人物认识

消息接收代码先保存消息并接入聊天和记忆活动；人物事实写入携带聊天来源、
证据类型和消息 ID 等元数据。
来源：[消息接收](https://github.com/Mai-with-u/MaiBot/blob/main/src/chat/heart_flow/heartflow_message_processor.py)、
[人物记录](https://github.com/Mai-with-u/MaiBot/blob/main/src/person_info/person_info.py)。

Lain 的设计选择：聊天档案、处理活动、人物认识与正式记忆分开关联。
事实应保留来源；认识可以修正。不能仅凭昵称把同名联系人跨平台合并，
也不能把模型推测升级为用户原话。用户多个入口的绑定需要可靠身份关系。

### 2.5 记忆召回、共享与纠正

文档区分主动查询、人物信息注入、可选的情境召回及跨聊天范围；
自然语言修正和部分实验性反馈纠错也有独立配置。
来源：[A_Memorix 配置](https://docs.mai-mai.org/manual/configuration/amemorix-config)。

Lain 的设计选择：少量相关经历自动浮现与按需查资料并存。
共享身份不意味着对所有交流对象开放全部私人材料，所有召回路径应使用一致的范围检查。
纠正应传递到摘要、认识和缓存，避免旧结论反复回来。
不直接采用上游完整向量、图谱或人物画像系统，也不默认给人贴心理标签。
是否需要嵌入/语义检索由实际召回效果、规模和成本决定。

### 2.6 会话运行实例与统一身份

HeartflowManager 按 session 管理聊天运行实例，并有实例创建锁与淘汰机制。
来源：[heartflow_manager.py](https://github.com/Mai-with-u/MaiBot/blob/main/src/chat/heart_flow/heartflow_manager.py)。

Lain 的设计选择：不同对话可以有独立短期现场，但共同引用云端身份和记忆。
复制每个聊天实例的全部数据再双向同步，不满足“同一个 Lain”的目标。
上游按 session 运行不等于每个实例就是独立人格，也不能直接证明其跨设备一致性适合 Lain。

## 3. 不能直接从参考项目推导的结论

- 没有实测“更自然”的程度，不能把产品介绍当作用户体验验收。
- 未验证上游断线、重启、发送回执丢失的完整恢复语义。
- 未验证实验性子任务等同于 Codex/Claude/DeepSeek 的持久会话管理。
- 未验证 Lain 的双账号只读观察、统一云端身份和多 PC 执行场景。
- 没有得出必须采用 Python、特定向量库、特定模型或固定多阶段推理的结论。
- 本轮未复制上游实现；若之后选择复用源码，应记录精确版本、来源及适用的许可证要求。

## 4. 后续有价值的验证

在明确授权实际接入和模型调用后，可使用合成或脱敏对话先做对照：
连发补充、闲聊收尾、严肃任务、短反应、回忆、纠正、未回应、课表勿扰和后台结果到达。
分别观察正确性、回应时机、打扰程度、人格稳定性、成本与延迟。
不要用消息数量或主观一句“像人”替代可解释的失败案例。

Lain 采用哪些机制、为什么不采用另一些，应回写 [CHAT_DESIGN.md](CHAT_DESIGN.md)，
并保留“来源事实”和“Lain 自己的设计判断”的区别。
