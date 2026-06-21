use super::AppConfig;

impl AppConfig {
    pub async fn resolve_from_secret_manager(&mut self) -> Result<(), String> {
        if !self.secret_manager_enabled {
            return Ok(());
        }

        Err(
            "NVBES_SECRET_MANAGER_ENABLED requires a Cloud secret-manager adapter. OSS reads configuration from environment variables."
                .to_string(),
        )
    }
}
