use super::*;
use super::{
    archives::safe,
    devices::{agent_error, proof},
};
use axum::http::HeaderMap;
use lain_wire::device_execution::*;
use lain_wire::{DeviceCommandId, DeviceId, RequestId, device_commands::*};

pub(super) fn owner_routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/devices/{id}/commands", post(queue).get(summaries))
        .route("/v1/devices/{id}/commands/changes", get(owner_changes))
        .route("/v1/devices/{id}/commands/{cid}", get(owner_get))
        .route("/v1/devices/{id}/commands/{cid}/cancel", post(cancel))
        .route(
            "/v1/devices/{id}/commands/{cid}/approvals",
            get(owner_decisions),
        )
        .route(
            "/v1/devices/{id}/commands/{cid}/approvals/{aid}",
            post(decide),
        )
        .route(
            "/v1/devices/{id}/commands/{cid}/receipts/{rid}",
            get(owner_receipt),
        )
}
async fn summaries(
    State(s): State<ApiState>,
    Path(id): Path<DeviceId>,
) -> Result<Json<Vec<DeviceCommandSummary>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_command_summaries(id))
            .await
            .map_err(control_error)?,
    )
}
pub(super) fn agent_routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/device-agent/{id}/commands/changes", get(agent_changes))
        .route("/v1/device-agent/{id}/commands/{cid}", get(agent_get))
        .route("/v1/device-agent/{id}/commands/{cid}/events", post(event))
        .route(
            "/v1/device-agent/{id}/commands/{cid}/approvals",
            get(agent_decisions),
        )
        .route(
            "/v1/device-agent/{id}/commands/{cid}/receipts/{rid}",
            get(agent_receipt),
        )
}
async fn decide(
    State(s): State<ApiState>,
    Path((id, cid, aid)): Path<(DeviceId, DeviceCommandId, lain_wire::WorkerApprovalId)>,
    Json(input): Json<DecideDeviceApproval>,
) -> Result<Json<DeviceApprovalDecision>, ApiError> {
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_approval_decide(id, cid, aid, input)
        })
        .await
        .map_err(control_error)?,
    )
}
async fn owner_decisions(
    State(s): State<ApiState>,
    Path((id, cid)): Path<(DeviceId, DeviceCommandId)>,
) -> Result<Json<Vec<DeviceApprovalDecision>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_approval_decisions(id, cid, None)
        })
        .await
        .map_err(control_error)?,
    )
}
async fn agent_decisions(
    State(s): State<ApiState>,
    Path((id, cid)): Path<(DeviceId, DeviceCommandId)>,
    headers: HeaderMap,
) -> Result<Json<Vec<DeviceApprovalDecision>>, ApiError> {
    let p = proof(&headers)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_approval_decisions(id, cid, Some(&p))
        })
        .await
        .map_err(agent_error)?,
    )
}
async fn queue(
    State(s): State<ApiState>,
    Path(id): Path<DeviceId>,
    Json(input): Json<QueueDeviceWork>,
) -> Result<Json<DeviceCommand>, ApiError> {
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_command_queue(id, input))
            .await
            .map_err(control_error)?,
    )
}
async fn cancel(
    State(s): State<ApiState>,
    Path((id, cid)): Path<(DeviceId, DeviceCommandId)>,
    Json(input): Json<CancelDeviceCommand>,
) -> Result<Json<DeviceCommand>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_command_cancel(id, cid, input))
            .await
            .map_err(control_error)?,
    )
}
async fn owner_get(
    State(s): State<ApiState>,
    Path((id, cid)): Path<(DeviceId, DeviceCommandId)>,
) -> Result<Json<DeviceCommand>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_command_get(id, cid, None))
            .await
            .map_err(control_error)?,
    )
}
async fn agent_get(
    State(s): State<ApiState>,
    Path((id, cid)): Path<(DeviceId, DeviceCommandId)>,
    headers: HeaderMap,
) -> Result<Json<DeviceCommand>, ApiError> {
    let p = proof(&headers)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_command_get(id, cid, Some(&p)))
            .await
            .map_err(agent_error)?,
    )
}
async fn event(
    State(s): State<ApiState>,
    Path((id, cid)): Path<(DeviceId, DeviceCommandId)>,
    headers: HeaderMap,
    Json(input): Json<ReportCommandEvent>,
) -> Result<Json<CommandEventReceipt>, ApiError> {
    let p = proof(&headers)?;
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_command_event(id, cid, &p, input)
        })
        .await
        .map_err(agent_error)?,
    )
}
async fn owner_receipt(
    State(s): State<ApiState>,
    Path((id, cid, rid)): Path<(DeviceId, DeviceCommandId, RequestId)>,
) -> Result<Json<CommandEventReceipt>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_command_receipt(id, cid, rid, None)
        })
        .await
        .map_err(control_error)?,
    )
}
async fn agent_receipt(
    State(s): State<ApiState>,
    Path((id, cid, rid)): Path<(DeviceId, DeviceCommandId, RequestId)>,
    headers: HeaderMap,
) -> Result<Json<CommandEventReceipt>, ApiError> {
    let p = proof(&headers)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_command_receipt(id, cid, rid, Some(&p))
        })
        .await
        .map_err(agent_error)?,
    )
}
async fn changes(
    s: ApiState,
    id: DeviceId,
    p: Option<String>,
    input: CommandChangesQuery,
) -> Result<Json<CommandChanges>, ApiError> {
    // Subscribe before the first durable read: no check/subscribe lost wake-up.
    let mut events = s.kernel.device_commands_subscribe();
    let mut shutdown = s.kernel.subscribe_shutdown();
    let proof = p.clone();
    let mut page = blocking(&s.kernel, move |k| {
        k.device_command_changes(id, proof.as_deref(), input)
    })
    .await
    .map_err(agent_error)?;
    if input.wait_ms > 0 && input.through.is_none() && page.changes.is_empty() {
        tokio::select! {
            _=events.recv()=>{},
            _=shutdown.changed()=>{},
            _=tokio::time::sleep(std::time::Duration::from_millis(input.wait_ms.into()))=>{},
        }
        // Including lag/revoke/shutdown: always reconstruct and re-authenticate.
        page = blocking(&s.kernel, move |k| {
            k.device_command_changes(id, p.as_deref(), input)
        })
        .await
        .map_err(agent_error)?;
    }
    safe(&s, page)
}
async fn owner_changes(
    State(s): State<ApiState>,
    Path(id): Path<DeviceId>,
    Query(input): Query<CommandChangesQuery>,
) -> Result<Json<CommandChanges>, ApiError> {
    changes(s, id, None, input).await
}
async fn agent_changes(
    State(s): State<ApiState>,
    Path(id): Path<DeviceId>,
    headers: HeaderMap,
    Query(input): Query<CommandChangesQuery>,
) -> Result<Json<CommandChanges>, ApiError> {
    changes(s, id, Some(proof(&headers)?), input).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request as HttpRequest,
    };
    use lain_wire::{DeviceWorkspaceId, devices::*};
    use serde_json::{Value, json};
    use tower::ServiceExt;
    const OWNER: &str = "fixture-owner-api-credential-123456";
    fn fixture() -> (
        tempfile::TempDir,
        Arc<Kernel>,
        Router,
        RegisterDevice,
        String,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
        k.mark_ready().unwrap();
        let token = format!("lain-device-{}", "1a".repeat(32));
        let r = RegisterDevice {
            id: DeviceId::new(),
            request_id: RequestId::new(),
            label: "commands-fixture".into(),
            platform: DevicePlatform::Linux,
            credential_fingerprint: device_credential_fingerprint(&token).unwrap(),
            workspaces: vec![DeviceWorkspace {
                id: DeviceWorkspaceId::new(),
                path: "/device/workspace".into(),
            }],
            capabilities: vec![],
            expires_at_ms: crate::domain::now_ms().unwrap() + 600000,
        };
        k.device_register(r.clone()).unwrap();
        k.device_claim(
            r.id,
            &r.credential_fingerprint,
            ClaimDevice {
                request_id: RequestId::new(),
                registration_fingerprint: registration_fingerprint(&r),
            },
        )
        .unwrap();
        let app = super::super::router(k.clone(), OWNER.parse().unwrap());
        (dir, k, app, r, token)
    }
    fn input(r: &RegisterDevice) -> QueueDeviceWork {
        QueueDeviceWork {
            request_id: RequestId::new(),
            expected_device_revision: 2,
            workspace_id: r.workspaces[0].id,
            profile_id: "codex/local".into(),
            sandbox: lain_wire::Sandbox::ReadOnly,
            task: "read fixture workspace".into(),
            expires_at_ms: crate::domain::now_ms().unwrap() + 60000,
        }
    }
    fn request(method: &str, path: &str, key: &str, body: Option<Value>) -> HttpRequest<Body> {
        HttpRequest::builder()
            .method(method)
            .uri(path)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {key}"))
            .body(body.map_or(Body::empty(), |v| {
                Body::from(serde_json::to_vec(&v).unwrap())
            }))
            .unwrap()
    }
    async fn call(app: &Router, r: HttpRequest<Body>) -> (StatusCode, Value) {
        let response = app.clone().oneshot(r).await.unwrap();
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    #[tokio::test]
    async fn direct_work_receipt_is_owner_only_and_does_not_leak_other_conversations() {
        let (_dir, k, app, r, key) = fixture();
        let conversation = k.create_conversation().unwrap().id;
        let other = k.create_conversation().unwrap().id;
        let scope = input(&r);
        let path = format!(
            "/v1/conversations/{conversation}/device-work/{}/requests/{}",
            r.id, scope.request_id
        );
        let (status, empty) = call(&app, request("GET", &path, OWNER, None)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(empty.is_null());
        let command = k.device_work_dispatch(conversation, r.id, scope).unwrap();
        assert_eq!(
            call(&app, request("GET", &path, &key, None)).await.0,
            StatusCode::UNAUTHORIZED
        );
        let (status, record) = call(&app, request("GET", &path, OWNER, None)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(record["command"]["id"], json!(command.id));
        let other_path = path.replace(&conversation.to_string(), &other.to_string());
        assert!(
            call(&app, request("GET", &other_path, OWNER, None))
                .await
                .1
                .is_null()
        );
        assert_eq!(k.device_command_summaries(r.id).unwrap().len(), 1);
    }
    #[tokio::test]
    async fn conversation_dispatch_is_owner_only_and_results_recover_in_the_same_binding() {
        use futures_util::StreamExt;
        use lain_wire::device_work::DeviceWorkPage;
        let (_dir, k, app, r, key) = fixture();
        let conversation = k.create_conversation().unwrap().id;
        let other = k.create_conversation().unwrap().id;
        let endpoint = format!("/v1/conversations/{conversation}/device-work");
        let queue = format!("{endpoint}/{}", r.id);
        let scope = input(&r);
        assert_eq!(
            call(&app, request("POST", &queue, &key, Some(json!(scope))))
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
        let (status, created) =
            call(&app, request("POST", &queue, OWNER, Some(json!(scope)))).await;
        assert_eq!(status, StatusCode::OK);
        let command: DeviceCommand = serde_json::from_value(created).unwrap();
        let (_, page) = call(&app, request("GET", &endpoint, OWNER, None)).await;
        let page: DeviceWorkPage = serde_json::from_value(page).unwrap();
        page.validate(conversation).unwrap();
        assert_eq!(page.work[0].command.id, command.id);
        let wrong = format!(
            "/v1/conversations/{other}/device-work/{}/{}",
            r.id, command.id
        );
        assert_eq!(
            call(&app, request("GET", &wrong, OWNER, None)).await.0,
            StatusCode::NOT_FOUND
        );
        let stream = app
            .clone()
            .oneshot(request("GET", &format!("{endpoint}/events"), OWNER, None))
            .await
            .unwrap();
        assert_eq!(stream.status(), StatusCode::OK);
        let mut frames = stream.into_body().into_data_stream();
        let first = tokio::time::timeout(Duration::from_secs(2), frames.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            String::from_utf8(first.to_vec())
                .unwrap()
                .contains("device_work_snapshot")
        );
        k.device_command_event(
            r.id,
            command.id,
            &r.credential_fingerprint,
            ReportCommandEvent {
                request_id: RequestId::new(),
                sequence: 1,
                kind: CommandEventKind::Accepted,
                detail: "accepted by fixture".into(),
                execution: None,
            },
        )
        .unwrap();
        let changed = tokio::time::timeout(Duration::from_secs(2), frames.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            String::from_utf8(changed.to_vec())
                .unwrap()
                .contains("accepted by fixture")
        );
        assert!(k.list_sessions().unwrap().is_empty());
    }

    #[tokio::test]
    async fn dashboard_summaries_are_owner_only_metadata_and_never_launch_work() {
        let (_dir, k, app, r, key) = fixture();
        let path = format!("/v1/devices/{}/commands", r.id);
        let task = input(&r);
        let exact_task = task.task.clone();
        assert_eq!(
            call(&app, request("GET", &path, OWNER, None)).await.1,
            json!([])
        );
        let c = k.device_command_queue(r.id, task).unwrap();
        assert_eq!(
            call(&app, request("GET", &path, &key, None)).await.0,
            StatusCode::UNAUTHORIZED
        );
        let (status, rows) = call(&app, request("GET", &path, OWNER, None)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["id"], c.id.to_string());
        assert_eq!(rows[0]["state"], "queued");
        assert!(rows[0].get("input").is_none() && rows[0].get("execution").is_none());
        assert!(!rows.to_string().contains(&exact_task));
        assert_eq!(k.device_command_get(r.id, c.id, None).unwrap(), c);
        let (status, sessions) = call(&app, request("GET", "/v1/work-sessions", OWNER, None)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(sessions, json!([]));
        assert_eq!(
            call(
                &app,
                request(
                    "GET",
                    &format!("/v1/devices/{}/commands", DeviceId::new()),
                    OWNER,
                    None
                )
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }
    #[tokio::test]
    async fn command_http_principals_secret_rejection_and_own_scope() {
        let (_dir, k, app, r, key) = fixture();
        let path = format!("/v1/devices/{}/commands", r.id);
        let task = input(&r);
        assert_eq!(
            call(&app, request("POST", &path, &key, Some(json!(task))))
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
        let mut secret = task.clone();
        secret.task = OWNER.into();
        assert_eq!(
            call(&app, request("POST", &path, OWNER, Some(json!(secret))))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
        let (status, c) = call(&app, request("POST", &path, OWNER, Some(json!(task)))).await;
        assert_eq!(status, StatusCode::OK);
        let cid: DeviceCommandId = c["id"].as_str().unwrap().parse().unwrap();
        let agent = format!("/v1/device-agent/{}/commands/{cid}", r.id);
        assert_eq!(
            call(&app, request("GET", &agent, OWNER, None)).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(&app, request("GET", &agent, &key, None)).await.0,
            StatusCode::OK
        );
        let other = format!("/v1/device-agent/{}/commands/{cid}", DeviceId::new());
        assert_eq!(
            call(&app, request("GET", &other, &key, None)).await.0,
            StatusCode::UNAUTHORIZED
        );
        let e = ReportCommandEvent {
            request_id: RequestId::new(),
            sequence: 1,
            kind: CommandEventKind::Accepted,
            detail: "fixture accepted".into(),
            execution: None,
        };
        assert_eq!(
            call(
                &app,
                request("POST", &format!("{agent}/events"), &key, Some(json!(e)))
            )
            .await
            .0,
            StatusCode::OK
        );
        k.device_revoke(
            r.id,
            RevokeDevice {
                expected_revision: 2,
            },
        )
        .unwrap();
        assert_eq!(
            call(
                &app,
                request("POST", &format!("{agent}/events"), &key, Some(json!(e)))
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            k.device_command_get(r.id, cid, None).unwrap().state,
            CommandState::Unknown
        );
    }
    #[tokio::test]
    async fn waiting_resync_wakes_on_publication_and_rechecks_revocation() {
        let (_dir, k, app, r, key) = fixture();
        let path = format!("/v1/device-agent/{}/commands/changes?wait_ms=25000", r.id);
        let waiting = call(&app, request("GET", &path, &key, None));
        tokio::pin!(waiting);
        assert!(futures_util::poll!(&mut waiting).is_pending());
        k.device_command_queue(r.id, input(&r)).unwrap();
        let (status, page) = tokio::time::timeout(std::time::Duration::from_secs(2), waiting)
            .await
            .unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["changes"].as_array().unwrap().len(), 1);
        let path = format!("{path}&after=1");
        let waiting = call(&app, request("GET", &path, &key, None));
        tokio::pin!(waiting);
        assert!(futures_util::poll!(&mut waiting).is_pending());
        k.device_revoke(
            r.id,
            RevokeDevice {
                expected_revision: 2,
            },
        )
        .unwrap();
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(2), waiting)
                .await
                .unwrap()
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    #[tokio::test]
    async fn only_owner_decides_and_only_bound_device_reads_decision_receipts() {
        let (_dir, k, app, r, key) = fixture();
        let c = k.device_command_queue(r.id, input(&r)).unwrap();
        let e = ReportCommandEvent {
            request_id: RequestId::new(),
            sequence: 1,
            kind: CommandEventKind::Accepted,
            detail: "fixture".into(),
            execution: None,
        };
        k.device_command_event(r.id, c.id, &r.credential_fingerprint, e)
            .unwrap();
        let mut evidence = ExecutionEvidence {
            execution_id: lain_wire::DeviceExecutionId::new(),
            session_id: Some(lain_wire::WorkSessionId::new()),
            turn_id: Some(lain_wire::WorkerTurnId::new()),
            phase: ExecutionPhase::WaitingForApproval,
            pending_approvals: vec![],
            approval_reports: vec![],
            result: None,
        };
        let mut a = DeviceApproval {
            id: lain_wire::WorkerApprovalId::new(),
            kind: "command_execution".into(),
            description: "Run selected fixture check".into(),
            truncated: false,
            choices: vec![
                DeviceApprovalChoice::ApproveOnce,
                DeviceApprovalChoice::Deny,
            ],
            fingerprint: String::new(),
        };
        a.fingerprint = approval_fingerprint(
            evidence.execution_id,
            evidence.session_id,
            evidence.turn_id,
            &a,
        );
        evidence.pending_approvals.push(a.clone());
        k.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            ReportCommandEvent {
                request_id: RequestId::new(),
                sequence: 2,
                kind: CommandEventKind::Started,
                detail: "fixture waiting".into(),
                execution: Some(evidence),
            },
        )
        .unwrap();
        let path = format!("/v1/devices/{}/commands/{}/approvals", r.id, c.id);
        let input = DecideDeviceApproval {
            request_id: RequestId::new(),
            fingerprint: a.fingerprint,
            choice: DeviceApprovalChoice::ApproveOnce,
        };
        let decision_path = format!("{path}/{}", a.id);
        assert_eq!(
            call(
                &app,
                request("POST", &decision_path, &key, Some(json!(input)))
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
        let agent = format!("/v1/device-agent/{}/commands/{}/approvals", r.id, c.id);
        assert_eq!(
            call(
                &app,
                request(
                    "POST",
                    &format!("{agent}/{}", a.id),
                    &key,
                    Some(json!(input))
                )
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
        let mut unknown = json!(input);
        unknown["provider_request"] = json!({"approve":true});
        assert_eq!(
            call(&app, request("POST", &decision_path, OWNER, Some(unknown)))
                .await
                .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        let cursor = k
            .device_command_changes(r.id, None, CommandChangesQuery::default())
            .unwrap()
            .next_after;
        let wait_path = format!(
            "/v1/device-agent/{}/commands/changes?after={cursor}&wait_ms=25000",
            r.id
        );
        let waiting = call(&app, request("GET", &wait_path, &key, None));
        tokio::pin!(waiting);
        assert!(futures_util::poll!(&mut waiting).is_pending());
        let (status, receipt) = call(
            &app,
            request("POST", &decision_path, OWNER, Some(json!(input))),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (wake, page) = tokio::time::timeout(std::time::Duration::from_secs(2), waiting)
            .await
            .unwrap();
        assert_eq!(wake, StatusCode::OK);
        assert_eq!(page["changes"].as_array().unwrap().len(), 1);
        assert!(page["changes"][0].get("decision").is_none());
        assert_eq!(
            call(
                &app,
                request("POST", &decision_path, OWNER, Some(json!(input)))
            )
            .await
            .1,
            receipt
        );
        assert_eq!(
            call(&app, request("GET", &agent, &key, None)).await.1,
            json!([receipt])
        );
        assert_eq!(
            call(&app, request("GET", &agent, OWNER, None)).await.0,
            StatusCode::UNAUTHORIZED
        );
        let before = k.device_command_get(r.id, c.id, None).unwrap();
        k.device_command_cancel(
            r.id,
            c.id,
            CancelDeviceCommand {
                expected_revision: before.revision,
            },
        )
        .unwrap();
        // Exact receipt recovery is allowed; it is not a new approval or application.
        assert_eq!(
            call(
                &app,
                request("POST", &decision_path, OWNER, Some(json!(input)))
            )
            .await
            .1,
            receipt
        );
        k.device_revoke(
            r.id,
            RevokeDevice {
                expected_revision: 2,
            },
        )
        .unwrap();
        assert_eq!(
            call(&app, request("GET", &agent, &key, None)).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(&app, request("GET", &path, OWNER, None)).await.1,
            json!([receipt])
        );
    }
}
