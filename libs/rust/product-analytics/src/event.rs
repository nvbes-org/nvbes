use serde_json::{Map, Value};
use uuid::Uuid;

pub type AnalyticsProperties = Map<String, Value>;

pub struct ProductAnalyticsEvent {
    pub(crate) name: &'static str,
    pub(crate) user_id: Option<Uuid>,
    pub(crate) workspace_id: Option<Uuid>,
    pub(crate) properties: AnalyticsProperties,
}

impl ProductAnalyticsEvent {
    pub fn user(name: &'static str, user_id: Uuid) -> Self {
        Self {
            name,
            user_id: Some(user_id),
            workspace_id: None,
            properties: AnalyticsProperties::new(),
        }
    }

    pub fn workspace(name: &'static str, workspace_id: Uuid) -> Self {
        Self {
            name,
            user_id: None,
            workspace_id: Some(workspace_id),
            properties: AnalyticsProperties::new(),
        }
    }

    pub fn workspace_for_user(name: &'static str, user_id: Uuid, workspace_id: Uuid) -> Self {
        Self {
            name,
            user_id: Some(user_id),
            workspace_id: Some(workspace_id),
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
}
