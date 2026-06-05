use chrono::Utc;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::{
    config::{ProductAnalyticsConfig, ProductAnalyticsError},
    event::{AnalyticsProperties, ProductAnalyticsEvent},
    privacy::{add_pseudonymous_context, is_allowed_event, pseudonymous_id, sanitize_properties},
};

const BATCH_SIZE: usize = 20;
const CHANNEL_SIZE: usize = 1024;

#[derive(Clone)]
pub struct ProductAnalytics {
    inner: Option<std::sync::Arc<ProductAnalyticsInner>>,
}

struct ProductAnalyticsInner {
    sender: mpsc::Sender<CaptureEvent>,
    salt: String,
}

impl ProductAnalytics {
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    pub fn new(config: ProductAnalyticsConfig) -> Result<Self, ProductAnalyticsError> {
        if !config.enabled {
            return Ok(Self::disabled());
        }

        let project_token = config
            .project_token
            .filter(|value| !value.trim().is_empty())
            .ok_or(ProductAnalyticsError::MissingProjectToken)?;
        let salt = config
            .analytics_id_salt
            .filter(|value| !value.trim().is_empty())
            .ok_or(ProductAnalyticsError::MissingAnalyticsSalt)?;
        let endpoint = batch_endpoint(&config.host)?;
        let client = reqwest::Client::new();
        let (sender, receiver) = mpsc::channel(CHANNEL_SIZE);

        tokio::spawn(batch_worker(receiver, client, endpoint, project_token));

        Ok(Self {
            inner: Some(std::sync::Arc::new(ProductAnalyticsInner { sender, salt })),
        })
    }

    pub fn capture(&self, event: ProductAnalyticsEvent) {
        let Some(inner) = &self.inner else {
            return;
        };
        if !is_allowed_event(event.name) {
            return;
        }

        let Some(distinct_source) = event.user_id.or(event.workspace_id) else {
            return;
        };
        let distinct_prefix = if event.user_id.is_some() {
            "usr"
        } else {
            "wks"
        };
        let distinct_id = pseudonymous_id(&inner.salt, distinct_prefix, distinct_source);
        let mut properties = sanitize_properties(event.properties);

        add_pseudonymous_context(
            &mut properties,
            &inner.salt,
            event.user_id,
            event.workspace_id,
        );

        let capture = CaptureEvent {
            event: event.name.to_string(),
            distinct_id,
            properties,
            timestamp: Utc::now().to_rfc3339(),
        };

        if let Err(error) = inner.sender.try_send(capture) {
            tracing::warn!(%error, "PostHog product analytics event dropped");
        }
    }

    pub fn capture_user_event(
        &self,
        name: &'static str,
        user_id: uuid::Uuid,
        properties: AnalyticsProperties,
    ) {
        self.capture(ProductAnalyticsEvent::user(name, user_id).properties(properties));
    }

    pub fn capture_workspace_event(
        &self,
        name: &'static str,
        workspace_id: uuid::Uuid,
        properties: AnalyticsProperties,
    ) {
        self.capture(ProductAnalyticsEvent::workspace(name, workspace_id).properties(properties));
    }
}

#[derive(Debug, Clone, Serialize)]
struct CaptureEvent {
    event: String,
    distinct_id: String,
    properties: AnalyticsProperties,
    timestamp: String,
}

#[derive(Serialize)]
struct BatchRequest<'a> {
    api_key: &'a str,
    batch: &'a [CaptureEvent],
}

async fn batch_worker(
    mut receiver: mpsc::Receiver<CaptureEvent>,
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
    batch: &mut Vec<CaptureEvent>,
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

fn batch_endpoint(host: &str) -> Result<String, ProductAnalyticsError> {
    let parsed = reqwest::Url::parse(host).map_err(|_| ProductAnalyticsError::InvalidHost)?;
    match parsed.scheme() {
        "http" | "https" => Ok(format!("{}/batch/", host.trim_end_matches('/'))),
        _ => Err(ProductAnalyticsError::InvalidHost),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_client_drops_events_without_panic() {
        let analytics = ProductAnalytics::disabled();
        analytics.capture_workspace_event(
            "workspace.created",
            uuid::Uuid::parse_str("018f2f61-4875-7f7a-8bc8-8f70a73d2b1f").unwrap(),
            AnalyticsProperties::new(),
        );
    }
}
