use nvbes_billing::dunning::{AccessPolicyState, policy_after_payment_failure};
use uuid::Uuid;

use crate::http::error::AppError;

pub fn next_access_policy_after_failure(previous_failures: u32) -> AccessPolicyState {
    policy_after_payment_failure(previous_failures)
}

pub fn access_policy_state_code(state: AccessPolicyState) -> &'static str {
    match state {
        AccessPolicyState::Active => "active",
        AccessPolicyState::Warning => "warning",
        AccessPolicyState::Grace => "grace",
        AccessPolicyState::Degraded => "degraded",
        AccessPolicyState::Suspended => "suspended",
        AccessPolicyState::Canceled => "canceled",
    }
}

pub fn access_policy_for_provider_attempt(attempt_count: i64) -> AccessPolicyState {
    let previous_failures = attempt_count.saturating_sub(1).min(i64::from(u32::MAX)) as u32;
    next_access_policy_after_failure(previous_failures)
}

pub async fn record_payment_failure_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    attempt_count: i64,
) -> Result<Uuid, AppError> {
    let tenant_id = tenant_id_for_workspace(tx, workspace_id).await?;
    let policy_state = access_policy_state_code(access_policy_for_provider_attempt(attempt_count));
    let case_id = open_dunning_case(tx, tenant_id, policy_state).await?;

    sqlx::query(
        r#"
        INSERT INTO billing_dunning_attempts (
          dunning_case_id, attempt_type, status, scheduled_at, completed_at
        )
        VALUES ($1, 'provider_payment_retry', 'failed', NOW(), NOW())
        "#,
    )
    .bind(case_id)
    .execute(tx.as_mut())
    .await?;

    insert_access_policy_snapshot(
        tx,
        tenant_id,
        workspace_id,
        policy_state,
        "provider_payment_failed",
    )
    .await?;

    Ok(case_id)
}

pub async fn record_payment_success_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let tenant_id = tenant_id_for_workspace(tx, workspace_id).await?;
    sqlx::query(
        r#"
        UPDATE billing_dunning_cases
        SET status = 'closed',
            policy_state = 'active',
            closed_at = COALESCE(closed_at, NOW()),
            updated_at = NOW()
        WHERE tenant_id = $1
          AND status = 'open'
        "#,
    )
    .bind(tenant_id)
    .execute(tx.as_mut())
    .await?;

    insert_access_policy_snapshot(
        tx,
        tenant_id,
        workspace_id,
        "active",
        "provider_payment_succeeded",
    )
    .await?;

    Ok(())
}

async fn tenant_id_for_workspace(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>("SELECT tenant_id FROM workspaces WHERE id = $1")
        .bind(workspace_id)
        .fetch_optional(tx.as_mut())
        .await?
        .ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))
}

async fn open_dunning_case(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    policy_state: &str,
) -> Result<Uuid, AppError> {
    if let Some(case_id) = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM billing_dunning_cases
        WHERE tenant_id = $1
          AND status = 'open'
        ORDER BY opened_at DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    {
        sqlx::query(
            r#"
            UPDATE billing_dunning_cases
            SET policy_state = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(case_id)
        .bind(policy_state)
        .execute(tx.as_mut())
        .await?;
        return Ok(case_id);
    }

    let case_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO billing_dunning_cases (tenant_id, status, policy_state)
        VALUES ($1, 'open', $2)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(policy_state)
    .fetch_one(tx.as_mut())
    .await?;
    Ok(case_id)
}

async fn insert_access_policy_snapshot(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    policy_state: &str,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO billing_access_policy_snapshots (
          tenant_id, workspace_id, policy_state, reason, effective_from
        )
        VALUES ($1, $2, $3, $4, NOW())
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(policy_state)
    .bind(reason)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_attempt_count_maps_to_dunning_policy() {
        assert_eq!(
            access_policy_for_provider_attempt(1),
            AccessPolicyState::Warning
        );
        assert_eq!(
            access_policy_for_provider_attempt(3),
            AccessPolicyState::Grace
        );
        assert_eq!(
            access_policy_for_provider_attempt(4),
            AccessPolicyState::Degraded
        );
        assert_eq!(
            access_policy_for_provider_attempt(6),
            AccessPolicyState::Suspended
        );
    }

    #[test]
    fn policy_state_codes_match_schema_values() {
        assert_eq!(
            access_policy_state_code(AccessPolicyState::Active),
            "active"
        );
        assert_eq!(
            access_policy_state_code(AccessPolicyState::Warning),
            "warning"
        );
        assert_eq!(access_policy_state_code(AccessPolicyState::Grace), "grace");
        assert_eq!(
            access_policy_state_code(AccessPolicyState::Degraded),
            "degraded"
        );
        assert_eq!(
            access_policy_state_code(AccessPolicyState::Suspended),
            "suspended"
        );
    }
}
