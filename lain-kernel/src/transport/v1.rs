use std::{collections::BTreeMap, time::Duration};

use axum::{
    extract::{
        WebSocketUpgrade,
        ws::{CloseFrame, Message, WebSocket, close_code},
    },
    http::{HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt, stream::SplitStream};
use serde::Serialize;
use serde_json::json;
use tokio::{sync::mpsc, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::{
    app::{AppState, VisibleTextFilter, classify_route},
    context::{CompileInput, ContextBudget, ContextCompiler, RelationshipState},
    domain::event::PROTOCOL_VERSION,
    error::AppError,
    memory,
    protocol::{ClientCommand, ErrorEvent, PongEvent, ServerHello, SnapshotEvent, transient_event},
    provider::{FinishReason, ModelEvent, ModelRequest, RouteKind, TokenUsage, ToolChoice},
    storage::{CompletionUsage, RunRecord, Storage, StorageError},
};

const OUTBOUND_CAPACITY: usize = 64;
const OUTBOUND_WAIT: Duration = Duration::from_secs(2);
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
const MAX_REPLAY_EVENTS: i64 = 500;

enum Outbound {
    Frame(Message),
    FlushClose,
}

pub async fn upgrade(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    state: AppState,
) -> Result<Response, AppError> {
    if !state.readiness.is_ready()
        || state.provider.is_none()
        || state.identity.is_none()
        || state.storage.is_none()
    {
        let failures = state
            .readiness
            .checks()
            .iter()
            .filter(|check| !check.ready)
            .map(|check| check.name)
            .collect::<Vec<_>>()
            .join(",");
        return Err(AppError::NotReady(failures));
    }
    let request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let mut response = ws
        .max_message_size(state.config.max_message_bytes)
        .max_frame_size(state.config.max_message_bytes)
        .on_upgrade(move |socket| handle_socket(socket, state, request_id))
        .into_response();
    response.headers_mut().insert(
        "x-lain-protocol",
        HeaderValue::from_static("lain-session-v1"),
    );
    Ok(response)
}

async fn handle_socket(socket: WebSocket, state: AppState, connection_id: String) {
    let (sink, mut incoming) = socket.split();
    let (outbound, receiver) = mpsc::channel(OUTBOUND_CAPACITY);
    let writer_shutdown = state.shutdown.child_token();
    let writer = tokio::spawn(writer_loop(sink, receiver, writer_shutdown));

    let hello = timeout(HELLO_TIMEOUT, incoming.next()).await;
    let negotiated = matches!(
        hello,
        Ok(Some(Ok(Message::Text(ref text))))
            if matches!(
                serde_json::from_str::<ClientCommand>(text),
                Ok(ClientCommand::Hello { versions }) if versions.contains(&PROTOCOL_VERSION)
            )
    );
    if !negotiated {
        let _ = send_json(&outbound, &ErrorEvent::new("unsupported_protocol", false)).await;
        let _ = send_close(
            &outbound,
            close_code::PROTOCOL,
            "protocol negotiation failed",
        )
        .await;
        drop(outbound);
        let _ = writer.await;
        return;
    }

    let Some(storage) = state.storage.clone() else {
        drop(outbound);
        let _ = writer.await;
        return;
    };
    if send_json(
        &outbound,
        &ServerHello::new(&connection_id, storage.scope()),
    )
    .await
    .is_err()
    {
        drop(outbound);
        let _ = writer.await;
        return;
    }

    let mut subscribed_session: Option<String> = None;
    loop {
        let incoming_message = tokio::select! {
            () = state.shutdown.cancelled() => break,
            message = incoming.next() => message,
        };
        let Some(Ok(message)) = incoming_message else {
            break;
        };
        let command = match message {
            Message::Text(text) => match serde_json::from_str::<ClientCommand>(&text) {
                Ok(command) => command,
                Err(_) => {
                    if send_json(&outbound, &ErrorEvent::new("invalid_command", true))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    continue;
                }
            },
            Message::Ping(payload) => {
                if send_frame(&outbound, Message::Pong(payload)).await.is_err() {
                    break;
                }
                continue;
            }
            Message::Pong(_) => continue,
            Message::Close(_) => {
                let _ = send_outbound(&outbound, Outbound::FlushClose).await;
                break;
            }
            Message::Binary(_) => {
                let _ = send_json(&outbound, &ErrorEvent::new("binary_not_supported", true)).await;
                continue;
            }
        };

        match command {
            ClientCommand::Hello { .. } => {
                if send_json(&outbound, &ErrorEvent::new("hello_already_completed", true))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            ClientCommand::SessionSubscribe {
                session_id,
                after_seq,
            } => match subscribe(&storage, &outbound, &session_id, after_seq).await {
                Ok(()) => subscribed_session = Some(session_id),
                Err(_) => {
                    if send_json(&outbound, &ErrorEvent::new("session_not_found", true))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            },
            ClientCommand::SessionUnsubscribe { session_id } => {
                if subscribed_session.as_deref() == Some(session_id.as_str()) {
                    subscribed_session = None;
                }
            }
            ClientCommand::Ping { nonce } => {
                if send_json(&outbound, &PongEvent::new(nonce.as_deref()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            ClientCommand::RunCancel { .. } => {
                if send_json(&outbound, &ErrorEvent::new("no_active_run", true))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            ClientCommand::MessageCreate {
                session_id,
                client_message_id,
                text,
            } => {
                if subscribed_session.as_deref() != Some(session_id.as_str()) {
                    if send_json(&outbound, &ErrorEvent::new("session_not_subscribed", true))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    continue;
                }
                if text.trim().is_empty()
                    || text.len() > state.config.max_message_bytes
                    || client_message_id.is_empty()
                    || client_message_id.len() > 128
                {
                    if send_json(&outbound, &ErrorEvent::new("invalid_message", true))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    continue;
                }
                let route_kind = classify_route(&text);
                let route = state.router.route(route_kind);
                let Some(identity) = state.identity.as_ref() else {
                    if send_json(&outbound, &ErrorEvent::new("identity_unavailable", true))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    continue;
                };
                let accepted = match storage
                    .accept_user_input(
                        &session_id,
                        &client_message_id,
                        &text,
                        "deepseek",
                        &route.model,
                        identity.version(),
                    )
                    .await
                {
                    Ok(accepted) => accepted,
                    Err(error) => {
                        tracing::error!(error = %error, "user input transaction failed");
                        if send_json(&outbound, &ErrorEvent::new("storage_error", true))
                            .await
                            .is_err()
                        {
                            break;
                        }
                        continue;
                    }
                };
                for event in &accepted.events {
                    if send_json(&outbound, event).await.is_err() {
                        break;
                    }
                }
                if accepted.deduplicated {
                    continue;
                }
                match execute_run(
                    &state,
                    &storage,
                    &outbound,
                    &mut incoming,
                    accepted.run,
                    route_kind,
                )
                .await
                {
                    RunOutcome::Continue => {}
                    RunOutcome::ConnectionClosed => break,
                }
            }
        }
    }

    drop(outbound);
    let _ = writer.await;
}

enum RunOutcome {
    Continue,
    ConnectionClosed,
}

async fn execute_run(
    state: &AppState,
    storage: &Storage,
    outbound: &mpsc::Sender<Outbound>,
    incoming: &mut SplitStream<WebSocket>,
    run: RunRecord,
    route_kind: crate::provider::RouteKind,
) -> RunOutcome {
    let started = match storage.start_run(&run.id).await {
        Ok(event) => event,
        Err(error) => {
            tracing::error!(run_id = run.id, error = %error, "run start transaction failed");
            let _ = send_json(outbound, &ErrorEvent::new("storage_error", true)).await;
            return RunOutcome::Continue;
        }
    };
    let client_gone_at_start = send_json(outbound, &started).await.is_err();

    let Some(provider) = state.provider.clone() else {
        return fail_run(storage, outbound, &run, "provider_unavailable").await;
    };
    let Some(identity) = state.identity.clone() else {
        return fail_run(storage, outbound, &run, "identity_unavailable").await;
    };
    let (recent_result, self_state_result, relationship_result) = tokio::join!(
        storage.recent_messages(&run.session_id, 80),
        storage.latest_self_state(),
        storage.latest_relationship_state(),
    );
    let recent = match recent_result {
        Ok(messages) => messages,
        Err(error) => {
            tracing::error!(run_id = run.id, error = %error, "recent message loading failed");
            return fail_run(storage, outbound, &run, "storage_error").await;
        }
    };
    let self_state = match self_state_result {
        Ok(state) => state,
        Err(error) => {
            tracing::warn!(
                run_id = run.id,
                source = "self_state",
                error = %error,
                "optional context source degraded"
            );
            None
        }
    };
    let relationship = match relationship_result {
        Ok(state) => state.unwrap_or_default(),
        Err(error) => {
            tracing::warn!(
                run_id = run.id,
                source = "relationship_state",
                error = %error,
                "optional context source degraded"
            );
            RelationshipState::default()
        }
    };
    let Some(current_message) = recent
        .iter()
        .find(|message| message.id == run.trigger_message_id)
        .cloned()
    else {
        tracing::error!(
            run_id = run.id,
            "trigger message missing from recent context"
        );
        return fail_run(storage, outbound, &run, "storage_error").await;
    };
    let route = state.router.route(route_kind);
    let capabilities = provider.capabilities();
    let budget = ContextBudget::for_model(
        capabilities.context_window,
        state.config.provider.max_output_tokens,
    );
    let mode = match route_kind {
        RouteKind::Chat => "chat",
        RouteKind::ComplexTask => "task",
        RouteKind::Maintenance => "maintenance",
    };
    let compiled = match ContextCompiler::new(identity).compile(CompileInput {
        run_id: run.id.clone(),
        mode: mode.to_string(),
        current_message,
        recent_messages: recent,
        self_state,
        relationship_state: relationship,
        memory_evidence: Vec::new(),
        now: Utc::now(),
        budget,
    }) {
        Ok(compiled) => compiled,
        Err(error) => {
            tracing::error!(run_id = run.id, error = %error, "context compilation failed");
            return fail_run(storage, outbound, &run, "context_compile_failed").await;
        }
    };
    let context_event = match storage.save_context_plan(&compiled.plan).await {
        Ok(event) => event,
        Err(error) => {
            tracing::error!(run_id = run.id, error = %error, "context plan persistence failed");
            return fail_run(storage, outbound, &run, "storage_error").await;
        }
    };
    let context_plan_id = compiled.plan.id.clone();
    let prompt_hash = compiled.plan.prompt_hash.clone();
    if send_json(outbound, &context_event).await.is_err() {
        // The provider call still completes so committed runs do not depend on a live client.
    }
    let request = ModelRequest {
        request_id: run.id.clone(),
        model: route.model,
        messages: compiled.messages,
        tools: Vec::new(),
        tool_choice: ToolChoice::None,
        temperature: None,
        top_p: None,
        max_output_tokens: Some(state.config.provider.max_output_tokens),
        response_format: route.response_format,
        thinking_mode: route.thinking_mode,
        timeout: state.config.provider.request_timeout,
        metadata: BTreeMap::from([
            ("route".to_string(), format!("{route_kind:?}")),
            ("context_plan_id".to_string(), context_plan_id),
            ("prompt_hash".to_string(), prompt_hash),
        ]),
    };
    let cancellation = state.shutdown.child_token();
    let stream_result = provider.stream(request, cancellation.clone()).await;
    let mut events = match stream_result {
        Ok(events) => events,
        Err(error) => {
            tracing::warn!(
                run_id = run.id,
                code = error.code(),
                "provider stream failed"
            );
            return fail_run(storage, outbound, &run, error.code()).await;
        }
    };

    let mut raw_reply = String::new();
    let mut visible_filter = VisibleTextFilter::default();
    let mut usage: Option<TokenUsage> = None;
    let mut reason = FinishReason::Unknown;
    let mut client_gone = client_gone_at_start;

    loop {
        let event = if client_gone {
            events.next().await
        } else {
            tokio::select! {
                () = state.shutdown.cancelled() => {
                    cancellation.cancel();
                    return fail_run(storage, outbound, &run, "shutdown").await;
                }
                event = events.next() => event,
                frame = incoming.next() => {
                    match handle_run_control(frame, outbound, &run, &cancellation).await {
                        ControlOutcome::Continue => continue,
                        ControlOutcome::Cancelled => {
                            let event = storage.cancel_run(&run.id).await;
                            if let Ok(event) = event {
                                let _ = send_json(outbound, &event).await;
                            }
                            return RunOutcome::Continue;
                        }
                        ControlOutcome::ConnectionClosed => {
                            client_gone = true;
                            continue;
                        }
                    }
                }
            }
        };
        let Some(event) = event else {
            break;
        };
        match event {
            Ok(ModelEvent::ResponseStarted { model, .. }) => {
                if !client_gone {
                    let event = transient_event(
                        storage.scope(),
                        &run.session_id,
                        &run,
                        "model.started",
                        json!({ "model": model }),
                    );
                    if send_json(outbound, &event).await.is_err() {
                        client_gone = true;
                    }
                }
            }
            Ok(ModelEvent::TextDelta { text }) => {
                raw_reply.push_str(&text);
                let visible = visible_filter.push(&text);
                if !visible.is_empty() && !client_gone {
                    let event = transient_event(
                        storage.scope(),
                        &run.session_id,
                        &run,
                        "text.delta",
                        json!({ "text": visible }),
                    );
                    if send_json(outbound, &event).await.is_err() {
                        client_gone = true;
                    }
                }
            }
            Ok(ModelEvent::Usage { usage: value }) => usage = Some(value),
            Ok(ModelEvent::Finish { reason: value }) => reason = value,
            Ok(ModelEvent::ReasoningDelta { .. })
            | Ok(ModelEvent::ToolCallStarted { .. })
            | Ok(ModelEvent::ToolCallArgumentsDelta { .. })
            | Ok(ModelEvent::ToolCallCompleted { .. })
            | Ok(ModelEvent::ProviderWarning { .. }) => {}
            Err(error) => {
                tracing::warn!(
                    run_id = run.id,
                    code = error.code(),
                    "provider event failed"
                );
                return fail_run(storage, outbound, &run, error.code()).await;
            }
        }
    }
    let tail = visible_filter.finish();
    if !tail.is_empty() && !client_gone {
        let event = transient_event(
            storage.scope(),
            &run.session_id,
            &run,
            "text.delta",
            json!({ "text": tail }),
        );
        let _ = send_json(outbound, &event).await;
    }
    let visible_reply = memory::strip_all_tags(&raw_reply);
    let finish_reason = finish_reason_name(reason);
    let completion_usage = CompletionUsage {
        input_tokens: usage
            .as_ref()
            .and_then(|value| i64::try_from(value.input_tokens).ok()),
        output_tokens: usage
            .as_ref()
            .and_then(|value| i64::try_from(value.output_tokens).ok()),
        cached_input_tokens: usage
            .as_ref()
            .and_then(|value| i64::try_from(value.cached_input_tokens).ok()),
    };
    match storage
        .complete_reply(&run.id, &visible_reply, finish_reason, completion_usage)
        .await
    {
        Ok(persisted) => {
            if !client_gone {
                for event in persisted {
                    let _ = send_json(outbound, &event).await;
                }
                RunOutcome::Continue
            } else {
                RunOutcome::ConnectionClosed
            }
        }
        Err(error) => {
            tracing::error!(run_id = run.id, error = %error, "reply completion transaction failed");
            let _ = send_json(outbound, &ErrorEvent::new("storage_error", true)).await;
            if client_gone {
                RunOutcome::ConnectionClosed
            } else {
                RunOutcome::Continue
            }
        }
    }
}

enum ControlOutcome {
    Continue,
    Cancelled,
    ConnectionClosed,
}

async fn handle_run_control(
    frame: Option<Result<Message, axum::Error>>,
    outbound: &mpsc::Sender<Outbound>,
    run: &RunRecord,
    cancellation: &CancellationToken,
) -> ControlOutcome {
    match frame {
        Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientCommand>(&text) {
            Ok(ClientCommand::RunCancel { run_id }) if run_id == run.id => {
                cancellation.cancel();
                ControlOutcome::Cancelled
            }
            Ok(ClientCommand::Ping { nonce }) => {
                let _ = send_json(outbound, &PongEvent::new(nonce.as_deref())).await;
                ControlOutcome::Continue
            }
            _ => {
                let _ = send_json(outbound, &ErrorEvent::new("run_in_progress", true)).await;
                ControlOutcome::Continue
            }
        },
        Some(Ok(Message::Ping(payload))) => {
            let _ = send_frame(outbound, Message::Pong(payload)).await;
            ControlOutcome::Continue
        }
        Some(Ok(Message::Pong(_))) => ControlOutcome::Continue,
        Some(Ok(Message::Close(_))) => {
            let _ = send_outbound(outbound, Outbound::FlushClose).await;
            ControlOutcome::ConnectionClosed
        }
        None | Some(Err(_)) => ControlOutcome::ConnectionClosed,
        Some(Ok(Message::Binary(_))) => {
            let _ = send_json(outbound, &ErrorEvent::new("binary_not_supported", true)).await;
            ControlOutcome::Continue
        }
    }
}

async fn fail_run(
    storage: &Storage,
    outbound: &mpsc::Sender<Outbound>,
    run: &RunRecord,
    code: &str,
) -> RunOutcome {
    match storage.fail_run(&run.id, code).await {
        Ok(event) => {
            let _ = send_json(outbound, &event).await;
        }
        Err(error) => {
            tracing::error!(run_id = run.id, error = %error, "run failure transaction failed")
        }
    }
    RunOutcome::Continue
}

async fn subscribe(
    storage: &Storage,
    outbound: &mpsc::Sender<Outbound>,
    session_id: &str,
    after_seq: i64,
) -> Result<(), StorageError> {
    let replay = storage
        .events_after(session_id, after_seq, MAX_REPLAY_EVENTS + 1)
        .await?;
    if after_seq <= 0 || replay.len() as i64 > MAX_REPLAY_EVENTS {
        let snapshot = storage.snapshot(session_id).await?;
        let _ = send_json(outbound, &SnapshotEvent::new(&snapshot)).await;
    } else {
        for event in replay {
            if send_json(outbound, &event).await.is_err() {
                break;
            }
        }
    }
    Ok(())
}

async fn writer_loop(
    mut sink: futures_util::stream::SplitSink<WebSocket, Message>,
    mut receiver: mpsc::Receiver<Outbound>,
    shutdown: CancellationToken,
) {
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.tick().await;
    loop {
        tokio::select! {
            () = shutdown.cancelled() => {
                let _ = sink.send(Message::Close(None)).await;
                break;
            }
            _ = heartbeat.tick() => {
                if sink.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }
            command = receiver.recv() => match command {
                Some(Outbound::Frame(frame)) => {
                    if sink.send(frame).await.is_err() {
                        break;
                    }
                }
                Some(Outbound::FlushClose) => {
                    let _ = sink.flush().await;
                    break;
                }
                None => {
                    let _ = sink.send(Message::Close(None)).await;
                    break;
                }
            }
        }
    }
}

async fn send_json<T: Serialize>(outbound: &mpsc::Sender<Outbound>, value: &T) -> Result<(), ()> {
    let text = serde_json::to_string(value).map_err(|_| ())?;
    send_frame(outbound, Message::Text(text.into())).await
}

async fn send_frame(outbound: &mpsc::Sender<Outbound>, frame: Message) -> Result<(), ()> {
    send_outbound(outbound, Outbound::Frame(frame)).await
}

async fn send_close(
    outbound: &mpsc::Sender<Outbound>,
    code: u16,
    reason: &'static str,
) -> Result<(), ()> {
    send_frame(
        outbound,
        Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })),
    )
    .await
}

async fn send_outbound(outbound: &mpsc::Sender<Outbound>, command: Outbound) -> Result<(), ()> {
    timeout(OUTBOUND_WAIT, outbound.send(command))
        .await
        .map_err(|_| ())?
        .map_err(|_| ())
}

const fn finish_reason_name(reason: FinishReason) -> &'static str {
    match reason {
        FinishReason::Stop => "stop",
        FinishReason::Length => "length",
        FinishReason::ContentFilter => "content_filter",
        FinishReason::ToolCalls => "tool_calls",
        FinishReason::InsufficientResource => "insufficient_resource",
        FinishReason::Cancelled => "cancelled",
        FinishReason::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_reasons_have_stable_wire_names() {
        assert_eq!(finish_reason_name(FinishReason::Stop), "stop");
        assert_eq!(finish_reason_name(FinishReason::ToolCalls), "tool_calls");
    }

    #[test]
    fn protocol_requires_an_explicit_common_version() {
        let valid = r#"{"type":"client.hello","versions":[1]}"#;
        let invalid = r#"{"type":"client.hello","versions":[2]}"#;
        assert!(matches!(
            serde_json::from_str::<ClientCommand>(valid),
            Ok(ClientCommand::Hello { versions }) if versions.contains(&PROTOCOL_VERSION)
        ));
        assert!(matches!(
            serde_json::from_str::<ClientCommand>(invalid),
            Ok(ClientCommand::Hello { versions }) if !versions.contains(&PROTOCOL_VERSION)
        ));
    }
}
