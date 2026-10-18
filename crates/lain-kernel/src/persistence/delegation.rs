//! Projection writes share the worker/history transaction. No second event bus,
//! worker status database or model invocation is required to create a notice.
use super::Store;
use crate::{
    actor::{
        domain::{ActionState, ActorTurn, ConversationId},
        tools::Tool,
    },
    backend::ControlError,
    delegation::*,
    domain::{TurnState, WorkSession, now_ms},
    events::{Event, EventKind},
};
use anyhow::{Context, Result, ensure};
use rusqlite::{OptionalExtension, Transaction, params};

impl Store {
    pub(super) fn record_delegation(
        tx: &Transaction<'_>,
        actor: &ActorTurn,
        session: &WorkSession,
    ) -> Result<()> {
        for action in actor
            .actions
            .iter()
            .filter(|a| a.state == ActionState::Executing)
        {
            let Some(receipt) = &action.result else {
                continue;
            };
            if receipt.get("work_session_id").and_then(|v| v.as_str())
                != Some(&session.id.to_string())
            {
                continue;
            }
            let instruction = match &action.tool {
                Tool::WorkCreate(args) => &args.task,
                Tool::WorkSend(args) => &args.text,
                _ => continue,
            };
            let turn = receipt
                .get("turn_id")
                .and_then(|v| v.as_str())
                .map(str::parse)
                .transpose()?;
            let previous: Option<String> = tx
                .query_row(
                    "SELECT snapshot FROM lain_delegations WHERE action_id = ?1",
                    [action.id.to_string()],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(previous) = previous {
                let previous: Delegation =
                    serde_json::from_str(&previous).context("invalid delegation record")?;
                ensure!(
                    previous.conversation_id == actor.conversation_id
                        && previous.actor_turn_id == actor.id
                        && previous.work_session_id == session.id
                        && previous.instruction == *instruction
                        && previous.worker_turn_id.is_none_or(|id| Some(id) == turn),
                    ControlError::Conflict
                );
            }
            let link = Delegation {
                conversation_id: actor.conversation_id,
                actor_turn_id: actor.id,
                action_id: action.id,
                work_session_id: session.id,
                worker_turn_id: turn,
                workspace: session.workspace.clone(),
                profile_id: session.runtime_profile_id.clone(),
                instruction: instruction.clone(),
                submission: if turn.is_some() {
                    SubmissionState::Submitting
                } else {
                    SubmissionState::Prepared
                },
                receipt: Some(receipt.clone()),
            };
            tx.execute("INSERT INTO lain_delegations(action_id, conversation_id, actor_turn_id, session_id, worker_turn_id, snapshot) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(action_id) DO UPDATE SET worker_turn_id = excluded.worker_turn_id, snapshot = excluded.snapshot",
                params![link.action_id.to_string(), link.conversation_id.to_string(), link.actor_turn_id.to_string(), session.id.to_string(), turn.map(|t| t.to_string()), serde_json::to_string(&link)?])?;
        }
        Ok(())
    }

    pub(super) fn project_delegation_event(tx: &Transaction<'_>, event: &Event) -> Result<()> {
        let turn = match &event.kind {
            EventKind::TurnStarted { turn }
            | EventKind::TurnCompleted { turn }
            | EventKind::TurnStateChanged { turn } => turn,
            _ => return Ok(()),
        };
        let value: Option<String> = tx.query_row("SELECT snapshot FROM lain_delegations WHERE worker_turn_id = ?1 AND session_id = ?2",
            (turn.id.to_string(), turn.work_session_id.to_string()), |r| r.get(0)).optional()?;
        let Some(value) = value else {
            return Ok(());
        };
        let mut link: Delegation =
            serde_json::from_str(&value).context("invalid delegation record")?;
        link.submission = if turn.state == TurnState::Starting {
            SubmissionState::Submitting
        } else if turn
            .failure
            .as_ref()
            .is_some_and(|f| f.kind == crate::domain::FailureKind::Rejected)
        {
            SubmissionState::Rejected
        } else if turn.state == TurnState::Unknown && link.submission != SubmissionState::Accepted {
            SubmissionState::Unknown
        } else {
            SubmissionState::Accepted
        };
        link.receipt = Some(
            serde_json::json!({"work_session_id":turn.work_session_id,"turn_id":turn.id,"submission":link.submission}),
        );
        tx.execute(
            "UPDATE lain_delegations SET snapshot = ?2 WHERE action_id = ?1",
            (link.action_id.to_string(), serde_json::to_string(&link)?),
        )?;
        if !turn.state.is_terminal() && turn.state != TurnState::Unknown {
            return Ok(());
        }
        let key = serde_json::to_string(&turn.state)?;
        let already_projected: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM lain_conversation_notices WHERE action_id = ?1 AND outcome_key = ?2)", (link.action_id.to_string(), &key), |r| r.get(0))?;
        if already_projected {
            return Ok(());
        }
        let evidence = Self::delegation_evidence(
            tx,
            &link,
            event.history_order.context("uncommitted terminal event")?,
        )?;
        let notice = ConversationNotice {
            id: NoticeId::new(),
            cursor: 0,
            conversation_id: link.conversation_id,
            actor_turn_id: link.actor_turn_id,
            action_id: link.action_id,
            work_session_id: link.work_session_id,
            worker_turn_id: link.worker_turn_id,
            timestamp_ms: event.timestamp_ms,
            source_history_order: event.history_order,
            outcome: NoticeOutcome::Worker { state: turn.state },
            evidence,
            acknowledged: false,
        };
        Self::insert_notice(tx, &key, &notice)
    }

    fn delegation_evidence(
        tx: &Transaction<'_>,
        link: &Delegation,
        until: u64,
    ) -> Result<ResultEvidence> {
        let mut result = ResultEvidence::default();
        let mut query = tx.prepare("SELECT payload FROM operational_history WHERE session_id = ?1 AND turn_id = ?2 AND sequence <= ?3 ORDER BY sequence")?;
        let rows = query.query_map(
            params![
                link.work_session_id.to_string(),
                link.worker_turn_id
                    .context("missing delegated turn")?
                    .to_string(),
                i64::try_from(until)?
            ],
            |r| r.get::<_, String>(0),
        )?;
        for row in rows {
            let event: Event = serde_json::from_str(&row?).context("invalid delegated history")?;
            result.truncated |= event.truncated;
            match event.kind {
                EventKind::AssistantMessageCompleted { text } => {
                    result.final_assistant_message =
                        Some(bounded(&text, 8192, &mut result.truncated));
                }
                EventKind::CommandCompleted {
                    command, exit_code, ..
                } if exit_code.is_some_and(|code| code != 0) => {
                    result.failed_command_count += 1;
                    if result.failed_commands.len() < 8 {
                        result.failed_commands.push(CommandFailure {
                            command: bounded(&command, 512, &mut result.truncated),
                            exit_code,
                        });
                    } else {
                        result.truncated = true;
                    }
                }
                EventKind::FileChanged { paths } => {
                    for path in paths {
                        let path = bounded(&path, 512, &mut result.truncated);
                        if !result.changed_files.contains(&path) {
                            if result.changed_files.len() < 16 {
                                result.changed_files.push(path);
                            } else {
                                result.truncated = true;
                            }
                        }
                    }
                }
                EventKind::ErrorObserved { mut failure }
                | EventKind::BackendDisconnected { mut failure } => {
                    failure.message = bounded(&failure.message, 512, &mut result.truncated);
                    if result.errors.len() < 4 {
                        result.errors.push(failure);
                    } else {
                        result.truncated = true;
                    }
                }
                EventKind::TurnCompleted { turn } | EventKind::TurnStateChanged { turn } => {
                    if let Some(mut failure) = turn.failure {
                        failure.message = bounded(&failure.message, 512, &mut result.truncated);
                        if !result.errors.contains(&failure) && result.errors.len() < 4 {
                            result.errors.push(failure);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(result)
    }

    fn insert_notice(tx: &Transaction<'_>, key: &str, notice: &ConversationNotice) -> Result<()> {
        tx.execute("INSERT INTO lain_conversation_notices(id, conversation_id, action_id, outcome_key, snapshot) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(action_id, outcome_key) DO NOTHING",
            params![notice.id.to_string(), notice.conversation_id.to_string(), notice.action_id.to_string(), key, serde_json::to_string(notice)?])?;
        tx.execute(
            "UPDATE lain_conversations SET updated_at_ms = max(updated_at_ms, ?2) WHERE id = ?1",
            (
                notice.conversation_id.to_string(),
                i64::try_from(notice.timestamp_ms)?,
            ),
        )?;
        Ok(())
    }

    pub(super) fn settle_delegation_submission(
        tx: &Transaction<'_>,
        actor: &ActorTurn,
    ) -> Result<()> {
        for action in actor
            .actions
            .iter()
            .filter(|a| matches!(a.state, ActionState::Failed | ActionState::Unknown))
        {
            let value: Option<String> = tx.query_row("SELECT snapshot FROM lain_delegations WHERE action_id = ?1 AND worker_turn_id IS NULL", [action.id.to_string()], |r| r.get(0)).optional()?;
            let Some(value) = value else {
                continue;
            };
            let mut link: Delegation =
                serde_json::from_str(&value).context("invalid delegation record")?;
            link.submission = SubmissionState::Unknown;
            tx.execute(
                "UPDATE lain_delegations SET snapshot = ?2 WHERE action_id = ?1",
                (link.action_id.to_string(), serde_json::to_string(&link)?),
            )?;
            Self::insert_notice(
                tx,
                "submission_unknown",
                &ConversationNotice {
                    id: NoticeId::new(),
                    cursor: 0,
                    conversation_id: link.conversation_id,
                    actor_turn_id: link.actor_turn_id,
                    action_id: link.action_id,
                    work_session_id: link.work_session_id,
                    worker_turn_id: None,
                    timestamp_ms: now_ms()?,
                    source_history_order: None,
                    outcome: NoticeOutcome::SubmissionUnknown,
                    evidence: ResultEvidence::default(),
                    acknowledged: false,
                },
            )?;
        }
        Ok(())
    }

    pub fn delegations(&self, conversation: ConversationId) -> Result<Vec<Delegation>> {
        let mut query = self.connection.prepare(
            "SELECT snapshot FROM lain_delegations WHERE conversation_id = ?1 ORDER BY rowid",
        )?;
        query
            .query_map([conversation.to_string()], |r| r.get::<_, String>(0))?
            .map(|r| serde_json::from_str(&r?).context("invalid delegation record"))
            .collect()
    }

    pub fn notices(
        &self,
        conversation: ConversationId,
        after: u64,
        limit: usize,
    ) -> Result<NoticePage> {
        ensure!(
            after <= i64::MAX as u64 && (1..=100).contains(&limit),
            ControlError::InvalidInput
        );
        // Validate the conversation even when it has no notices.
        self.actor_page(conversation, 0, 1)?;
        let mut query = self.connection.prepare("SELECT sequence, acknowledged, snapshot FROM lain_conversation_notices WHERE conversation_id = ?1 AND sequence > ?2 ORDER BY sequence LIMIT ?3")?;
        let rows = query.query_map(
            params![
                conversation.to_string(),
                i64::try_from(after)?,
                i64::try_from(limit + 1)?
            ],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, bool>(1)?,
                    r.get::<_, String>(2)?,
                ))
            },
        )?;
        let mut notices: Vec<ConversationNotice> = rows
            .map(|row| {
                let (cursor, acknowledged, value) = row?;
                let mut notice: ConversationNotice =
                    serde_json::from_str(&value).context("invalid conversation notice")?;
                notice.cursor = cursor.try_into()?;
                notice.acknowledged = acknowledged;
                Ok(notice)
            })
            .collect::<Result<_>>()?;
        let has_more = notices.len() > limit;
        notices.truncate(limit);
        Ok(NoticePage {
            next_cursor: notices.last().map(|n| n.cursor).unwrap_or(after),
            notices,
            has_more,
        })
    }

    pub fn acknowledge_notice(&mut self, conversation: ConversationId, id: NoticeId) -> Result<()> {
        ensure!(self.connection.execute("UPDATE lain_conversation_notices SET acknowledged = 1 WHERE conversation_id = ?1 AND id = ?2", (conversation.to_string(), id.to_string()))? == 1, ControlError::NotFound);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actor::{
            domain::{Action, ActionId, ActorTurn, ActorTurnId, MessageRole},
            tools::CreateArg,
        },
        domain::{CancellationState, RuntimeAttachment, Turn, TurnId, WorkSessionState},
        worker::{TurnRecord, WorkerState},
    };

    #[test]
    fn notice_commit_is_atomic_idempotent_and_recovers_without_publication() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("atomic.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let c = store.create_conversation().unwrap();
        let session = WorkSession::new(
            "codex".parse().unwrap(),
            "codex/local".parse().unwrap(),
            dir.path().to_owned(),
        )
        .unwrap();
        let tid = TurnId::new();
        let mut actor = ActorTurn {
            id: ActorTurnId::new(),
            conversation_id: c.id,
            request_id: uuid::Uuid::new_v4(),
            sequence: 0,
            revision: 0,
            state: crate::actor::domain::TurnState::Running,
            messages: vec![],
            actions: vec![],
            error: None,
            model_requests: 1,
            usage_tokens: 0,
            timing: Default::default(),
        };
        actor
            .message(MessageRole::User, "original user request".into())
            .unwrap();
        actor.actions.push(Action { id: ActionId::new(), call_id: "fixture".into(), tool: Tool::WorkCreate(CreateArg { workspace: dir.path().to_owned(), profile_id: "codex/local".parse().unwrap(), sandbox: crate::domain::SandboxMode::ReadOnly, task: "exact instruction".into() }), state: ActionState::Executing, result: Some(serde_json::json!({"work_session_id":session.id,"turn_id":tid,"stage":"submitting"})) });
        store.save_actor_turn(&mut actor, true).unwrap();
        let mut worker = WorkerState {
            turns: vec![TurnRecord {
                provider_turn_id: Some("native".into()),
                turn: Turn {
                    id: tid,
                    work_session_id: session.id,
                    state: TurnState::Running,
                    cancellation: CancellationState::NotRequested,
                    created_at_ms: 1,
                    updated_at_ms: 1,
                    failure: None,
                },
            }],
            ..Default::default()
        };
        store
            .save_with_actor_receipt(&session, Some(&worker), true, &mut [], Some(&mut actor))
            .unwrap();
        let before = store.history_cursor().unwrap();
        worker.turns[0].turn.state = TurnState::Completed;
        let mut terminal = Event::new(
            session.id,
            Some(tid),
            2,
            EventKind::TurnCompleted {
                turn: worker.turns[0].turn.clone(),
            },
        );
        store.connection.execute_batch("CREATE TRIGGER reject_notice BEFORE INSERT ON lain_conversation_notices BEGIN SELECT RAISE(ABORT, 'fixture notice failure'); END;").unwrap();
        assert!(
            store
                .save(
                    &session,
                    Some(&worker),
                    false,
                    std::slice::from_mut(&mut terminal)
                )
                .is_err()
        );
        assert_eq!(store.history_cursor().unwrap(), before);
        assert!(store.notices(c.id, 0, 100).unwrap().notices.is_empty());
        let persisted: String = store
            .connection
            .query_row(
                "SELECT snapshot FROM worker_state WHERE session_id = ?1",
                [session.id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<WorkerState>(&persisted)
                .unwrap()
                .turns[0]
                .turn
                .state,
            TurnState::Running
        );
        store
            .connection
            .execute_batch("DROP TRIGGER reject_notice;")
            .unwrap();
        store
            .save(
                &session,
                Some(&worker),
                false,
                std::slice::from_mut(&mut terminal),
            )
            .unwrap();
        let first = store.notices(c.id, 0, 100).unwrap().notices.remove(0);
        // A repeated normalized terminal transition does not create a new notice.
        let mut repeated = Event::new(session.id, Some(tid), 3, terminal.kind.clone());
        store
            .append_events(std::slice::from_mut(&mut repeated))
            .unwrap();
        assert_eq!(store.notices(c.id, 0, 100).unwrap().notices.len(), 1);
        // No broadcast was ever published: emulate death after COMMIT, before
        // publication, then open the real recovery path against that database.
        drop(store);
        let kernel = crate::kernel::Kernel::open(&path).unwrap();
        let recovered = kernel.notices(c.id, 0, 100).unwrap().notices.remove(0);
        assert_eq!(recovered.id, first.id);
        assert_eq!(recovered.cursor, first.cursor);
        assert!(!recovered.acknowledged);
        assert_eq!(
            kernel.turns(session.id).unwrap()[0].state,
            TurnState::Completed
        );
        assert_eq!(
            kernel.session(session.id).unwrap().unwrap().runtime,
            RuntimeAttachment::Detached
        );
        assert_eq!(
            kernel.session(session.id).unwrap().unwrap().state,
            WorkSessionState::Created
        );
    }
}
