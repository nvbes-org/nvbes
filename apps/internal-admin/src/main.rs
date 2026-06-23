#[path = "internal_admin.access_center.rs"]
mod access_center;
#[path = "internal_admin.app.rs"]
mod app;
#[path = "internal_admin.audit.rs"]
mod audit;
#[path = "internal_admin.audit_evidence_center.rs"]
mod audit_evidence_center;
#[path = "internal_admin.billing.admin.rs"]
mod billing_admin;
#[path = "internal_admin.billing.admin.access.rs"]
mod billing_admin_access;
#[path = "internal_admin.billing.admin.exports.rs"]
mod billing_admin_exports;
#[path = "internal_admin.billing.admin.mutations.rs"]
mod billing_admin_mutations;
#[path = "internal_admin.billing.admin.overview.rs"]
mod billing_admin_overview;
#[path = "internal_admin.billing.admin.provider_events.rs"]
mod billing_admin_provider_events;
#[path = "internal_admin.billing.admin.search.rs"]
mod billing_admin_search;
#[path = "internal_admin.billing.admin.types.rs"]
mod billing_admin_types;
#[path = "internal_admin.billing.admin.validation.rs"]
mod billing_admin_validation;
#[path = "internal_admin.billing.runbooks.rs"]
mod billing_runbooks;
#[path = "internal_admin.command_center.rs"]
mod command_center;
#[path = "internal_admin.communications_center.rs"]
mod communications_center;
#[path = "internal_admin.compliance_center.rs"]
mod compliance_center;
#[path = "internal_admin.customer_center.rs"]
mod customer_center;
#[path = "internal_admin.developer_center.rs"]
mod developer_center;
#[path = "internal_admin.http.error.rs"]
mod error;
#[path = "internal_admin.global_search.rs"]
mod global_search;
#[path = "internal_admin.identity_governance_center.rs"]
mod identity_governance_center;
#[path = "internal_admin.operations_center.rs"]
mod operations_center;
#[path = "internal_admin.region_center.rs"]
mod region_center;
#[path = "internal_admin.revenue_center.rs"]
mod revenue_center;
#[path = "internal_admin.http.routes.rs"]
mod routes;
#[path = "internal_admin.security_center.rs"]
mod security_center;
#[path = "internal_admin.tenants.rs"]
mod tenants;
#[path = "internal_admin.users.rs"]
mod users;
#[path = "internal_admin.workspaces.rs"]
mod workspaces;

use std::net::SocketAddr;

use nvbes_core::config::AppConfig;
use nvbes_core::http::keep_alive;
use nvbes_observability::{
    init_error_reporting, init_tracing, install_safe_panic_hook, start_continuous_profiling,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    let _error_reporting_guard = init_error_reporting(&config);
    install_safe_panic_hook();
    init_tracing(&config);
    let _profiling_guard =
        start_continuous_profiling(&config, "internal-admin").map_err(anyhow::Error::msg)?;

    let db = nvbes_core::postgres_runtime::connect_pool(&config).await?;
    let app = app::build_router(app::AppState::new(config.clone(), db));
    let http_address = SocketAddr::from(([0, 0, 0, 0], config.api_port));
    let http_listener = keep_alive::bind_listener_with_keepalive(http_address, 4096)?;

    tracing::info!(
        app = %config.app_name,
        environment = %config.environment,
        address = %http_address,
        "starting nvbes Internal Admin"
    );

    axum::serve(
        http_listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;

    Ok(())
}
