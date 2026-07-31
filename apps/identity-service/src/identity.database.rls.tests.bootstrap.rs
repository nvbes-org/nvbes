use std::ffi::OsString;

use anyhow::ensure;

use super::support::RlsTestDatabase;

pub async fn ensure_redis_is_ready() {
    crate::test_support::ensure_test_redis().await;
}

pub async fn assert_existing_owner_bootstrap(database: &RlsTestDatabase) -> anyhow::Result<()> {
    let _environment = EnvironmentGuard::development();
    let config = crate::app::AppConfig {
        app_name: "account-rls-baseline-bootstrap-test".to_string(),
        environment: "development".to_string(),
        web_base_url: "http://localhost:3001".to_string(),
        api_base_url: "http://localhost:4000".to_string(),
        email_provider: "mock".to_string(),
        otp_provider: "mock".to_string(),
        redis_url: std::env::var("NVBES_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        redis_password: std::env::var("NVBES_REDIS_PASSWORD")
            .ok()
            .filter(|password| !password.trim().is_empty()),
        redis_max_connections: 2,
        ..Default::default()
    };

    let state = crate::app::AppState::bootstrap(&config, database.owner.clone()).await?;
    ensure!(
        state
            .allowed_browser_origins
            .contains("http://localhost:3001"),
        "the existing owner-backed AppState bootstrap must load browser origins"
    );

    let owner_bypasses_rls = sqlx::query_scalar::<_, bool>(
        "SELECT role.rolsuper
             OR role.rolbypassrls
             OR database.datdba = role.oid
         FROM pg_roles AS role
         CROSS JOIN pg_database AS database
         WHERE role.rolname = current_user
           AND database.datname = current_database()",
    )
    .fetch_one(&state.db)
    .await?;
    ensure!(
        owner_bypasses_rls,
        "baseline smoke is expected to document the current privileged bootstrap path"
    );
    Ok(())
}

struct EnvironmentGuard {
    previous: Option<OsString>,
}

impl EnvironmentGuard {
    fn development() -> Self {
        let previous = std::env::var_os("NVBES_ENV");
        unsafe {
            std::env::set_var("NVBES_ENV", "development");
        }
        Self { previous }
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var("NVBES_ENV", previous);
            } else {
                std::env::remove_var("NVBES_ENV");
            }
        }
    }
}
