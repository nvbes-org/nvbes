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
#[path = "billing.worker.queue.tests.rs"]
mod tests;
