use nvbes_product_analytics::{CapturedProductAnalyticsEvent, ProductAnalyticsSink};
use serde::Serialize;
use tokio::sync::mpsc;

const BATCH_SIZE: usize = 20;
const CHANNEL_SIZE: usize = 1024;

#[derive(Debug, Clone)]
pub struct PostHogAnalyticsConfig {
    pub host: String,
    pub project_token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PostHogAnalyticsError {
    #[error("PostHog host must be a valid HTTP(S) URL")]
    InvalidHost,
    #[error("PostHog project token is required")]
    MissingProjectToken,
}

#[derive(Clone)]
pub struct PostHogAnalyticsSink {
    sender: mpsc::Sender<CapturedProductAnalyticsEvent>,
}

impl PostHogAnalyticsSink {
    pub fn new(config: PostHogAnalyticsConfig) -> Result<Self, PostHogAnalyticsError> {
        if config.project_token.trim().is_empty() {
            return Err(PostHogAnalyticsError::MissingProjectToken);
        }

        let endpoint = batch_endpoint(&config.host)?;
        let client = reqwest::Client::new();
        let (sender, receiver) = mpsc::channel(CHANNEL_SIZE);
        tokio::spawn(batch_worker(
            receiver,
            client,
            endpoint,
            config.project_token,
        ));

        Ok(Self { sender })
    }
}

impl ProductAnalyticsSink for PostHogAnalyticsSink {
    fn capture(&self, event: CapturedProductAnalyticsEvent) {
        if let Err(error) = self.sender.try_send(event) {
            tracing::warn!(%error, "PostHog product analytics event dropped");
        }
    }
}

#[derive(Serialize)]
struct BatchRequest<'a> {
    api_key: &'a str,
    batch: &'a [CapturedProductAnalyticsEvent],
}

async fn batch_worker(
    mut receiver: mpsc::Receiver<CapturedProductAnalyticsEvent>,
    client: reqwest::Client,
    endpoint: String,
    project_token: String,
) {
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        tokio::select! {
            maybe_event = receiver.recv() => {
                let Some(event) = maybe_event else {
                    flush_batch(&client, &endpoint, &project_token, &mut batch).await;
                    return;
                };
                batch.push(event);
                if batch.len() >= BATCH_SIZE {
                    flush_batch(&client, &endpoint, &project_token, &mut batch).await;
                }
            }
            _ = interval.tick() => {
                flush_batch(&client, &endpoint, &project_token, &mut batch).await;
            }
        }
    }
}

async fn flush_batch(
    client: &reqwest::Client,
    endpoint: &str,
    project_token: &str,
    batch: &mut Vec<CapturedProductAnalyticsEvent>,
) {
    if batch.is_empty() {
        return;
    }

    let payload = std::mem::take(batch);
    let request = BatchRequest {
        api_key: project_token,
        batch: &payload,
    };

    match client.post(endpoint).json(&request).send().await {
        Ok(response) if !response.status().is_success() => {
            tracing::warn!(status = %response.status(), "PostHog product analytics batch rejected");
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(%error, "PostHog product analytics batch failed"),
    }
}

fn batch_endpoint(host: &str) -> Result<String, PostHogAnalyticsError> {
    let parsed = reqwest::Url::parse(host).map_err(|_| PostHogAnalyticsError::InvalidHost)?;
    match parsed.scheme() {
        "http" | "https" => Ok(format!("{}/batch/", host.trim_end_matches('/'))),
        _ => Err(PostHogAnalyticsError::InvalidHost),
    }
}
