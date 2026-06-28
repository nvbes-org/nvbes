use nvbes_billing::validate_plan_code;
use nvbes_region::geo::{GeoLookupPurpose, GeoLookupRecordContext, record_geo_resolution_tx};
use sqlx::PgPool;

use super::db::{
    fetch_active_price_mapping_tx, fetch_billing_state_tx, fetch_plan_by_code_tx,
    upsert_billing_customer_tx,
};
use super::manage_geo::resolve_checkout_geo;
use super::manage_redirect_urls::resolve_billing_redirect_url;
use super::stripe::{
    create_stripe_checkout_session, create_stripe_customer, create_stripe_portal_session,
};
use super::types::*;
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use nvbes_core::config::AppConfig;

pub async fn create_checkout_session(
    db: &PgPool,
    config: &AppConfig,
    access: &WorkspaceAccess,
    input: CreateCheckoutInput,
    ip: Option<String>,
    trusted_country_header: Option<String>,
    user_agent: Option<String>,
) -> Result<CheckoutSessionResponse, AppError> {
    let target_plan_code = validate_plan_code(&input.plan_code)
        .ok_or_else(|| AppError::bad_request("invalid_plan", "Unsupported billing plan."))?;
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let record = fetch_billing_state_tx(&mut tx, access.workspace_id).await?;
    let geo_resolution = resolve_checkout_geo(
        &mut tx,
        config,
        ip.as_deref(),
        trusted_country_header.as_deref(),
        record.country.as_deref(),
    )
    .await?;
    let checkout_country = geo_resolution
        .location
        .as_ref()
        .map(|location| location.country_code.as_str());

    record_geo_resolution_tx(
        &mut tx,
        GeoLookupRecordContext {
            purpose: GeoLookupPurpose::Payment,
            subject_type: Some("workspace"),
            subject_id: Some(access.workspace_id),
            request_id: None,
        },
        &geo_resolution,
    )
    .await?;

    let target_plan = fetch_plan_by_code_tx(&mut tx, &target_plan_code).await?;

    if target_plan.code == "trial" {
        return Err(AppError::bad_request(
            "invalid_plan",
            "Checkout cannot be created for the trial plan.",
        ));
    }

    let mapping =
        fetch_active_price_mapping_tx(&mut tx, target_plan.plan_id, checkout_country).await?;
    let customer_id = match record
        .stripe_customer_id
        .clone()
        .or_else(|| record.billing_customer_id.clone())
    {
        Some(customer_id) => customer_id,
        None => {
            let customer = create_stripe_customer(config, &record).await?;
            upsert_billing_customer_tx(&mut tx, access.workspace_id, &customer.id).await?;
            customer.id
        }
    };

    let success_url = resolve_billing_redirect_url(
        input.success_url.as_deref(),
        &config.billing_default_success_url,
        &config.web_base_url,
        config.staging_web_base_url.as_deref(),
        "success_url",
        "NVBES_BILLING_SUCCESS_URL",
    )?;
    let cancel_url = resolve_billing_redirect_url(
        input.cancel_url.as_deref(),
        &config.billing_default_cancel_url,
        &config.web_base_url,
        config.staging_web_base_url.as_deref(),
        "cancel_url",
        "NVBES_BILLING_CANCEL_URL",
    )?;

    let session = create_stripe_checkout_session(
        config,
        &customer_id,
        record.owner_principal_id,
        access.workspace_id,
        &target_plan.code,
        &mapping.stripe_price_id,
        &success_url,
        &cancel_url,
    )
    .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: access.workspace_id,
            actor_user_id: Some(access.auth.user_id),
            actor_principal_id: Some(access.auth.principal_id),
            action: "billing.checkout_started",
            target_type: "workspace",
            target_id: Some(access.workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "plan_code": target_plan.code,
                "stripe_customer_id": customer_id,
                "stripe_price_id": mapping.stripe_price_id,
                "stripe_product_id": mapping.stripe_product_id,
                "geo_country_code": checkout_country,
                "geo_source": geo_resolution.source.as_str(),
                "geo_confidence": geo_resolution.confidence.as_str(),
                "geo_network_kind": geo_resolution.network_kind.as_str(),
                "geo_risk_score": geo_resolution.risk_score,
                "geo_risk_labels": geo_resolution.risk_labels.clone(),
                "price_country_code": mapping.country_code,
                "pricing_region": mapping.pricing_region,
                "checkout_session_id": session.id,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(CheckoutSessionResponse {
        provider: "stripe".to_string(),
        session_id: session.id.clone(),
        checkout_id: session.id,
        url: session.url,
        provider_customer_id: customer_id.clone(),
        provider_price_id: Some(mapping.stripe_price_id.clone()),
        payment_id: None,
        stripe_customer_id: customer_id,
        stripe_price_id: mapping.stripe_price_id,
    })
}

pub async fn create_portal_session(
    config: &AppConfig,
    db: &PgPool,
    access: &WorkspaceAccess,
    input: CreatePortalInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<PortalSessionResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let record = fetch_billing_state_tx(&mut tx, access.workspace_id).await?;
    let customer_id = record
        .stripe_customer_id
        .clone()
        .or_else(|| record.billing_customer_id.clone())
        .ok_or_else(|| {
            AppError::conflict(
                "missing_billing_customer",
                "Create a checkout session before opening the billing portal.",
            )
        })?;
    let return_url = resolve_billing_redirect_url(
        input.return_url.as_deref(),
        &config.billing_default_portal_return_url,
        &config.web_base_url,
        config.staging_web_base_url.as_deref(),
        "return_url",
        "NVBES_BILLING_PORTAL_RETURN_URL",
    )?;

    let session = create_stripe_portal_session(config, &customer_id, &return_url).await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: access.workspace_id,
            actor_user_id: Some(access.auth.user_id),
            actor_principal_id: Some(access.auth.principal_id),
            action: "billing.portal_opened",
            target_type: "workspace",
            target_id: Some(access.workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "stripe_customer_id": customer_id,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(PortalSessionResponse {
        provider: "stripe".to_string(),
        url: session.url,
        stripe_customer_id: customer_id,
    })
}
