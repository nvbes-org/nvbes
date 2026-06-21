use chrono::Utc;
use serde::Serialize;
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
