use serde_json::json;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::billing::v1::AdminBillingPlatformActionResult;

pub async fn transition_kyc_profile(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    profile_id: Uuid,
    next_state: &'static str,
    reason: String,
) -> Result<AdminBillingPlatformActionResult, Status> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        WITH previous AS (
          SELECT id, review_status AS previous_state
          FROM billing_kyc_profiles
          WHERE id = $1 AND tenant_id = $2 AND review_status <> $3
        ),
        updated AS (
          UPDATE billing_kyc_profiles bkp
          SET review_status = $3, reviewed_at = NOW(), reviewed_by_principal_id = $4,
            review_reason = $5, updated_at = NOW()
          FROM previous
          WHERE bkp.id = previous.id
          RETURNING bkp.review_status AS next_state
        )
        SELECT previous.previous_state, updated.next_state
        FROM previous
        JOIN updated ON TRUE
        "#,
    )
    .bind(profile_id)
    .bind(tenant_id)
    .bind(next_state)
    .bind(actor_principal_id)
    .bind(reason)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "kyc_profile_not_reviewable: KYC profile is missing, belongs to another tenant, or already has the requested status.",
        )
    })?;
    let (action_kind, audit_action) = kyc_action_names(next_state);

    Ok(AdminBillingPlatformActionResult {
        object_id: profile_id.to_string(),
        action_kind: action_kind.to_string(),
        status: row.1,
        audit_action: audit_action.to_string(),
        target_type: "billing_kyc_profile".to_string(),
        previous_state: row.0,
        metadata_json: json!({ "kyc_profile_id": profile_id }).to_string(),
    })
}

pub async fn activate_einvoicing_profile(
    db: &sqlx::PgPool,
    profile_id: Uuid,
) -> Result<AdminBillingPlatformActionResult, Status> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        WITH previous AS (
          SELECT id, status AS previous_state
          FROM billing_einvoicing_profiles
          WHERE id = $1 AND status <> 'active'
        ),
        updated AS (
          UPDATE billing_einvoicing_profiles bep
          SET status = 'active', updated_at = NOW()
          FROM previous
          WHERE bep.id = previous.id
          RETURNING bep.status AS next_state
        )
        SELECT previous.previous_state, updated.next_state
        FROM previous
        JOIN updated ON TRUE
        "#,
    )
    .bind(profile_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "einvoicing_profile_not_activatable: E-invoicing profile is missing or already active.",
        )
    })?;

    Ok(AdminBillingPlatformActionResult {
        object_id: profile_id.to_string(),
        action_kind: "activate_einvoicing_profile".to_string(),
        status: row.1,
        audit_action: "billing_platform.einvoicing_profile.activated".to_string(),
        target_type: "billing_einvoicing_profile".to_string(),
        previous_state: row.0,
        metadata_json: json!({ "einvoicing_profile_id": profile_id }).to_string(),
    })
}

fn kyc_action_names(next_state: &'static str) -> (&'static str, &'static str) {
    if next_state == "approved" {
        ("approve_kyc_profile", "billing_platform.kyc.approved")
    } else {
        ("reject_kyc_profile", "billing_platform.kyc.rejected")
    }
}

