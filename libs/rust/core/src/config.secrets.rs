use super::AppConfig;
use super::validation::validate_config_urls_and_secrets;

impl AppConfig {
    pub async fn resolve_from_secret_manager(&mut self) -> Result<(), String> {
        if !self.secret_manager_enabled {
            return Ok(());
        }

        let scw_config = crate::scw_kms::KmsConfig {
            access_key: self.scw_access_key.clone().ok_or_else(|| {
                "SCW_ACCESS_KEY required when secret_manager_enabled is true".to_string()
            })?,
            secret_key: self.scw_secret_key.clone().ok_or_else(|| {
                "SCW_SECRET_KEY required when secret_manager_enabled is true".to_string()
            })?,
            project_id: self.scw_project_id.clone().ok_or_else(|| {
                "SCW_DEFAULT_PROJECT_ID required when secret_manager_enabled is true".to_string()
            })?,
            region: self.scw_region.clone(),
            key_id: self.scw_kms_key_id.clone(),
            enabled: true,
        };

        let resolver = crate::scw_secrets::SecretResolver::new(&scw_config, &self.environment);
        let secrets = resolver.resolve_secrets().await?;

        if let Some(v) = secrets.get("NVBES_DATABASE_URL") {
            self.database_url = v.clone();
        }
        if let Some(v) = secrets.get("NVBES_JWT_SECRET") {
            self.jwt_secret = v.clone();
        }
        if let Some(v) = secrets.get("NVBES_STRIPE_SECRET_KEY") {
            self.stripe_secret_key = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_STRIPE_WEBHOOK_SECRET") {
            self.stripe_webhook_secret = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_SENTRY_DSN") {
            self.sentry_dsn = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_OTLP_ENDPOINT") {
            self.otlp_endpoint = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_OTLP_AUTHORIZATION_HEADER") {
            self.otlp_authorization_header = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_POSTHOG_PROJECT_TOKEN") {
            self.posthog_project_token = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_ANALYTICS_ID_SALT") {
            self.analytics_id_salt = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_PROFILING_ENDPOINT") {
            self.profiling_endpoint = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_PROFILING_BASIC_AUTH_USER") {
            self.profiling_basic_auth_user = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_PROFILING_BASIC_AUTH_PASSWORD") {
            self.profiling_basic_auth_password = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_OBSERVABILITY_INTERNAL_TOKEN") {
            self.observability_internal_token = Some(v.clone());
        }
        if let Some(v) = secrets.get("SCW_TEM_FROM_EMAIL") {
            self.scw_tem_from_email = Some(v.clone());
        }
        if let Some(v) = secrets.get("SCW_TEM_WEBHOOK_SECRET") {
            self.scw_tem_webhook_secret = Some(v.clone());
        }
        if let Some(v) = secrets.get("TURNSTILE_SECRET_KEY") {
            self.turnstile_secret_key = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_REQUEST_E2EE_SECRET") {
            self.request_e2ee_secret = Some(v.clone());
        }
        if let Some(v) = secrets.get("NVBES_REDIS_URL") {
            self.redis_url = v.clone();
        }
        if let Some(v) = secrets.get("NVBES_REDIS_PASSWORD") {
            self.redis_password = Some(v.clone());
        }

        validate_config_urls_and_secrets(self)?;
        tracing::info!("Config resolved from Scaleway Secret Manager");
        Ok(())
    }
}
