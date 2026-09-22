use base64::Engine;
use serde_json::{Map, Value};

use super::AppConfig;

impl AppConfig {
    pub async fn resolve_from_secret_manager(&mut self) -> Result<(), String> {
        if !self.secret_manager_enabled {
            return Ok(());
        }

        let secret_id = required_env("NVBES_SECRET_MANAGER_SECRET_ID")?;
        let region =
            std::env::var("NVBES_SECRET_MANAGER_REGION").unwrap_or_else(|_| "fr-par".to_string());
        if !matches!(region.as_str(), "fr-par" | "nl-ams" | "pl-waw") {
            return Err(
                "NVBES_SECRET_MANAGER_REGION must be fr-par, nl-ams, or pl-waw".to_string(),
            );
        }
        let auth_token = std::env::var("NVBES_SECRET_MANAGER_AUTH_TOKEN")
            .or_else(|_| std::env::var("SCW_SECRET_KEY"))
            .map_err(|_| {
                "NVBES_SECRET_MANAGER_AUTH_TOKEN or SCW_SECRET_KEY is required".to_string()
            })?;
        let api_base_url = std::env::var("NVBES_SECRET_MANAGER_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.scaleway.com".to_string());
        let url = format!(
            "{}/secret-manager/v1beta1/regions/{region}/secrets/{secret_id}/versions/latest/access",
            api_base_url.trim_end_matches('/')
        );
        let response = reqwest::Client::builder()
            .https_only(api_base_url.starts_with("https://"))
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|error| format!("cannot build Secret Manager client: {error}"))?
            .get(url)
            .header("X-Auth-Token", auth_token)
            .send()
            .await
            .map_err(|error| format!("Secret Manager is unavailable: {error}"))?
            .error_for_status()
            .map_err(|error| format!("Secret Manager rejected access: {error}"))?;
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| format!("Secret Manager returned invalid JSON: {error}"))?;
        let secrets = decoded_secret_object(payload)?;
        self.apply_secret_values(&secrets)?;
        super::validation::validate_config_urls_and_secrets(self)
    }

    fn apply_secret_values(&mut self, secrets: &Map<String, Value>) -> Result<(), String> {
        assign_required(secrets, "database_url", &mut self.database_url)?;
        assign_required(
            secrets,
            "billing_database_url",
            &mut self.billing_database_url,
        )?;
        assign_required(secrets, "jwt_secret", &mut self.jwt_secret)?;

        assign_optional(
            secrets,
            "auth_password_pepper",
            &mut self.auth_password_pepper,
        )?;
        assign_optional(
            secrets,
            "auth_factor_encryption_key",
            &mut self.auth_factor_encryption_key,
        )?;
        assign_optional(secrets, "stripe_secret_key", &mut self.stripe_secret_key)?;
        assign_optional(
            secrets,
            "stripe_webhook_secret",
            &mut self.stripe_webhook_secret,
        )?;
        assign_optional(secrets, "mollie_api_key", &mut self.mollie_api_key)?;
        assign_optional(secrets, "sentry_dsn", &mut self.sentry_dsn)?;
        assign_optional(
            secrets,
            "otlp_authorization_header",
            &mut self.otlp_authorization_header,
        )?;
        assign_optional(
            secrets,
            "product_analytics_token",
            &mut self.product_analytics_token,
        )?;
        assign_optional(secrets, "analytics_id_salt", &mut self.analytics_id_salt)?;
        assign_optional(
            secrets,
            "profiling_basic_auth_password",
            &mut self.profiling_basic_auth_password,
        )?;
        assign_optional(
            secrets,
            "observability_internal_token",
            &mut self.observability_internal_token,
        )?;
        assign_optional(secrets, "twilio_account_sid", &mut self.twilio_account_sid)?;
        assign_optional(secrets, "twilio_auth_token", &mut self.twilio_auth_token)?;
        assign_optional(secrets, "storage_access_key", &mut self.storage_access_key)?;
        assign_optional(secrets, "storage_secret_key", &mut self.storage_secret_key)?;
        assign_optional(secrets, "maxmind_account_id", &mut self.maxmind_account_id)?;
        assign_optional(
            secrets,
            "maxmind_license_key",
            &mut self.maxmind_license_key,
        )?;
        assign_optional(
            secrets,
            "request_e2ee_secret",
            &mut self.request_e2ee_secret,
        )?;
        Ok(())
    }
}

fn decoded_secret_object(payload: Value) -> Result<Map<String, Value>, String> {
    if let Some(data) = payload.get("data").and_then(Value::as_str) {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|error| format!("Secret Manager data is not valid base64: {error}"))?;
        return serde_json::from_slice::<Value>(&decoded)
            .map_err(|error| format!("Secret Manager data is not valid JSON: {error}"))?
            .as_object()
            .cloned()
            .ok_or_else(|| "Secret Manager data must be a JSON object".to_string());
    }
    payload
        .as_object()
        .cloned()
        .ok_or_else(|| "Secret Manager response must be a JSON object".to_string())
}

fn required_env(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is required"))
}

fn assign_required(
    secrets: &Map<String, Value>,
    name: &str,
    target: &mut String,
) -> Result<(), String> {
    if let Some(value) = secrets.get(name) {
        *target = value
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("Secret Manager field {name} must be a non-empty string"))?
            .to_string();
    }
    Ok(())
}

fn assign_optional(
    secrets: &Map<String, Value>,
    name: &str,
    target: &mut Option<String>,
) -> Result<(), String> {
    if let Some(value) = secrets.get(name) {
        *target = match value {
            Value::Null => None,
            Value::String(value) if !value.is_empty() => Some(value.clone()),
            _ => {
                return Err(format!(
                    "Secret Manager field {name} must be a non-empty string or null"
                ));
            }
        };
    }
    Ok(())
}

#[cfg(test)]
#[path = "config.secrets.tests.rs"]
mod tests;
