use super::*;
use crate::memory::*;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Mutex as StdMutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::Notify;

#[derive(Default)]
struct ScopeSchemaModel {
    definitions: StdMutex<Vec<Vec<Value>>>,
}
impl ActorModel for ScopeSchemaModel {
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<model::Reply>> + Send + '_>> {
        self.definitions.lock().unwrap().push(request.tools);
        Box::pin(async {
            Ok(model::Reply {
                text: "schema fixture complete".into(),
                calls: vec![],
                usage_tokens: 1,
                continuation: vec![],
            })
        })
    }
}

#[tokio::test]
async fn scoped_memory_model_tools_advertise_exact_selected_scope_ids() {
    use lain_wire::{
        DeviceId, DeviceWorkspaceId, ProjectId, RequestId, devices::*, memory_context::*,
        projects::*,
    };
    let dir = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
    kernel.mark_ready().unwrap();
    let proof = format!("lain-device-{}", "ab".repeat(32));
    let device = RegisterDevice {
        id: DeviceId::new(),
        request_id: RequestId::new(),
        label: "schema fixture PC".into(),
        platform: DevicePlatform::Windows,
        credential_fingerprint: device_credential_fingerprint(&proof).unwrap(),
        workspaces: vec![DeviceWorkspace {
            id: DeviceWorkspaceId::new(),
            path: "C:\\synthetic\\schema-fixture".into(),
        }],
        capabilities: vec![],
        expires_at_ms: crate::domain::now_ms().unwrap() + 600000,
    };
    kernel.device_register(device.clone()).unwrap();
    kernel
        .device_claim(
            device.id,
            &device.credential_fingerprint,
            ClaimDevice {
                request_id: RequestId::new(),
                registration_fingerprint: registration_fingerprint(&device),
            },
        )
        .unwrap();
    let project = kernel
        .project_create(CreateProject {
            id: ProjectId::new(),
            request_id: RequestId::new(),
            name: "schema fixture project".into(),
        })
        .unwrap();
    let mapping = kernel
        .set_project_workspace(SetProjectWorkspace {
            request_id: RequestId::new(),
            device_id: device.id,
            workspace_id: device.workspaces[0].id,
            expected_device_revision: 2,
            expected_revision: 0,
            project_id: Some(project.id),
            expected_project_revision: Some(project.revision),
        })
        .unwrap();
    let conversation = kernel.create_conversation().unwrap();
    kernel
        .set_memory_context(
            conversation.id,
            SetMemoryContext {
                request_id: RequestId::new(),
                expected_revision: 0,
                selection: Some(MemoryContextSelection {
                    device_id: device.id,
                    workspace_id: device.workspaces[0].id,
                    device_revision: 2,
                    project: Some(MemoryProjectContext {
                        project_id: project.id,
                        project_revision: project.revision,
                        mapping_revision: mapping.revision,
                    }),
                }),
            },
        )
        .unwrap();
    let fixture = Arc::new(ScopeSchemaModel::default());
    let actor = Actor::with_model(
        kernel.clone(),
        None,
        "synthetic-scope-owner-12345678901234567890"
            .parse()
            .unwrap(),
        fixture.clone(),
    );
    let t = actor
        .submit(
            conversation.id,
            Uuid::new_v4(),
            "inspect available memory".into(),
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        while kernel.actor_turn(t.id).unwrap().state == TurnState::Running {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(kernel.actor_turn(t.id).unwrap().state, TurnState::Completed);
    let definitions = fixture.definitions.lock().unwrap()[0].clone();
    assert!(!definitions.iter().any(|d| d["name"] == "memory_list"));
    for name in ["memory_search", "memory_inspect", "memory_propose"] {
        let definition = definitions.iter().find(|d| d["name"] == name).unwrap();
        let properties = &definition["parameters"]["properties"];
        let schema = if name == "memory_propose" {
            &properties["draft"]["properties"]["scope"]
        } else {
            &properties["scope"]
        };
        let variants = schema["anyOf"].as_array().unwrap();
        assert!(variants.iter().any(|v| v["properties"]["kind"]["enum"]
            == json!(["global", "internal"])
            && v["required"] == json!(["kind"])
            && v["additionalProperties"] == false));
        for scope in [
            MemoryScope::Conversation {
                conversation_id: conversation.id,
            },
            MemoryScope::Device {
                device_id: device.id,
            },
            MemoryScope::DeviceWorkspace {
                device_id: device.id,
                workspace_id: device.workspaces[0].id,
            },
            MemoryScope::Project {
                project_id: project.id,
            },
        ] {
            let value = json!(scope);
            let variant = variants
                .iter()
                .find(|v| v["properties"]["kind"]["enum"] == json!([value["kind"]]))
                .unwrap_or_else(|| panic!("{name} omits selected scope {}", value["kind"]));
            for (key, value) in value.as_object().unwrap() {
                assert_eq!(
                    variant["properties"][key]["enum"],
                    json!([value]),
                    "{name} must advertise exact selected IDs"
                );
                assert!(
                    variant["required"]
                        .as_array()
                        .unwrap()
                        .contains(&json!(key))
                );
            }
            assert_eq!(variant["additionalProperties"], false);
        }
        assert!(
            variants
                .iter()
                .all(|v| v["properties"]["kind"]["enum"] != json!(["workspace"])),
            "selected remote context must not offer daemon-local path scopes"
        );
    }

    let legacy = kernel.create_conversation().unwrap();
    let t = actor
        .submit(legacy.id, Uuid::new_v4(), "inspect legacy scopes".into())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        while kernel.actor_turn(t.id).unwrap().state == TurnState::Running {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(kernel.actor_turn(t.id).unwrap().state, TurnState::Completed);
    let definitions = fixture.definitions.lock().unwrap()[1].clone();
    assert!(!definitions.iter().any(|d| d["name"] == "memory_list"));
    for name in ["memory_search", "memory_inspect", "memory_propose"] {
        let definition = definitions.iter().find(|d| d["name"] == name).unwrap();
        let properties = &definition["parameters"]["properties"];
        let schema = if name == "memory_propose" {
            &properties["draft"]["properties"]["scope"]
        } else {
            &properties["scope"]
        };
        let variants = schema["anyOf"].as_array().unwrap();
        let local = variants
            .iter()
            .find(|v| v["properties"]["kind"]["enum"] == json!(["conversation"]))
            .unwrap();
        assert_eq!(
            local["properties"]["conversation_id"]["enum"],
            json!([legacy.id])
        );
        assert!(!schema.to_string().contains(&conversation.id.to_string()));
        assert!(
            variants
                .iter()
                .any(|v| v["properties"]["kind"]["enum"] == json!(["workspace"])
                    && v["properties"]["path"]["type"] == "string")
        );
        let encoded = schema.to_string();
        assert!(
            !encoded.contains(&device.id.to_string()) && !encoded.contains(&project.id.to_string()),
            "one conversation's selected IDs leaked into another tool schema"
        );
        assert!(variants.iter().all(|v| matches!(
            v["properties"]["kind"]["enum"][0].as_str(),
            Some("global" | "internal" | "workspace" | "conversation")
        )));
    }
}

struct HeldModel {
    record: MemoryId,
    calls: AtomicUsize,
    reached: Notify,
    release: Notify,
    requests: StdMutex<Vec<Value>>,
}
impl HeldModel {
    async fn reply(
        &self,
        request: Request,
        preview: Option<preview::PreviewSink>,
    ) -> Result<model::Reply> {
        self.requests.lock().unwrap().push(json!(request.input));
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            return Ok(model::Reply {
                text: String::new(),
                calls: vec![model::Call {
                    id: "synthetic-read".into(),
                    name: "memory_inspect".into(),
                    arguments: json!({"memory_id":self.record,"scope":{"kind":"global"}})
                        .to_string(),
                }],
                usage_tokens: 1,
                continuation: vec![],
            });
        }
        ensure!(n == 1, "unexpected fixture inference");
        self.reached.notify_one();
        self.release.notified().await;
        if let Some(preview) = preview {
            preview.delta("withdrawn-memory-marker");
        }
        Ok(model::Reply {
            text: "withdrawn-memory-marker".into(),
            calls: vec![],
            usage_tokens: 1,
            continuation: vec![],
        })
    }
}
impl ActorModel for HeldModel {
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<model::Reply>> + Send + '_>> {
        Box::pin(self.reply(request, None))
    }
    fn respond_stream(
        &self,
        request: Request,
        preview: preview::PreviewSink,
    ) -> Pin<Box<dyn Future<Output = Result<model::Reply>> + Send + '_>> {
        Box::pin(self.reply(request, Some(preview)))
    }
}

#[tokio::test]
async fn scoped_memory_actor_discards_inflight_reply_and_preview_after_source_changes() {
    let dir = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
    kernel.mark_ready().unwrap();
    let record = kernel
        .add_memory(AddMemory {
            request_id: lain_wire::RequestId::new(),
            draft: MemoryDraft {
                content: "synthetic evidence marker".into(),
                kind: MemoryKind::Episodic,
                scope: MemoryScope::Global,
            },
        })
        .unwrap();
    let model = Arc::new(HeldModel {
        record: record.id,
        calls: AtomicUsize::new(0),
        reached: Notify::new(),
        release: Notify::new(),
        requests: StdMutex::new(vec![]),
    });
    let actor = Actor::with_model(
        kernel.clone(),
        None,
        "synthetic-memory-owner-12345678901234567890"
            .parse()
            .unwrap(),
        model.clone(),
    );
    let conversation = kernel.create_conversation().unwrap();
    let mut events = actor.subscribe();
    let t = actor
        .submit(
            conversation.id,
            Uuid::new_v4(),
            "read the selected fact".into(),
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), model.reached.notified())
        .await
        .unwrap();
    let input = serde_json::to_string(&model.requests.lock().unwrap()[1]).unwrap();
    assert!(input.contains("synthetic evidence marker"));
    kernel
        .change_memory(
            record.id,
            ChangeMemory {
                expected_revision: 1,
                status: MemoryStatus::Disabled,
                superseded_by: None,
            },
        )
        .unwrap();
    model.release.notify_one();
    let settled = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let current = kernel.actor_turn(t.id).unwrap();
            if current.state != TurnState::Running {
                break current;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(settled.state, TurnState::Unknown);
    assert!(
        settled
            .messages
            .iter()
            .all(|m| !m.content.contains("withdrawn-memory-marker"))
    );
    while let Ok(event) = events.try_recv() {
        if let Observation::Preview(event) = event
            && let lain_wire::PreviewUpdate::Delta { text } = event.update
        {
            assert!(!text.contains("withdrawn-memory-marker"));
        }
    }
    assert_eq!(model.calls.load(Ordering::SeqCst), 2);
}

struct ProposalModel {
    record: MemoryId,
    calls: AtomicUsize,
}
impl ActorModel for ProposalModel {
    fn respond(
        &self,
        _: Request,
    ) -> Pin<Box<dyn Future<Output = Result<model::Reply>> + Send + '_>> {
        Box::pin(async move {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            ensure!(n < 2, "unexpected fixture inference");
            let (name, arguments) = if n == 0 {
                (
                    "memory_inspect",
                    json!({"memory_id":self.record,"scope":{"kind":"global"}}),
                )
            } else {
                (
                    "memory_propose",
                    json!({"draft":{"content":"synthetic derived fact","kind":"episodic","scope":{"kind":"global"}}}),
                )
            };
            Ok(model::Reply {
                text: String::new(),
                calls: vec![model::Call {
                    id: format!("fixture-{n}"),
                    name: name.into(),
                    arguments: arguments.to_string(),
                }],
                usage_tokens: 1,
                continuation: vec![],
            })
        })
    }
}
#[tokio::test]
async fn scoped_memory_actor_can_deny_or_cancel_after_approval_becomes_stale() {
    for cancel in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
        k.mark_ready().unwrap();
        let r = k
            .add_memory(AddMemory {
                request_id: lain_wire::RequestId::new(),
                draft: MemoryDraft {
                    content: "synthetic source".into(),
                    kind: MemoryKind::Episodic,
                    scope: MemoryScope::Global,
                },
            })
            .unwrap();
        let a = Actor::with_model(
            k.clone(),
            None,
            "synthetic-memory-owner-12345678901234567890"
                .parse()
                .unwrap(),
            Arc::new(ProposalModel {
                record: r.id,
                calls: AtomicUsize::new(0),
            }),
        );
        let c = k.create_conversation().unwrap();
        let t = a
            .submit(c.id, Uuid::new_v4(), "propose a derived fact".into())
            .await
            .unwrap();
        let pending = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let t = k.actor_turn(t.id).unwrap();
                if t.state != TurnState::Running {
                    break t;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(pending.state, TurnState::PendingConfirmation);
        let action = pending.actions.last().unwrap().id;
        k.change_memory(
            r.id,
            ChangeMemory {
                expected_revision: 1,
                status: MemoryStatus::Disabled,
                superseded_by: None,
            },
        )
        .unwrap();
        assert!(a.decide(t.id, action, true).await.is_err());
        assert_eq!(
            k.actor_turn(t.id).unwrap().state,
            TurnState::PendingConfirmation
        );
        let rejected = if cancel {
            a.cancel(t.id).await.unwrap()
        } else {
            a.decide(t.id, action, false).await.unwrap()
        };
        assert_eq!(
            rejected.state,
            if cancel {
                TurnState::Cancelled
            } else {
                TurnState::Completed
            }
        );
        assert_eq!(
            k.memory_candidate(CandidateId(action.0)).unwrap().status,
            CandidateStatus::Rejected
        );
    }
}
