use crate::{api::Client, emit};
use anyhow::{Context, Result, ensure};
use clap::{Args, Subcommand, ValueEnum};
use lain_wire::{
    CandidateId, MemoryId, RequestId, SummaryId, WorkSessionId, WorkerTurnId, memory::*,
};
use serde_json::{Value, json};

#[derive(Clone, Copy, ValueEnum)]
pub enum Kind {
    Episodic,
    ProjectKnowledge,
    Preference,
    Reflection,
}
impl From<Kind> for MemoryKind {
    fn from(k: Kind) -> Self {
        match k {
            Kind::Episodic => Self::Episodic,
            Kind::ProjectKnowledge => Self::ProjectKnowledge,
            Kind::Preference => Self::Preference,
            Kind::Reflection => Self::Reflection,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Status {
    Active,
    Disabled,
    Superseded,
}
impl From<Status> for MemoryStatus {
    fn from(s: Status) -> Self {
        match s {
            Status::Active => Self::Active,
            Status::Disabled => Self::Disabled,
            Status::Superseded => Self::Superseded,
        }
    }
}
#[derive(Args)]
pub struct Scope {
    /// Absolute workspace directory on the daemon host. No local path resolution.
    #[arg(long,conflicts_with_all=["global","internal","channel","conversation"])]
    workspace: Option<String>,
    #[arg(long, conflicts_with_all = ["internal","channel","conversation"])]
    global: bool,
    #[arg(long, conflicts_with_all = ["channel","conversation"])]
    internal: bool,
    #[arg(long, conflicts_with = "conversation")]
    channel: Option<lain_wire::ChannelId>,
    /// Exact cloud conversation; does not share another conversation's memory.
    #[arg(long)]
    conversation: Option<lain_wire::ConversationId>,
}
impl Scope {
    fn value(self) -> MemoryScope {
        if let Some(conversation_id) = self.conversation {
            MemoryScope::Conversation { conversation_id }
        } else if let Some(path) = self.workspace {
            MemoryScope::Workspace { path }
        } else if self.internal {
            MemoryScope::Internal
        } else if let Some(channel_id) = self.channel {
            MemoryScope::Channel { channel_id }
        } else {
            MemoryScope::Global
        }
    }
}
#[derive(Args)]
pub struct Filter {
    #[command(flatten)]
    scope: Scope,
    #[arg(long, value_enum)]
    kind: Option<Kind>,
    #[arg(long, value_enum, default_value = "active")]
    status: Status,
    #[arg(long, default_value_t = 0)]
    after: u64,
    #[arg(long,default_value_t=20,value_parser=clap::value_parser!(u16).range(1..=50))]
    limit: u16,
}
impl Filter {
    fn query(self, text: Option<String>) -> MemoryQuery {
        MemoryQuery {
            scope: self.scope.value(),
            kind: self.kind.map(Into::into),
            status: Some(self.status.into()),
            text,
            after: self.after,
            limit: self.limit,
        }
    }
}
#[derive(Subcommand)]
pub enum Commands {
    List {
        #[command(flatten)]
        filter: Filter,
    },
    Search {
        text: String,
        #[command(flatten)]
        filter: Filter,
    },
    Show {
        id: MemoryId,
    },
    /// Direct owner-authored statement; this command itself authorizes the write.
    Add {
        content: String,
        #[arg(long, value_enum)]
        kind: Kind,
        #[command(flatten)]
        scope: Scope,
        #[arg(long)]
        request_id: Option<RequestId>,
    },
    Disable {
        id: MemoryId,
        #[arg(long)]
        revision: u64,
    },
    Enable {
        id: MemoryId,
        #[arg(long)]
        revision: u64,
    },
    Supersede {
        id: MemoryId,
        #[arg(long)]
        with: MemoryId,
        #[arg(long)]
        revision: u64,
    },
    Delete {
        id: MemoryId,
        #[arg(long)]
        revision: u64,
    },
    Candidates {
        #[arg(long, default_value_t = 0)]
        after: u64,
    },
    Candidate {
        id: CandidateId,
    },
    Accept {
        id: CandidateId,
    },
    Reject {
        id: CandidateId,
    },
    /// Select project knowledge from an inspected SessionSummary for review.
    FromSummary {
        summary: SummaryId,
        content: String,
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        request_id: Option<RequestId>,
    },
}
#[derive(Subcommand)]
pub enum Summaries {
    List {
        session: WorkSessionId,
    },
    Show {
        id: SummaryId,
    },
    Create {
        session: WorkSessionId,
        #[arg(long)]
        turn: WorkerTurnId,
    },
}
pub async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    let value: Value = match command {
        Commands::List { filter } => {
            let page = client
                .post::<MemoryPage>("v1/memory/query", Some(json!(filter.query(None))))
                .await?;
            serde_json::to_value(page)?
        }
        Commands::Search { text, filter } => {
            let page = client
                .post::<MemoryPage>("v1/memory/query", Some(json!(filter.query(Some(text)))))
                .await?;
            serde_json::to_value(page)?
        }
        Commands::Show { id } => serde_json::to_value(
            client
                .get::<MemoryRecord>(&format!("v1/memory/{id}"))
                .await?,
        )?,
        Commands::Add {
            content,
            kind,
            scope,
            request_id,
        } => {
            let input = AddMemory {
                request_id: request_id.unwrap_or_default(),
                draft: MemoryDraft {
                    content,
                    kind: kind.into(),
                    scope: scope.value(),
                },
            };
            client.post("v1/memory", Some(json!(input))).await?
        }
        Commands::Disable { id, revision } | Commands::Enable { id, revision } => {
            let status = if matches!(command, Commands::Enable { .. }) {
                MemoryStatus::Active
            } else {
                MemoryStatus::Disabled
            };
            let change = ChangeMemory {
                expected_revision: revision,
                status,
                superseded_by: None,
            };
            client
                .post(&format!("v1/memory/{id}/status"), Some(json!(change)))
                .await?
        }
        Commands::Supersede { id, with, revision } => {
            let change = ChangeMemory {
                expected_revision: revision,
                status: MemoryStatus::Superseded,
                superseded_by: Some(with),
            };
            client
                .post(&format!("v1/memory/{id}/status"), Some(json!(change)))
                .await?
        }
        Commands::Delete { id, revision } => {
            client
                .post(
                    &format!("v1/memory/{id}/delete"),
                    Some(json!({"expected_revision": revision})),
                )
                .await?
        }
        Commands::Candidates { after } => {
            client
                .get(&format!("v1/memory-candidates?after={after}&limit=50"))
                .await?
        }
        Commands::Candidate { id } => client.get(&format!("v1/memory-candidates/{id}")).await?,
        Commands::Accept { id } | Commands::Reject { id } => {
            let accept = matches!(command, Commands::Accept { .. });
            client
                .post(
                    &format!("v1/memory-candidates/{id}/decision"),
                    Some(json!({"accept": accept})),
                )
                .await?
        }
        Commands::FromSummary {
            summary,
            content,
            workspace,
            request_id,
        } => {
            let draft = MemoryDraft {
                content,
                kind: MemoryKind::ProjectKnowledge,
                scope: MemoryScope::Workspace { path: workspace },
            };
            client.post("v1/memory-candidates", Some(json!({
                "request_id": request_id.unwrap_or_default(), "summary_id": summary, "draft": draft,
            }))).await?
        }
    };
    if !machine && value.get("records").is_some() {
        let page: MemoryPage = serde_json::from_value(value)?;
        for record in page.records {
            println!(
                "{}  {:?}  {:?}  {}\n  {}",
                record.id,
                record.status,
                record.draft.kind,
                crate::tui::safe_text(&serde_json::to_string(&record.draft.scope)?),
                crate::tui::safe_text(&record.draft.content)
            );
        }
        if page.has_more {
            println!("More: --after {}", page.next_cursor);
        }
        Ok(())
    } else {
        emit(&value, machine)
    }
}
pub async fn summaries(client: &Client, command: Summaries, machine: bool) -> Result<()> {
    let value: Value = match command {
        Summaries::List { session } => {
            client
                .get(&format!("v1/work-sessions/{session}/summaries"))
                .await?
        }
        Summaries::Show { id } => client.get(&format!("v1/session-summaries/{id}")).await?,
        Summaries::Create { session, turn } => {
            client
                .post(
                    &format!("v1/work-sessions/{session}/summaries"),
                    Some(json!({"turn_id":turn})),
                )
                .await?
        }
    };
    emit(&value, machine)
}

/// Shared exact draft inspection for both desk and legacy chat confirmations.
pub(crate) fn briefing_review(
    turn: &lain_wire::ActorSnapshot,
    action: &lain_wire::ActorAction,
) -> Result<String> {
    if let lain_wire::ToolCall::BriefingRemember(q) = &action.tool {
        use lain_wire::memory::*;
        let c = action
            .memory_review
            .as_ref()
            .context("记忆预览不可用；请刷新或用 /stop 取消")?;
        c.validate().map_err(anyhow::Error::msg)?;
        let MemorySource::BriefingSelection { evidence: e } = &c.provenance.source else {
            anyhow::bail!("记忆预览来源不匹配")
        };
        ensure!(
            action.state == lain_wire::ActionState::Proposed
                && c.status == CandidateStatus::Candidate
                && c.id.0 == action.id.0
                && c.actor_action == Some(action.id)
                && e.actor_turn_id == turn.id
                && e.conversation_id == turn.conversation_id
                && e.briefing_id == q.briefing_id,
            "记忆预览身份不匹配"
        );
        ensure!(
            e.reconciliation
                .as_ref()
                .is_none_or(|r| q.include_related && r.member_numbers.contains(&q.item_number)),
            "关联事项预览与实际选择不匹配"
        );
        let related = e
            .reconciliation
            .as_ref()
            .map(|r| {
                format!(
                    "\n整组关联：原编号 {:?}；完整发现/核对依赖保存在服务端，不只上述主批次。",
                    r.member_numbers
                )
            })
            .unwrap_or_default();
        return Ok(crate::tui::safe_text(&format!(
            "保存来源事项 · 第 {} 条\n本次操作：新增 1 条记忆；不代表报名、承诺参加或核实通知。\n可见范围：仅当前会话\n有效期：{}\n\n实际保存内容：\n{}\n\n来源与关联影响：\n性质：来源通知的模型概括，未独立核实；本次确认只授权保存。\n原文依据：主批次 {} 条输入，包含未引用的输入。{}\n来源或输入变化后，这条记忆可能停止召回；上述有效期不是可读性保证。\n今后更正或忘记时，共用旧输入的其他来源记忆也可能变得不可用，但不会因此一并删除其他记忆记录。\n保存不扩大来源阅读或跨会话共享权限；原始消息和操作历史遵循各自保留规则。",
            q.item_number,
            review_expiry(e.expires_at_ms),
            c.draft.content,
            e.records.len(),
            related
        )));
    }
    anyhow::bail!("不是来源事项保存提案")
}

pub(crate) fn selection_review(
    turn: &lain_wire::ActorSnapshot,
    action: &lain_wire::ActorAction,
) -> Result<String> {
    let lain_wire::ToolCall::MemorySelectionChange(q) = &action.tool else {
        anyhow::bail!("不是记忆管理提案")
    };
    let r = action
        .memory_change_review
        .as_ref()
        .context("记忆管理预览不可用；请刷新或用 /stop 取消")?;
    turn.validate().map_err(anyhow::Error::msg)?;
    let original = turn
        .messages
        .iter()
        .find(|m| m.id == q.message_id && m.role == lain_wire::MessageRole::User)
        .context("缺少本轮完整用户原话")?;
    ensure!(
        r.request == *q
            && r.actor_turn_id == turn.id
            && r.action_id == action.id
            && r.target.conversation_id == turn.conversation_id
            && action.state == lain_wire::ActionState::Proposed
            && r.valid_replacement()
            && original.content.contains(&q.quote),
        "记忆管理预览与用户原话不匹配"
    );
    let user_text = if original.content == q.quote {
        original.content.clone()
    } else {
        format!("{}\n\n模型选取的引文：\n{}", original.content, q.quote)
    };
    let state = match r.target.status {
        MemoryStatus::Active => "已启用（不代表来源或旧正文仍可读取）",
        MemoryStatus::Disabled => "已停用",
        MemoryStatus::Superseded => "已被替代",
    };
    let identity = format!(
        "核对信息：\n目标：{} · 版本 {}\n记录状态：{}\n原保存时间：{}",
        q.memory_id,
        q.expected_revision,
        state,
        review_expiry(r.target.recorded_at_ms)
    );
    let boundaries = "关联影响：\n如果仍有旧来源输入依赖，共用这些输入的其他来源记忆也可能变得不可用。这里只处理选中的 1 条，不会一并删除其他记忆记录；不能据此推定其他事项仍可召回。\n不会恢复已撤回的来源，也不删除 QQ 平台消息；原始档案和操作历史遵循各自保留规则。";
    if q.operation == MemoryOperation::Correct {
        let c = r.replacement.as_ref().context("缺少实际更正草稿")?;
        return Ok(crate::tui::safe_text(&format!(
            "更正已保存事项\n本次操作：用下方新内容替代选中的 1 条旧记忆；旧正文不再用于当前召回。\n范围：仅当前会话\n有效期不延长：{}\n\n实际保存的新内容（用户更正，未独立核实）：\n{}\n\n你的完整原话：\n{}\n\n{}\n\n{}",
            review_expiry(r.target.expires_at_ms),
            c.draft.content,
            user_text,
            boundaries,
            identity
        )));
    }
    Ok(crate::tui::safe_text(&format!(
        "忘记来源事项\n本次操作：删除选中的 1 条语义记忆及检索副本，并抑制从关联旧输入自动重新记住。\n范围：仅当前会话\n原有效期：{}\n本页只展示管理信息，不重现旧正文。\n\n你的原话：\n{}\n\n{}\n\n{}",
        review_expiry(r.target.expires_at_ms),
        user_text,
        boundaries,
        identity
    )))
}

fn review_expiry(ms: u64) -> String {
    let date = i64::try_from(ms / 1000)
        .ok()
        .and_then(|s| time::OffsetDateTime::from_unix_timestamp(s).ok());
    date.map(|d| {
        format!(
            "{}-{:02}-{:02} {:02}:{:02} UTC",
            d.year(),
            u8::from(d.month()),
            d.day(),
            d.hour(),
            d.minute()
        )
    })
    .unwrap_or_else(|| "时间未知".into())
}
