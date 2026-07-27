use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    config::{ProductAnalyticsConfig, ProductAnalyticsError},
    event::{AnalyticsProperties, ProductAnalyticsEvent},
    privacy::{add_pseudonymous_context, is_allowed_event, pseudonymous_id, sanitize_properties},
};

#[derive(Clone)]
pub struct ProductAnalytics {
    inner: Option<std::sync::Arc<ProductAnalyticsInner>>,
}

struct ProductAnalyticsInner {
    sink: Arc<dyn ProductAnalyticsSink>,
    salt: String,
}

pub trait ProductAnalyticsSink: Send + Sync {
    fn capture(&self, event: CapturedProductAnalyticsEvent);
}

impl ProductAnalytics {
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    pub fn new(config: ProductAnalyticsConfig) -> Result<Self, ProductAnalyticsError> {
        if !config.enabled {
            return Ok(Self::disabled());
        }

        tracing::warn!(
            "Product analytics requested without an analytics adapter; events will be dropped"
        );
        Ok(Self::disabled())
    }

    pub fn with_sink(
        config: ProductAnalyticsConfig,
        sink: Arc<dyn ProductAnalyticsSink>,
    ) -> Result<Self, ProductAnalyticsError> {
        if !config.enabled {
            return Ok(Self::disabled());
        }

        let salt = config
            .analytics_id_salt
            .filter(|value| !value.trim().is_empty())
            .ok_or(ProductAnalyticsError::MissingAnalyticsSalt)?;

        Ok(Self {
            inner: Some(std::sync::Arc::new(ProductAnalyticsInner { sink, salt })),
        })
    }

    pub fn capture(&self, event: ProductAnalyticsEvent) {
        let Some(inner) = &self.inner else {
            return;
        };
        if !is_allowed_event(event.name) {
            return;
        }

        let distinct_id = if let Some(correlation) = &event.correlation {
            correlation.distinct_id.clone()
        } else {
            let Some(distinct_source) = event.user_id.or(event.workspace_id) else {
                return;
            };
            let distinct_prefix = if event.user_id.is_some() {
                "usr"
            } else {
                "wks"
            };
            pseudonymous_id(&inner.salt, distinct_prefix, distinct_source)
        };
        let mut properties = sanitize_properties(event.properties);

        add_pseudonymous_context(
            &mut properties,
            &inner.salt,
            event.user_id,
            event.workspace_id,
        );
        if let Some(correlation) = event.correlation {
            properties.insert("$session_id".to_string(), json!(correlation.session_id));
        }

        let capture = CapturedProductAnalyticsEvent {
            event: event.name.to_string(),
            distinct_id,
            properties,
            timestamp: Utc::now().to_rfc3339(),
        };

        inner.sink.capture(capture);
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
pub struct CapturedProductAnalyticsEvent {
    pub event: String,
    pub distinct_id: String,
    pub properties: AnalyticsProperties,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<CapturedProductAnalyticsEvent>>,
    }

    impl ProductAnalyticsSink for RecordingSink {
        fn capture(&self, event: CapturedProductAnalyticsEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[test]
    fn disabled_client_drops_events_without_panic() {
        let analytics = ProductAnalytics::disabled();
        analytics.capture_workspace_event(
            "workspace.created",
            uuid::Uuid::parse_str("018f2f61-4875-7f7a-8bc8-8f70a73d2b1f").unwrap(),
            AnalyticsProperties::new(),
        );
    }

    #[test]
    fn browser_correlation_connects_server_event_to_posthog_session() {
        let sink = Arc::new(RecordingSink::default());
        let analytics = ProductAnalytics::with_sink(
            ProductAnalyticsConfig {
                enabled: true,
                analytics_id_salt: Some("a sufficiently strong analytics salt".to_string()),
            },
            sink.clone(),
        )
        .unwrap();

        analytics.capture(
            ProductAnalyticsEvent::user(
                "auth.signup_completed",
                uuid::Uuid::parse_str("018f2f61-4875-7f7a-8bc8-8f70a73d2b1f").unwrap(),
            )
            .correlation(Some("distinct_12345678"), Some("session_12345678")),
        );

        let events = sink.events.lock().unwrap();
        assert_eq!(events[0].distinct_id, "distinct_12345678");
        assert_eq!(
            events[0].properties.get("$session_id"),
            Some(&json!("session_12345678"))
        );
    }
}
