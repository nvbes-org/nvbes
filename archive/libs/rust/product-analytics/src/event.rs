use serde_json::{Map, Value};
use uuid::Uuid;

pub type AnalyticsProperties = Map<String, Value>;

pub struct ProductAnalyticsEvent {
    pub(crate) name: &'static str,
    pub(crate) user_id: Option<Uuid>,
    pub(crate) workspace_id: Option<Uuid>,
    pub(crate) correlation: Option<ProductAnalyticsCorrelation>,
    pub(crate) properties: AnalyticsProperties,
}

pub(crate) struct ProductAnalyticsCorrelation {
    pub(crate) distinct_id: String,
    pub(crate) session_id: String,
}

impl ProductAnalyticsEvent {
    pub fn user(name: &'static str, user_id: Uuid) -> Self {
        Self {
            name,
            user_id: Some(user_id),
            workspace_id: None,
            correlation: None,
            properties: AnalyticsProperties::new(),
        }
    }

    pub fn workspace(name: &'static str, workspace_id: Uuid) -> Self {
        Self {
            name,
            user_id: None,
            workspace_id: Some(workspace_id),
            correlation: None,
            properties: AnalyticsProperties::new(),
        }
    }

    pub fn workspace_for_user(name: &'static str, user_id: Uuid, workspace_id: Uuid) -> Self {
        Self {
            name,
            user_id: Some(user_id),
            workspace_id: Some(workspace_id),
            correlation: None,
            properties: AnalyticsProperties::new(),
        }
    }

    pub fn property(mut self, key: &'static str, value: impl Into<Value>) -> Self {
        self.properties.insert(key.to_string(), value.into());
        self
    }

    pub fn properties(mut self, properties: AnalyticsProperties) -> Self {
        self.properties.extend(properties);
        self
    }

    pub fn correlation(mut self, distinct_id: Option<&str>, session_id: Option<&str>) -> Self {
        self.correlation = match (distinct_id, session_id) {
            (Some(distinct_id), Some(session_id))
                if is_safe_correlation_id(distinct_id) && is_safe_correlation_id(session_id) =>
            {
                Some(ProductAnalyticsCorrelation {
                    distinct_id: distinct_id.to_string(),
                    session_id: session_id.to_string(),
                })
            }
            _ => None,
        };
        self
    }
}

fn is_safe_correlation_id(value: &str) -> bool {
    (8..=200).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

pub fn properties(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> AnalyticsProperties {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

pub fn size_bytes_bucket(bytes: i64) -> &'static str {
    if bytes < 1024 * 1024 {
        "<1mb"
    } else if bytes < 10 * 1024 * 1024 {
        "1-10mb"
    } else if bytes < 100 * 1024 * 1024 {
        "10-100mb"
    } else if bytes < 1024 * 1024 * 1024 {
        "100mb-1gb"
    } else {
        "1gb+"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_buckets_are_coarse() {
        assert_eq!(size_bytes_bucket(10), "<1mb");
        assert_eq!(size_bytes_bucket(2 * 1024 * 1024), "1-10mb");
        assert_eq!(size_bytes_bucket(2 * 1024 * 1024 * 1024), "1gb+");
    }

    #[test]
    fn correlation_rejects_values_that_could_contain_pii() {
        let event = ProductAnalyticsEvent::user(
            "auth.signup_completed",
            Uuid::parse_str("018f2f61-4875-7f7a-8bc8-8f70a73d2b1f").unwrap(),
        )
        .correlation(Some("person@example.com"), Some("session_12345678"));

        assert!(event.correlation.is_none());
    }
}
