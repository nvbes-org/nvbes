use serde_json::Value;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::psp_signals::BillingPspFraudSignal;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BillingFraudVelocity {
    pub recent_ip_checkouts: i64,
    pub recent_ip_workspaces: i64,
    pub recent_workspace_countries: i64,
    pub recent_payment_methods: i64,
    pub recent_payment_failures: i64,
    pub trusted_checkout_assessments: i64,
}

#[derive(Debug, Clone)]
pub struct BillingFraudAssessmentInput<'a> {
    pub workspace_id: Uuid,
    pub actor_principal_id: Option<Uuid>,
    pub provider: Option<&'a str>,
    pub checkout_id: Option<&'a str>,
    pub payment_id: Option<&'a str>,
    pub plan_code: &'a str,
    pub amount_minor: i64,
    pub currency: &'a str,
    pub ip_address: Option<&'a str>,
    pub geo_country_code: Option<&'a str>,
    pub billing_country_code: Option<&'a str>,
    pub network_kind: &'a str,
    pub network_risk_score: u8,
    pub network_risk_labels: &'a [String],
    pub fraud_score: u8,
    pub fraud_decision: &'a str,
    pub network_threat: &'a str,
    pub fraud_labels: &'a [String],
    pub fraud_reasons: &'a [String],
    pub enforcement_action: &'a str,
    pub metadata: Value,
}

pub async fn billing_fraud_velocity_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    ip_address: Option<&str>,
    provider: Option<&str>,
) -> Result<BillingFraudVelocity, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
          COUNT(*) FILTER (
            WHERE $2::inet IS NOT NULL
              AND ip_address = $2::inet
              AND created_at >= now() - interval '1 hour'
          )::bigint AS recent_ip_checkouts,
          COUNT(DISTINCT workspace_id) FILTER (
            WHERE $2::inet IS NOT NULL
              AND ip_address = $2::inet
              AND created_at >= now() - interval '24 hours'
          )::bigint AS recent_ip_workspaces,
          COUNT(DISTINCT geo_country_code) FILTER (
            WHERE workspace_id = $1
              AND geo_country_code IS NOT NULL
              AND created_at >= now() - interval '24 hours'
          )::bigint AS recent_workspace_countries,
          COUNT(DISTINCT metadata->>'provider_payment_method_id') FILTER (
            WHERE workspace_id = $1
              AND metadata->>'provider_payment_method_id' IS NOT NULL
              AND created_at >= now() - interval '30 days'
          )::bigint AS recent_payment_methods,
          COUNT(*) FILTER (
            WHERE workspace_id = $1
              AND metadata->>'provider_payment_status' = 'failed'
              AND created_at >= now() - interval '7 days'
          )::bigint AS recent_payment_failures,
          COUNT(*) FILTER (
            WHERE workspace_id = $1
              AND review_status = 'trusted'
              AND ($3::text IS NULL OR provider = $3)
              AND created_at >= now() - interval '180 days'
          )::bigint AS trusted_checkout_assessments
        FROM billing_fraud_assessments
        WHERE created_at >= now() - interval '180 days'
        "#,
    )
    .bind(workspace_id)
    .bind(ip_address)
    .bind(provider)
    .fetch_one(&mut **tx)
    .await?;

    Ok(BillingFraudVelocity {
        recent_ip_checkouts: row.get("recent_ip_checkouts"),
        recent_ip_workspaces: row.get("recent_ip_workspaces"),
        recent_workspace_countries: row.get("recent_workspace_countries"),
        recent_payment_methods: row.get("recent_payment_methods"),
        recent_payment_failures: row.get("recent_payment_failures"),
        trusted_checkout_assessments: row.get("trusted_checkout_assessments"),
    })
}

pub async fn insert_billing_fraud_assessment_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: BillingFraudAssessmentInput<'_>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO billing_fraud_assessments (
          workspace_id, actor_principal_id, provider, checkout_id, payment_id,
          plan_code, amount_minor, currency, ip_address, geo_country_code,
          billing_country_code, network_kind, network_risk_score, network_risk_labels,
          fraud_score, fraud_decision, network_threat, fraud_labels, fraud_reasons,
          enforcement_action, metadata
        )
        VALUES (
          $1, $2, $3, $4, $5, $6, $7, $8, $9::inet, $10, $11,
          $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
        )
        RETURNING id
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.actor_principal_id)
    .bind(input.provider)
    .bind(input.checkout_id)
    .bind(input.payment_id)
    .bind(input.plan_code)
    .bind(input.amount_minor)
    .bind(input.currency)
    .bind(input.ip_address)
    .bind(input.geo_country_code)
    .bind(input.billing_country_code)
    .bind(input.network_kind)
    .bind(i16::from(input.network_risk_score))
    .bind(input.network_risk_labels)
    .bind(i16::from(input.fraud_score))
    .bind(input.fraud_decision)
    .bind(input.network_threat)
    .bind(input.fraud_labels)
    .bind(input.fraud_reasons)
    .bind(input.enforcement_action)
    .bind(sqlx::types::Json(input.metadata))
    .fetch_one(&mut **tx)
    .await
}

pub async fn insert_billing_fraud_psp_signal_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    signal: &BillingPspFraudSignal,
) -> Result<Uuid, sqlx::Error> {
    insert_billing_fraud_assessment_tx(
        tx,
        BillingFraudAssessmentInput {
            workspace_id,
            actor_principal_id: None,
            provider: Some(signal.provider),
            checkout_id: None,
            payment_id: signal.provider_payment_id.as_deref(),
            plan_code: signal.plan_code.as_deref().unwrap_or("unknown"),
            amount_minor: signal.amount_minor.unwrap_or(0),
            currency: signal.currency.as_deref().unwrap_or("EUR"),
            ip_address: None,
            geo_country_code: None,
            billing_country_code: None,
            network_kind: "unknown",
            network_risk_score: 0,
            network_risk_labels: &[],
            fraud_score: signal.score,
            fraud_decision: signal.decision.as_str(),
            network_threat: signal.network_threat.as_str(),
            fraud_labels: &signal.labels,
            fraud_reasons: &signal.reasons,
            enforcement_action: "observe",
            metadata: signal.metadata.clone(),
        },
    )
    .await
}
