use std::sync::Arc;

use async_trait::async_trait;
use aws_sdk_sqs::{Client, config::Credentials};
use tokio::sync::mpsc;

use crate::config::{DispatchMode, DispatchQueueConfig};

#[async_trait]
pub trait DispatchQueue: Send + Sync {
    async fn enqueue(&self, event_id: &str) -> anyhow::Result<()>;
}

pub struct QueueRuntime {
    pub publisher: Arc<dyn DispatchQueue>,
    pub local_receiver: Option<mpsc::Receiver<String>>,
}

pub fn runtime(mode: &DispatchMode) -> QueueRuntime {
    match mode {
        DispatchMode::InMemory => {
            let (sender, receiver) = mpsc::channel(256);
            QueueRuntime {
                publisher: Arc::new(InMemoryDispatchQueue { sender }),
                local_receiver: Some(receiver),
            }
        }
        DispatchMode::Scaleway(config) => QueueRuntime {
            publisher: Arc::new(ScalewayDispatchQueue::new(config)),
            local_receiver: None,
        },
    }
}

pub struct InMemoryDispatchQueue {
    sender: mpsc::Sender<String>,
}

#[async_trait]
impl DispatchQueue for InMemoryDispatchQueue {
    async fn enqueue(&self, event_id: &str) -> anyhow::Result<()> {
        self.sender
            .send(event_id.to_string())
            .await
            .map_err(|_| anyhow::anyhow!("local billing dispatch queue is closed"))
    }
}

pub struct ScalewayDispatchQueue {
    client: Client,
    queue_url: String,
}

impl ScalewayDispatchQueue {
    pub fn new(config: &DispatchQueueConfig) -> Self {
        let credentials = Credentials::new(
            config.access_key.clone(),
            config.secret_key.clone(),
            None,
            None,
            "nvbes-billing-worker",
        );
        let sdk_config = aws_sdk_sqs::Config::builder()
            .behavior_version(aws_sdk_sqs::config::BehaviorVersion::latest())
            .credentials_provider(credentials)
            .endpoint_url(&config.endpoint)
            .region(aws_sdk_sqs::config::Region::new(config.region.clone()))
            .build();
        Self {
            client: Client::from_conf(sdk_config),
            queue_url: config.queue_url.clone(),
        }
    }
}

#[async_trait]
impl DispatchQueue for ScalewayDispatchQueue {
    async fn enqueue(&self, event_id: &str) -> anyhow::Result<()> {
        self.client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(event_id)
            .send()
            .await
            .map_err(|error| anyhow::anyhow!("billing dispatch enqueue failed: {error}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DispatchQueueConfig;

    #[tokio::test]
    async fn in_memory_queue_enqueues_and_receives() {
        let mut runtime = runtime(&DispatchMode::InMemory);
        let rx = runtime.local_receiver.as_mut().expect("must have receiver");
        runtime
            .publisher
            .enqueue("evt_123")
            .await
            .expect("enqueue should succeed");
        let received = rx.recv().await.expect("must receive event");
        assert_eq!(received, "evt_123");
    }

    #[test]
    fn scaleway_runtime_builds_without_local_receiver() {
        let mode = DispatchMode::Scaleway(DispatchQueueConfig {
            endpoint: "https://sqs.mnq.fr-par.scaleway.com".into(),
            queue_url: "https://sqs.mnq.fr-par.scaleway.com/123/billing-dispatch".into(),
            region: "fr-par".into(),
            access_key: "SCWXXXXXXXXXXXXXXXXX".into(),
            secret_key: "11111111-2222-3333-4444-555555555555".into(),
        });
        let runtime = runtime(&mode);
        assert!(runtime.local_receiver.is_none());
    }
}
