#[path = "internal_admin.access_center.rs"]
mod access_center;
#[path = "internal_admin.access_center.actions.rs"]
mod access_center_actions;
#[cfg(test)]
#[path = "internal_admin.access_center.actions.tests.rs"]
mod access_center_actions_tests;
#[path = "internal_admin.app.rs"]
mod app;
#[path = "internal_admin.audit.rs"]
mod audit;
#[path = "internal_admin.audit_evidence_center.rs"]
mod audit_evidence_center;
#[path = "internal_admin.backoffice_authorization.rs"]
mod backoffice_authorization;
#[path = "internal_admin.backoffice_dual_control.rs"]
mod backoffice_dual_control;
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
#[path = "internal_admin.billing_platform_center.rs"]
mod billing_platform_center;
#[path = "internal_admin.billing_platform_center.action_log.rs"]
mod billing_platform_center_action_log;
#[path = "internal_admin.billing_platform_center.actions.rs"]
mod billing_platform_center_actions;
#[cfg(test)]
#[path = "internal_admin.billing_platform_center.actions.tests.rs"]
mod billing_platform_center_actions_tests;
#[path = "internal_admin.billing_platform_center.mutations.rs"]
mod billing_platform_center_mutations;
#[path = "internal_admin.billing_platform_center.types.rs"]
mod billing_platform_center_types;
#[path = "internal_admin.billing_platform_center.validation.rs"]
mod billing_platform_center_validation;
#[path = "internal_admin.billing.runbooks.rs"]
mod billing_runbooks;
#[path = "internal_admin.command_center.rs"]
mod command_center;
#[path = "internal_admin.communications_center.rs"]
mod communications_center;
#[path = "internal_admin.communications_center.actions.rs"]
mod communications_center_actions;
#[cfg(test)]
#[path = "internal_admin.communications_center.actions.tests.rs"]
mod communications_center_actions_tests;
#[path = "internal_admin.communications_center.mutations.rs"]
mod communications_center_mutations;
#[path = "internal_admin.communications_center.types.rs"]
mod communications_center_types;
#[path = "internal_admin.communications_center.validation.rs"]
mod communications_center_validation;
#[path = "internal_admin.compliance_center.rs"]
mod compliance_center;
#[path = "internal_admin.compliance_center.actions.rs"]
mod compliance_center_actions;
#[cfg(test)]
#[path = "internal_admin.compliance_center.actions.tests.rs"]
mod compliance_center_actions_tests;
#[path = "internal_admin.compliance_center.mutations.rs"]
mod compliance_center_mutations;
#[path = "internal_admin.compliance_center.types.rs"]
mod compliance_center_types;
#[path = "internal_admin.compliance_center.validation.rs"]
mod compliance_center_validation;
#[path = "internal_admin.customer_center.rs"]
mod customer_center;
#[path = "internal_admin.developer_center.rs"]
mod developer_center;
#[path = "internal_admin.developer_center.actions.rs"]
mod developer_center_actions;
#[cfg(test)]
#[path = "internal_admin.developer_center.actions.tests.rs"]
mod developer_center_actions_tests;
#[path = "internal_admin.developer_center.mutations.rs"]
mod developer_center_mutations;
#[path = "internal_admin.developer_center.types.rs"]
mod developer_center_types;
#[path = "internal_admin.developer_center.validation.rs"]
mod developer_center_validation;
#[path = "internal_admin.entitlements_center.rs"]
mod entitlements_center;
#[path = "internal_admin.entitlements_center.actions.rs"]
mod entitlements_center_actions;
#[cfg(test)]
#[path = "internal_admin.entitlements_center.actions.tests.rs"]
mod entitlements_center_actions_tests;
#[path = "internal_admin.entitlements_center.mutations.rs"]
mod entitlements_center_mutations;
#[path = "internal_admin.http.error.rs"]
mod error;
#[path = "internal_admin.global_search.rs"]
mod global_search;
#[path = "internal_admin.idempotency.rs"]
mod idempotency;
#[path = "internal_admin.identity_governance_center.rs"]
mod identity_governance_center;
#[path = "internal_admin.identity_governance_center.actions.rs"]
mod identity_governance_center_actions;
#[cfg(test)]
#[path = "internal_admin.identity_governance_center.actions.tests.rs"]
mod identity_governance_center_actions_tests;
#[path = "internal_admin.operations_center.rs"]
mod operations_center;
#[path = "internal_admin.operations_center.actions.rs"]
mod operations_center_actions;
#[cfg(test)]
#[path = "internal_admin.operations_center.actions.tests.rs"]
mod operations_center_actions_tests;
#[path = "internal_admin.operations_center.mutations.rs"]
mod operations_center_mutations;
#[path = "internal_admin.operations_center.types.rs"]
mod operations_center_types;
#[path = "internal_admin.operations_center.validation.rs"]
mod operations_center_validation;
#[path = "internal_admin.rate_limit.rs"]
mod rate_limit;
#[path = "internal_admin.region_center.rs"]
mod region_center;
#[path = "internal_admin.region_center.actions.rs"]
mod region_center_actions;
#[cfg(test)]
#[path = "internal_admin.region_center.actions.tests.rs"]
mod region_center_actions_tests;
#[path = "internal_admin.region_center.mutations.rs"]
mod region_center_mutations;
#[path = "internal_admin.region_center.types.rs"]
mod region_center_types;
#[path = "internal_admin.region_center.validation.rs"]
mod region_center_validation;
#[path = "internal_admin.revenue_center.rs"]
mod revenue_center;
#[path = "internal_admin.revenue_center.actions.rs"]
mod revenue_center_actions;
#[cfg(test)]
#[path = "internal_admin.revenue_center.actions.tests.rs"]
mod revenue_center_actions_tests;
#[path = "internal_admin.revenue_center.mutations.rs"]
mod revenue_center_mutations;
#[path = "internal_admin.revenue_center.recent.rs"]
mod revenue_center_recent;
#[path = "internal_admin.revenue_center.types.rs"]
mod revenue_center_types;
#[path = "internal_admin.revenue_center.validation.rs"]
mod revenue_center_validation;
#[path = "internal_admin.risk_decision_center.rs"]
mod risk_decision_center;
#[path = "internal_admin.risk_decision_center.actions.rs"]
mod risk_decision_center_actions;
#[cfg(test)]
#[path = "internal_admin.risk_decision_center.actions.tests.rs"]
mod risk_decision_center_actions_tests;
#[path = "internal_admin.risk_decision_center.mutations.rs"]
mod risk_decision_center_mutations;
#[path = "internal_admin.risk_decision_center.types.rs"]
mod risk_decision_center_types;
#[path = "internal_admin.risk_decision_center.validation.rs"]
mod risk_decision_center_validation;
#[path = "internal_admin.http.routes.rs"]
mod routes;
#[path = "internal_admin.security_center.rs"]
mod security_center;
#[path = "internal_admin.security_center.actions.rs"]
mod security_center_actions;
#[path = "internal_admin.tenants.rs"]
mod tenants;
#[path = "internal_admin.usage_center.rs"]
mod usage_center;
#[path = "internal_admin.usage_center.actions.rs"]
mod usage_center_actions;
#[cfg(test)]
#[path = "internal_admin.usage_center.actions.tests.rs"]
mod usage_center_actions_tests;
#[path = "internal_admin.usage_center.mutations.rs"]
mod usage_center_mutations;
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
