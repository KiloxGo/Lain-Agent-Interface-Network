use std::{collections::BTreeMap, time::Duration};

use serde::Deserialize;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    config::{MemoryConfig, ProviderConfig},
    provider::{
        ModelMessage, ModelRequest, ModelRouter, Role, RouteKind, SharedProvider, ToolChoice,
    },
    storage::Storage,
};

use super::types::ExtractionEnvelope;

const EXTRACTION_SYSTEM_PROMPT: &str = include_str!("../../prompts/memory_extractor_v1.md");

#[derive(Debug, Deserialize)]
struct JobPayload {
    run_id: String,
}

pub async fn run(
    storage: Storage,
    provider: SharedProvider,
    router: ModelRouter,
    provider_config: ProviderConfig,
    config: MemoryConfig,
    shutdown: CancellationToken,
) {
    let worker_id = format!("memory-worker:{}", Uuid::now_v7());
    tracing::info!(worker_id, "memory extraction worker started");
    loop {
        if shutdown.is_cancelled() {
            break;
        }
        match process_one(
            &storage,
            &provider,
            &router,
            &provider_config,
            &config,
            &worker_id,
            shutdown.child_token(),
        )
        .await
        {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => tracing::warn!(error, "memory worker iteration failed"),
        }
        tokio::select! {
            () = shutdown.cancelled() => break,
            () = tokio::time::sleep(config.poll_interval) => {}
        }
    }
    tracing::info!(worker_id, "memory extraction worker stopped");
}

pub async fn process_one(
    storage: &Storage,
    provider: &SharedProvider,
    router: &ModelRouter,
    provider_config: &ProviderConfig,
    config: &MemoryConfig,
    worker_id: &str,
    cancellation: CancellationToken,
) -> Result<bool, String> {
    let job = storage
        .claim_durable_job("memory_extract", worker_id, config.lease_duration)
        .await
        .map_err(|error| error.to_string())?;
    let Some(job) = job else {
        return Ok(false);
    };
    let result = async {
        let payload: JobPayload = serde_json::from_str(&job.payload_json)
            .map_err(|error| format!("job_payload_invalid:{error}"))?;
        let source = storage
            .load_extraction_source(&payload.run_id)
            .await
            .map_err(|error| format!("source_load_failed:{error}"))?;
        let route = router.route(RouteKind::Maintenance);
        let user_payload = serde_json::to_string(&json!({
            "user_text": source.user_text,
            "observed_at": source.observed_at,
            "maximum_candidates": config.max_candidates_per_message,
        }))
        .map_err(|error| format!("request_encode_failed:{error}"))?;
        let request = ModelRequest {
            request_id: format!("memory:{}", job.id),
            model: route.model,
            messages: vec![
                ModelMessage::text(Role::System, EXTRACTION_SYSTEM_PROMPT),
                ModelMessage::text(Role::User, user_payload),
            ],
            tools: Vec::new(),
            tool_choice: ToolChoice::None,
            temperature: Some(0.0),
            top_p: None,
            max_output_tokens: Some(provider_config.max_output_tokens.min(1536)),
            response_format: route.response_format,
            thinking_mode: route.thinking_mode,
            timeout: provider_config.request_timeout,
            metadata: BTreeMap::from([
                ("route".to_string(), "memory_extract".to_string()),
                (
                    "extractor_version".to_string(),
                    super::types::EXTRACTOR_VERSION.to_string(),
                ),
                (
                    "source_message_id".to_string(),
                    source.user_message_id.clone(),
                ),
            ]),
        };
        let value = provider
            .complete_json(request, cancellation)
            .await
            .map_err(|error| format!("{}:{error}", error.code()))?;
        let mut envelope: ExtractionEnvelope = serde_json::from_value(value)
            .map_err(|error| format!("extraction_schema_invalid:{error}"))?;
        envelope
            .candidates
            .truncate(config.max_candidates_per_message);
        for candidate in &envelope.candidates {
            storage
                .record_memory_candidate(&source, candidate)
                .await
                .map_err(|error| format!("candidate_persist_failed:{error}"))?;
        }
        Ok::<usize, String>(envelope.candidates.len())
    }
    .await;

    match result {
        Ok(candidate_count) => {
            let completed = storage
                .complete_durable_job(&job.id, worker_id)
                .await
                .map_err(|error| error.to_string())?;
            if !completed {
                return Err("memory job lease was lost before completion".to_string());
            }
            tracing::info!(
                job_id = job.id,
                candidate_count,
                "memory extraction completed"
            );
        }
        Err(error) => {
            let retry_secs = 2_u64.saturating_pow(job.attempts.clamp(1, 6) as u32);
            let code = error.split(':').next().unwrap_or("memory_extract_failed");
            let status = storage
                .fail_durable_job(&job, worker_id, code, Duration::from_secs(retry_secs))
                .await
                .map_err(|storage_error| storage_error.to_string())?;
            tracing::warn!(job_id = job.id, status, code, "memory extraction failed");
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        provider::{
            FinishReason, ModelEvent,
            mock::{MockProvider, MockStep},
        },
        storage::CompletionUsage,
    };

    use super::*;

    #[tokio::test]
    async fn worker_extracts_and_commits_grounded_memory() {
        let storage = Storage::in_memory().await.expect("storage");
        let session_id = storage.scope().session_id.clone();
        let accepted = storage
            .accept_user_input(
                &session_id,
                "worker-memory",
                "I prefer dark mode",
                "mock",
                "flash",
                "identity-v1",
            )
            .await
            .expect("accept");
        storage.start_run(&accepted.run.id).await.expect("start");
        storage
            .complete_reply(
                &accepted.run.id,
                "Noted.",
                "stop",
                CompletionUsage::default(),
            )
            .await
            .expect("complete");
        let response = json!({"candidates": [{
            "memory_type": "preference",
            "scope": "user",
            "subject": "user",
            "predicate": "interface_theme",
            "value": "dark mode",
            "evidence_quote": "I prefer dark mode",
            "explicitness": "explicit",
            "confidence": 0.99,
            "importance": 0.6,
            "sensitivity": "personal",
            "valid_from": null,
            "valid_to": null
        }]});
        let mock = Arc::new(MockProvider::new([vec![
            MockStep::Event(ModelEvent::TextDelta {
                text: response.to_string(),
            }),
            MockStep::Event(ModelEvent::Finish {
                reason: FinishReason::Stop,
            }),
        ]]));
        let provider = mock.clone() as SharedProvider;
        let config = MemoryConfig {
            extraction_enabled: true,
            poll_interval: Duration::from_millis(1),
            lease_duration: Duration::from_secs(30),
            max_candidates_per_message: 8,
        };
        assert!(
            process_one(
                &storage,
                &provider,
                &ModelRouter::new("flash".to_string(), "pro".to_string()),
                &ProviderConfig::for_test(),
                &config,
                "test-worker",
                CancellationToken::new(),
            )
            .await
            .expect("process")
        );
        let requests = mock.requests().await;
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].metadata.get("route").map(String::as_str),
            Some("memory_extract")
        );
        let evidence = storage
            .retrieve_memory_evidence(&session_id, "dark mode", 8)
            .await
            .expect("retrieve committed memory");
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].text.contains("dark mode"));
        assert!(
            storage
                .claim_durable_job("memory_extract", "another-worker", Duration::from_secs(30))
                .await
                .expect("claim after completion")
                .is_none()
        );
    }
}
