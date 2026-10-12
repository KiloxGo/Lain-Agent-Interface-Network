//! Closed typed registry; no shell, filesystem, HTTP, configuration or approval tool.
use crate::{
    backend::Supervisor,
    domain::{RuntimeProfileId, SandboxMode, SessionOptions, WorkSessionId},
    kernel::{Kernel, blocking},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "arguments", deny_unknown_fields)]
pub enum Tool {
    #[serde(rename = "work.list")]
    WorkList(Empty),
    #[serde(rename = "work.inspect")]
    WorkInspect(SessionArg),
    #[serde(rename = "work.history")]
    WorkHistory(HistoryArg),
    #[serde(rename = "profile.list")]
    ProfileList(Empty),
    #[serde(rename = "profile.inspect")]
    ProfileInspect(ProfileArg),
    #[serde(rename = "work.create")]
    WorkCreate(CreateArg),
    #[serde(rename = "work.send")]
    WorkSend(SendArg),
    #[serde(rename = "work.cancel")]
    WorkCancel(SessionArg),
    #[serde(rename = "work.resume")]
    WorkResume(SessionArg),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionArg {
    pub session_id: WorkSessionId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryArg {
    pub session_id: WorkSessionId,
    pub after: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileArg {
    pub profile_id: RuntimeProfileId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateArg {
    pub workspace: PathBuf,
    pub profile_id: RuntimeProfileId,
    pub sandbox: SandboxMode,
    pub task: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendArg {
    pub session_id: WorkSessionId,
    pub text: String,
}
impl Tool {
    pub fn mutating(&self) -> bool {
        matches!(
            self,
            Self::WorkCreate(_) | Self::WorkSend(_) | Self::WorkCancel(_) | Self::WorkResume(_)
        )
    }
    pub fn validate(&self) -> Result<()> {
        let text = match self {
            Self::WorkCreate(a) => {
                ensure!(
                    a.workspace.is_absolute(),
                    "workspace must be absolute on daemon host"
                );
                Some(&a.task)
            }
            Self::WorkSend(a) => Some(&a.text),
            _ => None,
        };
        if let Some(text) = text {
            ensure!(
                !text.trim().is_empty() && text.len() <= 8192,
                "tool text must contain 1–8192 bytes"
            );
        }
        if let Self::WorkHistory(a) = self {
            ensure!(a.after <= i64::MAX as u64, "invalid history cursor");
        }
        Ok(())
    }
    pub fn parse(wire_name: &str, arguments: &str) -> Result<Self> {
        ensure!(
            arguments.len() <= 12288 && wire_name.len() <= 64,
            "tool call too large"
        );
        let arguments: Value = serde_json::from_str(arguments)
            .map_err(|_| anyhow::anyhow!("malformed tool arguments"))?;
        let name = wire_name.replace('_', ".");
        let tool: Tool = serde_json::from_value(json!({"name":name,"arguments":arguments}))
            .map_err(|_| anyhow::anyhow!("unknown tool or invalid arguments"))?;
        tool.validate()?;
        Ok(tool)
    }
}
pub fn definitions() -> Vec<Value> {
    let string = || json!({"type":"string"});
    let session = || json!({"session_id":{"type":"string","format":"uuid"}});
    [
        ("work_list", "List up to 50 work sessions; does not start workers.", json!({})),
        ("work_inspect", "Inspect a real work session.", session()),
        ("work_history", "Read up to 20 bounded history events after a cursor.", json!({"session_id":{"type":"string","format":"uuid"},"after":{"type":"integer","minimum":0}})),
        ("profile_list", "List safe runtime profile health.", json!({})),
        ("profile_inspect", "Inspect a runtime profile without credentials.", json!({"profile_id":string()})),
        ("work_create", "Propose delegation to Codex. Requires owner confirmation. Workspace is an absolute daemon-host path; task is submitted after creating real work. Defaults should be codex/local and read_only. Never fabricate a workspace.", json!({"workspace":string(),"profile_id":string(),"sandbox":{"type":"string","enum":["read_only","workspace_write"]},"task":string()})),
        ("work_send", "Propose a follow-up instruction; owner confirmation required.", json!({"session_id":{"type":"string","format":"uuid"},"text":string()})),
        ("work_cancel", "Propose worker cancellation; owner confirmation required. Acknowledgement is not completion.", session()),
        ("work_resume", "Propose worker attachment/reconciliation; owner confirmation required.", session()),
    ].into_iter().map(|(name, description, properties)| {
        let required: Vec<_> = properties.as_object().unwrap().keys().cloned().collect();
        json!({"type":"function","name":name,"description":description,"strict":true,"parameters":{"type":"object","properties":properties,"required":required,"additionalProperties":false}})
    }).collect()
}
pub async fn read(
    kernel: &Arc<Kernel>,
    workers: Option<&Arc<Supervisor>>,
    tool: &Tool,
) -> Result<Value> {
    Ok(match tool {
        Tool::WorkList(_) => {
            let mut sessions = blocking(kernel, Kernel::list_sessions).await?;
            sessions.sort_by_key(|s| std::cmp::Reverse(s.updated_at_ms));
            let more = sessions.len() > 50;
            sessions.truncate(50);
            json!({"sessions":sessions,"truncated":more})
        }
        Tool::WorkInspect(a) => {
            let id = a.session_id;
            json!(
                blocking(kernel, move |k| k.session(id))
                    .await?
                    .ok_or(crate::backend::ControlError::NotFound)?
            )
        }
        Tool::WorkHistory(a) => {
            let a = a.clone();
            json!(blocking(kernel, move |k| k.history(a.session_id, a.after, None, 20)).await?)
        }
        Tool::ProfileList(_) => {
            let mut profiles = workers
                .ok_or(crate::backend::ControlError::Unavailable)?
                .profiles()
                .await?;
            let more = profiles.len() > 50;
            profiles.truncate(50);
            let profiles: Vec<_> = profiles.into_iter().map(actor_profile_view).collect();
            json!({"profiles":profiles,"truncated":more})
        }
        Tool::ProfileInspect(a) => actor_profile_view(
            workers
                .ok_or(crate::backend::ControlError::Unavailable)?
                .profiles()
                .await?
                .into_iter()
                .find(|p| p.profile.definition.id == a.profile_id)
                .ok_or(crate::backend::ControlError::NotFound)?,
        ),
        _ => anyhow::bail!("mutating tool requires owner confirmation"),
    })
}
// The model only needs credential availability. SecretRef handles remain
// inspectable through owner profile APIs, outside ordinary Actor context/results.
fn actor_profile_view(profile: crate::profiles::ProfileView) -> Value {
    let mut value = json!(profile);
    value
        .as_object_mut()
        .expect("profile object")
        .remove("secret_ref");
    value
}
/// `created` persists the session ID before prompt submission, preserving partial delegation.
pub async fn mutate(
    workers: &Arc<Supervisor>,
    tool: &Tool,
    created: impl AsyncFnOnce(Value) -> Result<()>,
) -> Result<Value> {
    Ok(match tool {
        Tool::WorkCreate(a) => {
            let session = workers
                .create_with_profile(
                    a.workspace.clone(),
                    Some(a.profile_id.clone()),
                    SessionOptions {
                        model: None,
                        sandbox: Some(a.sandbox),
                    },
                )
                .await?;
            created(json!({"work_session_id":session.id,"stage":"created_not_submitted"})).await?;
            let turn = workers.prompt(session.id, a.task.clone()).await?;
            json!({"work_session_id":session.id,"turn_id":turn.id,"stage":"submitted","worker_completion":"not_observed"})
        }
        Tool::WorkSend(a) => json!(workers.prompt(a.session_id, a.text.clone()).await?),
        Tool::WorkCancel(a) => json!(workers.cancel(a.session_id).await?),
        Tool::WorkResume(a) => json!(workers.resume(a.session_id).await?),
        _ => anyhow::bail!("not a mutation"),
    })
}
