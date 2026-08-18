use nvbes_core::config::AppConfig;
use nvbes_observability::{
    capture_error_reporting_smoke, init_error_reporting_for_service, init_tracing,
    install_safe_panic_hook, start_continuous_profiling,
};
use tracing::info;

#[path = "billing.worker.rs"]
mod worker;

const BILLING_WORKER_METRICS_BIND_ADDR_ENV: &str = "NVBES_BILLING_WORKER_METRICS_BIND_ADDR";
const DEFAULT_BILLING_WORKER_METRICS_BIND_ADDR: &str = "127.0.0.1:4104";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    config
        .resolve_from_secret_manager()
        .await
        .map_err(anyhow::Error::msg)?;

    let _error_reporting_guard = init_error_reporting_for_service(&config, "billing-worker");
    install_safe_panic_hook();
    init_tracing(&config);

    let arg1 = std::env::args().nth(1);

    if matches!(arg1.as_deref(), Some("error-reporting-smoke")) {
        let result = capture_error_reporting_smoke(
            "billing-worker",
            &config.environment,
            "worker",
            config.sentry_dsn.is_some(),
        );
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    let _profiling_guard =
        start_continuous_profiling(&config, "billing-worker").map_err(anyhow::Error::msg)?;

    let db = nvbes_billing_service::database::connect_pool(&config).await?;
    nvbes_billing_service::database::run_migrations(&db).await?;
    let redis = nvbes_core::redis_runtime::require_redis_pool(&config).await?;
    let email = nvbes_email::EmailClient::connect(nvbes_email::EmailClientConfig::from_env(
        &config.environment,
    )?)
    .await?;
    let product_analytics = build_product_analytics(&config)?;
    let state =
        worker::BillingWorkerState::new(config.clone(), db, redis, email, product_analytics);

    if matches!(arg1.as_deref(), Some("run-billing-jobs")) {
        tracing::info!("running billing-worker in Serverless Job mode: run-billing-jobs");
        return worker::run_billing_jobs_once(&state).await;
    }

    let metrics_bind_addr = billing_worker_metrics_bind_addr();
    let _metrics_server = nvbes_observability::start_metrics_server(
        &config,
        state.observability.clone(),
        &metrics_bind_addr,
    )
    .await?;

    tracing::info!(
        app = %config.app_name,
        environment = %config.environment,
        "starting nvbes Billing worker"
    );

    worker::run_loop_until_shutdown(state, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

fn billing_worker_metrics_bind_addr() -> String {
    std::env::var(BILLING_WORKER_METRICS_BIND_ADDR_ENV)
        .unwrap_or_else(|_| DEFAULT_BILLING_WORKER_METRICS_BIND_ADDR.to_string())
}

fn build_product_analytics(
    config: &AppConfig,
) -> anyhow::Result<nvbes_product_analytics::ProductAnalytics> {
    let analytics_config = nvbes_product_analytics::ProductAnalyticsConfig {
        enabled: config.product_analytics_enabled,
        analytics_id_salt: config.analytics_id_salt.clone(),
    };

    if !config.product_analytics_enabled {
        info!("Billing product analytics disabled");
        return Ok(nvbes_product_analytics::ProductAnalytics::disabled());
    }

    let sink = nvbes_analytics_posthog::PostHogAnalyticsSink::new(
        nvbes_analytics_posthog::PostHogAnalyticsConfig {
            host: config.posthog_host.clone(),
            project_token: config.product_analytics_token.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "NVBES_PRODUCT_ANALYTICS_TOKEN is required when product analytics is enabled"
                )
            })?,
        },
    )?;
    info!("Billing product analytics enabled with PostHog");

    Ok(nvbes_product_analytics::ProductAnalytics::with_sink(
        analytics_config,
        std::sync::Arc::new(sink),
    )?)
}
