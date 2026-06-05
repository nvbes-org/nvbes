use nvbes_core::config::AppConfig;
use nvbes_observability::{
    capture_sentry_smoke, init_sentry, init_tracing, install_safe_panic_hook,
};
use sqlx::postgres::PgPoolOptions;

#[path = "identity.worker.rs"]
mod worker;

pub use nvbes_identity_api::{app, domains, email, http};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().map_err(anyhow::Error::msg)?;

    let _sentry_guard = Box::leak(Box::new(init_sentry(&config)));
    install_safe_panic_hook();
    init_tracing(&config);

    if matches!(std::env::args().nth(1).as_deref(), Some("sentry-smoke")) {
        let result = capture_sentry_smoke(
            "identity-worker",
            &config.environment,
            "worker",
            config.sentry_dsn.is_some(),
        );
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await?;

    nvbes_identity_api::database::run_migrations(&db).await?;

    let state = nvbes_identity_api::app::AppState::bootstrap(&config, db).await?;
    tracing::info!(
        app = %config.app_name,
        environment = %config.environment,
        "starting nvbes Identity billing worker"
    );

    worker::run_loop_until_shutdown(state, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}
