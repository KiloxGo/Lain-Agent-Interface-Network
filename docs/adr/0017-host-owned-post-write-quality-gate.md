# ADR 0017：宿主拥有的写后质量门与嵌套文件创建

- 状态：Accepted
- 日期：2026-08-30
- 决策者：KiloxGo

## 背景

工作循环已有 inspect/edit/check/report 提示，但模型成功写入后只收到普通工具回执；空项目还必须先用平台 shell 创建父目录。前者无法确定性提醒复查，后者在 Windows 上引入了不必要的 shell 语法和审批失败面。

## 决策

1. `create_file` 继续保持单文件、UTF-8、1 MiB、no-clobber 和一次性 diff 审批，但允许目标的父目录尚不存在。
2. prepare 阶段只校验 workspace-relative 路径、排除项、平台保留名、现存父路径的类型与 symlink，不产生目录。
3. approval 后逐组件创建目录；每一步重新拒绝 symlink/非目录/竞态替换，随后再次解析目标并使用原有 hard-link no-clobber 发布。
4. 成功的 `replace_text`/`create_file` 后，host 在下一次模型请求中加入固定、可信的 system quality gate；成功的 `run_check` 清除本批写后检查需求。
5. `run_check` 非零退出、执行错误、拒绝或不可用均产生固定、可信的恢复 gate。动态路径和工具输出不进入 system 文本，避免把不可信数据提权为指令。

本 ADR 仅替代 ADR 0008 中“父目录必须存在/不自动创建父目录”的条款；其余 no-clobber 与审批约束不变。

## 不采用

- 自动无条件运行测试：无法可靠判断任意项目的正确命令，且会产生额外副作用审批。
- 多文件事务与 checkpoint：需要 rollback journal 和更大的协议面，待真实多文件失败轨迹证明必要后再做。
- 把提醒附在工具输出：workspace 输出按协议是不可信数据，不能承载宿主策略。

## 验证

- 单测覆盖 prepare 不落盘、审批后递归建目录、目标 no-clobber 与 command exit 判定。
- `coding-recover-node-001` 必须保持 `1 -> fix -> 0` 的检查闭环。
- `coding-nested-create-review-001` 必须不用 shell 创建两个嵌套文件，并在最终回复前复读它们。
