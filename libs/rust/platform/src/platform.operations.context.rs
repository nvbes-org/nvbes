use crate::cockpit_model::ServiceId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceEndpoint {
    pub service: ServiceId,
    pub base_url: String,
}

#[derive(Clone)]
pub struct ContextClient {
    client: reqwest::Client,
    endpoints: Vec<ServiceEndpoint>,
}

#[derive(Serialize)]
pub struct ServiceContext {
    pub service: ServiceId,
    pub status: &'static str,
    pub checked_at: chrono::DateTime<Utc>,
    pub subject_context: &'static str,
}

impl ContextClient {
    pub fn new(endpoints: Vec<ServiceEndpoint>, production: bool) -> Result<Self, String> {
        for endpoint in &endpoints {
            let url = reqwest::Url::parse(&endpoint.base_url).map_err(|_| "invalid service URL")?;
            if !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
                || (url.scheme() != "https" && (production || url.scheme() != "http"))
            {
                return Err(
                    "service URLs must be HTTPS origins without credentials in production".into(),
                );
            }
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "invalid HTTP client")?;
        Ok(Self { client, endpoints })
    }

    pub async fn snapshot(&self) -> Vec<ServiceContext> {
        let mut result = Vec::new();
        for service in [
            ServiceId::Identity,
            ServiceId::Account,
            ServiceId::Billing,
            ServiceId::Email,
            ServiceId::TrustRisk,
        ] {
            let status =
                if let Some(endpoint) = self.endpoints.iter().find(|e| e.service == service) {
                    match self
                        .client
                        .get(format!(
                            "{}/health/ready",
                            endpoint.base_url.trim_end_matches('/')
                        ))
                        .send()
                        .await
                    {
                        Ok(response) if response.status().is_success() => "ready",
                        _ => "unavailable",
                    }
                } else {
                    "not_configured"
                };
            result.push(ServiceContext { service, status, checked_at: Utc::now(),
                subject_context: "Consult owner API and attach minimized evidence to the case; user session impersonation is unavailable" });
        }
        result
    }
}
