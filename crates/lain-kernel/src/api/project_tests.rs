use super::*;
use axum::body::{Body, to_bytes};
use axum::http::Request as HttpRequest;
use lain_wire::{DeviceId, DeviceWorkspaceId, ProjectId, RequestId, devices::*, projects::*};
use serde_json::{Value, json};
use tower::ServiceExt;

const OWNER: &str = "fixture-project-owner-12345678901234567890";
struct NeverInfer;
impl crate::actor::model::ActorModel for NeverInfer {
    fn respond(
        &self,
        _: crate::actor::model::Request,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = anyhow::Result<crate::actor::model::Reply>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async { anyhow::bail!("duplicate receipt must not infer") })
    }
}
async fn call(
    app: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = HttpRequest::builder()
        .method(method)
        .uri(path)
        .header("Content-Type", "application/json");
    if let Some(t) = token {
        request = request.header("Authorization", format!("Bearer {t}"));
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(body.map_or(Body::empty(), |v| {
                    Body::from(serde_json::to_vec(&v).unwrap())
                }))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn projects_owner_api_admits_explicit_assignment_and_denies_device_administration() {
    let dir = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
    kernel.mark_ready().unwrap();
    let app = super::router(kernel.clone(), OWNER.parse().unwrap());
    let input = CreateProject {
        id: ProjectId::new(),
        request_id: RequestId::new(),
        name: "isolated API fixture".into(),
    };
    assert_eq!(
        call(&app, "POST", "/v1/projects", None, Some(json!(input)))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, body) = call(
        &app,
        "POST",
        "/v1/projects",
        Some(OWNER),
        Some(json!(input)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let project: Project = serde_json::from_value(body).unwrap();
    assert_eq!(project.id, input.id);
    assert_eq!(project.revision, 1);
    assert_eq!(
        call(&app, "GET", "/v1/projects", Some(OWNER), None).await.1,
        json!([project])
    );
    let device_token = format!("lain-device-{}", "ab".repeat(32));
    let r = RegisterDevice {
        id: DeviceId::new(),
        request_id: RequestId::new(),
        label: "synthetic PC".into(),
        platform: DevicePlatform::Windows,
        credential_fingerprint: device_credential_fingerprint(&device_token).unwrap(),
        workspaces: vec![DeviceWorkspace {
            id: DeviceWorkspaceId::new(),
            path: "C:\\synthetic\\not-on-kernel-host".into(),
        }],
        capabilities: vec![],
        expires_at_ms: crate::domain::now_ms().unwrap() + 600000,
    };
    kernel.device_register(r.clone()).unwrap();
    kernel
        .device_claim(
            r.id,
            &r.credential_fingerprint,
            ClaimDevice {
                request_id: RequestId::new(),
                registration_fingerprint: registration_fingerprint(&r),
            },
        )
        .unwrap();
    let conversation = kernel.create_conversation().unwrap();
    let context_path = format!("/v1/conversations/{}/memory-context", conversation.id);
    let context = lain_wire::memory_context::SetMemoryContext {
        request_id: RequestId::new(),
        expected_revision: 0,
        selection: Some(lain_wire::memory_context::MemoryContextSelection {
            device_id: r.id,
            workspace_id: r.workspaces[0].id,
            device_revision: 2,
            project: None,
        }),
    };
    assert_eq!(
        call(
            &app,
            "POST",
            &context_path,
            Some(&device_token),
            Some(json!(context))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, "GET", &context_path, Some(&device_token), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let chosen = call(
        &app,
        "POST",
        &context_path,
        Some(OWNER),
        Some(json!(context)),
    )
    .await;
    assert_eq!(chosen.0, StatusCode::OK);
    assert_eq!(chosen.1["revision"], 1);
    assert_eq!(
        call(&app, "GET", &context_path, Some(OWNER), None).await.1,
        chosen.1
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &context_path,
            Some(OWNER),
            Some(json!(context))
        )
        .await
        .1,
        chosen.1
    );
    let mut changed = context.clone();
    changed.selection = None;
    assert_eq!(
        call(
            &app,
            "POST",
            &context_path,
            Some(OWNER),
            Some(json!(changed))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mapping = SetProjectWorkspace {
        request_id: RequestId::new(),
        device_id: r.id,
        workspace_id: r.workspaces[0].id,
        expected_device_revision: 2,
        project_id: Some(project.id),
        expected_project_revision: Some(1),
        expected_revision: 0,
    };
    let show = format!("/v1/projects/{}", project.id);
    let assigned = format!(
        "/v1/devices/{}/workspaces/{}/project",
        r.id, r.workspaces[0].id
    );
    for path in ["/v1/projects", show.as_str(), assigned.as_str()] {
        assert_eq!(
            call(&app, "GET", path, Some(&device_token), None).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    for (path, body) in [
        ("/v1/projects", json!(input)),
        ("/v1/project-workspaces", json!(mapping)),
    ] {
        assert_eq!(
            call(&app, "POST", path, Some(&device_token), Some(body))
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        call(&app, "GET", &assigned, Some(OWNER), None).await.1,
        Value::Null
    );
    let (status, receipt) = call(
        &app,
        "POST",
        "/v1/project-workspaces",
        Some(OWNER),
        Some(json!(mapping)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        call(&app, "GET", &assigned, Some(OWNER), None).await.1,
        receipt
    );
    let change = ChangeProject {
        request_id: RequestId::new(),
        expected_revision: 1,
        name: project.name.clone(),
        state: ProjectState::Archived,
    };
    let change_path = format!("{show}/change");
    assert_eq!(
        call(
            &app,
            "POST",
            &change_path,
            Some(&device_token),
            Some(json!(change))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, "POST", &change_path, Some(OWNER), Some(json!(change)))
            .await
            .0,
        StatusCode::OK
    );
    let mut stale = mapping.clone();
    stale.request_id = RequestId::new();
    stale.expected_revision = 1;
    assert_eq!(
        call(
            &app,
            "POST",
            "/v1/project-workspaces",
            Some(OWNER),
            Some(json!(stale))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert!(
        serde_json::from_value::<crate::actor::tools::Tool>(
            json!({"name":"project.create","arguments":input})
        )
        .is_err()
    );
}
#[tokio::test]
async fn bridge_api_recovery_and_duplicate_receipts_do_not_redisclose_withdrawn_bodies() {
    use crate::{
        actor::domain::{MessageRole, TurnState},
        memory::*,
    };
    let dir = tempfile::tempdir().unwrap();
    let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
    k.mark_ready().unwrap();
    let r = k
        .add_memory(AddMemory {
            request_id: RequestId::new(),
            draft: MemoryDraft {
                content: "source fact".into(),
                kind: MemoryKind::Episodic,
                scope: MemoryScope::Global,
            },
        })
        .unwrap();
    let conversation = k.create_conversation().unwrap();
    let request = uuid::Uuid::new_v4();
    let (t, _) = k
        .actor_submit_scoped(conversation.id, request, "read the source".into(), None)
        .unwrap();
    k.actor_memory_inspect(t.id, r.id, MemoryScope::Global)
        .unwrap();
    k.actor_update(t.id, |t| {
        t.message(MessageRole::Assistant, "withdrawn-network-marker".into())
    })
    .unwrap();
    k.delete_memory(r.id, r.revision).unwrap();
    k.actor_update(t.id, |t| {
        t.state = TurnState::Unknown;
        Ok(())
    })
    .unwrap();
    assert!(
        k.actor_turn(t.id)
            .unwrap()
            .messages
            .iter()
            .any(|m| m.content == "withdrawn-network-marker")
    );
    let a = crate::actor::Actor::with_model(
        k.clone(),
        None,
        OWNER.parse().unwrap(),
        Arc::new(NeverInfer),
    );
    let app = super::actor_router(k, OWNER.parse().unwrap(), None, a);
    for (method, path, body) in [
        ("GET", format!("/v1/actor-turns/{}", t.id), None),
        (
            "GET",
            format!("/v1/conversations/{}?recent=true", conversation.id),
            None,
        ),
        (
            "POST",
            format!("/v1/conversations/{}/turns", conversation.id),
            Some(json!({"request_id":request,"text":"read the source"})),
        ),
    ] {
        let (status, view) = call(&app, method, &path, Some(OWNER), body).await;
        assert!(status.is_success(), "{status}: {view}");
        assert!(
            !view.to_string().contains("withdrawn-network-marker"),
            "{view}"
        );
    }
    let request = HttpRequest::builder()
        .uri(format!("/v1/actor-turns/{}/events", t.id))
        .header("Authorization", format!("Bearer {OWNER}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 65536).await.unwrap();
    assert!(!String::from_utf8_lossy(&body).contains("withdrawn-network-marker"));
}

#[tokio::test]
async fn bridge_queued_sse_revalidates_at_consumption_and_withholds_pending_actions() {
    use crate::{actor::domain::*, actor::tools::Tool, memory::*};
    use futures_util::StreamExt;
    for pending in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
        k.mark_ready().unwrap();
        let record = k
            .add_memory(AddMemory {
                request_id: RequestId::new(),
                draft: MemoryDraft {
                    content: "current source".into(),
                    kind: MemoryKind::Episodic,
                    scope: MemoryScope::Global,
                },
            })
            .unwrap();
        let c = k.create_conversation().unwrap();
        let (t, _) = k
            .actor_submit_scoped(c.id, uuid::Uuid::new_v4(), "read source".into(), None)
            .unwrap();
        k.actor_memory_inspect(t.id, record.id, MemoryScope::Global)
            .unwrap();
        k.actor_update(t.id, move |t| {
            t.message(MessageRole::Assistant, "queued-private-marker".into())?;
            if pending {
                t.state = TurnState::PendingConfirmation;
                t.actions.push(Action {
                    id: ActionId::new(),
                    call_id: "proposal".into(),
                    tool: Tool::WorkCancel(crate::actor::tools::SessionArg {
                        session_id: crate::domain::WorkSessionId::new(),
                    }),
                    state: ActionState::Proposed,
                    result: None,
                });
            } else {
                t.state = TurnState::Completed;
            }
            Ok(())
        })
        .unwrap();
        let a = crate::actor::Actor::with_model(
            k.clone(),
            None,
            OWNER.parse().unwrap(),
            Arc::new(NeverInfer),
        );
        let app = super::actor_router(k.clone(), OWNER.parse().unwrap(), None, a.clone());
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri(format!("/v1/actor-turns/{}/events", t.id))
                    .header("Authorization", format!("Bearer {OWNER}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // The response is constructed while current; consumption starts after withdrawal.
        k.delete_memory(record.id, record.revision).unwrap();
        let bytes = response
            .into_body()
            .into_data_stream()
            .next()
            .await
            .unwrap()
            .unwrap();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(!text.contains("queued-private-marker"), "{text}");
        assert!(text.contains("source_unavailable"), "{text}");
        let view = k.actor_view(t.id).unwrap();
        view.validate().unwrap();
        assert!(view.actions.is_empty());
        if pending {
            assert_eq!(view.state, lain_wire::ActorTurnState::PendingConfirmation);
            a.cancel(t.id).await.unwrap();
            assert_eq!(
                k.actor_view(t.id).unwrap().state,
                lain_wire::ActorTurnState::Cancelled
            );
        }
    }
}
