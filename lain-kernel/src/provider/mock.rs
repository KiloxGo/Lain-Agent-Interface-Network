use std::{collections::VecDeque, sync::Arc, time::Duration};

use async_stream::stream;
use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::{
    ModelError, ModelErrorKind, ModelEvent, ModelEventStream, ModelProvider, ModelRequest,
    ProviderCapabilities, Role,
};

#[derive(Debug, Clone)]
pub enum MockStep {
    Event(ModelEvent),
    Error(ModelError),
    Delay(Duration),
}

#[derive(Debug, Clone, Default)]
pub struct MockProvider {
    scripts: Arc<Mutex<VecDeque<Vec<MockStep>>>>,
    requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl MockProvider {
    pub fn new(scripts: impl IntoIterator<Item = Vec<MockStep>>) -> Self {
        Self {
            scripts: Arc::new(Mutex::new(scripts.into_iter().collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn requests(&self) -> Vec<ModelRequest> {
        self.requests.lock().await.clone()
    }
}

#[async_trait]
impl ModelProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            context_window: 4096,
            max_output_tokens: 1024,
            supports_streaming: true,
            supports_json_output: true,
            supports_tools: true,
            supports_strict_tools: true,
            supports_parallel_tools: true,
            supports_thinking: true,
            requires_reasoning_replay_for_tools: false,
            returns_usage: true,
            returns_cache_usage: true,
            supports_idempotency: true,
            supported_roles: vec![Role::System, Role::User, Role::Assistant, Role::Tool],
        }
    }

    async fn stream(
        &self,
        request: ModelRequest,
        cancellation: CancellationToken,
    ) -> Result<ModelEventStream, ModelError> {
        self.requests.lock().await.push(request);
        let steps = self.scripts.lock().await.pop_front().ok_or_else(|| {
            ModelError::new(ModelErrorKind::Configuration, "mock script queue is empty")
        })?;

        Ok(Box::pin(stream! {
            for step in steps {
                match step {
                    MockStep::Event(event) => yield Ok(event),
                    MockStep::Error(error) => {
                        yield Err(error);
                        break;
                    }
                    MockStep::Delay(duration) => {
                        tokio::select! {
                            () = cancellation.cancelled() => {
                                yield Err(ModelError::new(ModelErrorKind::Cancelled, "mock request cancelled"));
                                break;
                            }
                            () = tokio::time::sleep(duration) => {}
                        }
                    }
                }
            }
        }))
    }
}
