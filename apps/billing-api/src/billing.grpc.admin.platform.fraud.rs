use serde_json::json;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::billing::v1::AdminBillingPlatformActionResult;

pub async fn review_fraud_assessment(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
    assessment_id: Uuid,
    next_status: &'static str,
    reason: String,
) -> Result<AdminBillingPlatformActionResult, Status> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        WITH previous AS (
          SELECT id, review_status AS previous_state
          FROM billing_fraud_assessments
          WHERE id = $1 AND workspace_id = $2 AND review_status <> $3
        ),
        updated AS (
          UPDATE billing_fraud_assessments bfa
          SET review_status = $3,
            reviewed_at = NOW(),
            reviewed_by_principal_id = $4,
            review_reason = $5
          FROM previous
          WHERE bfa.id = previous.id
          RETURNING bfa.review_status AS next_state
        )
        SELECT previous.previous_state, updated.next_state
        FROM previous
        JOIN updated ON TRUE
        "#,
    )
    .bind(assessment_id)
    .bind(workspace_id)
    .bind(next_status)
    .bind(actor_principal_id)
    .bind(reason)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "billing_fraud_assessment_not_reviewable: Fraud assessment is missing or already has the requested review status.",
        )
    })?;

    Ok(AdminBillingPlatformActionResult {
        object_id: assessment_id.to_string(),
        action_kind: fraud_review_action_kind(next_status).to_string(),
        status: row.1,
        audit_action: fraud_review_audit_action(next_status).to_string(),
        target_type: "billing_fraud_assessment".to_string(),
        previous_state: row.0,
        metadata_json: json!({ "billing_fraud_assessment_id": assessment_id }).to_string(),
    })
}

fn fraud_review_action_kind(next_status: &str) -> &'static str {
    match next_status {
        "approved" => "approve_fraud_assessment",
        "trusted" => "trust_fraud_assessment",
        _ => "reject_fraud_assessment",
    }
}

fn fraud_review_audit_action(next_status: &str) -> &'static str {
    match next_status {
        "approved" => "billing_platform.fraud_review.approved",
        "trusted" => "billing_platform.fraud_review.trusted",
        _ => "billing_platform.fraud_review.rejected",
    }
}

#[cfg(test)]
mod tests {
    use super::{fraud_review_action_kind, fraud_review_audit_action};

    #[test]
    fn fraud_review_action_contract_matches_status() {
        assert_eq!(
            fraud_review_action_kind("approved"),
            "approve_fraud_assessment"
        );
        assert_eq!(
            fraud_review_audit_action("trusted"),
            "billing_platform.fraud_review.trusted"
        );
        assert_eq!(
            fraud_review_action_kind("rejected"),
            "reject_fraud_assessment"
        );
    }
}

