use chrono::{DateTime, Duration, Utc};
use sqlx::{Postgres, Transaction};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise, privileged_authentication, service_status::sql_status,
};

const DEFAULT_ELEVATION_MINUTES: i32 = 15;
const MAX_ELEVATION_MINUTES: i32 = 60;

pub async fn authorize_admin_elevation(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: enterprise::AuthorizeAdminElevationRequest,
) -> Result<enterprise::AdminElevationAuthorization, Status> {
    let stored_role = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM tenant_memberships
        WHERE tenant_id = $1
          AND principal_id = $2
          AND status = 'active'
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::permission_denied("active tenant membership is required"))?;
    if stored_role != request.base_role.trim() {
        return Err(Status::permission_denied(
            "base_role does not match the active tenant membership",
        ));
    }
    let reason = non_empty(&request.reason, "reason")?;
    let mut authorization = authorize_admin_elevation_decision(tenant_id, &request)?;
    let grant_id =
        record_admin_elevation(db, tenant_id, actor_id, &request, &reason, &authorization).await?;
    authorization.grant_id = grant_id.to_string();
    authorization.status = "active".to_string();
    Ok(authorization)
}

pub fn authorize_admin_elevation_decision(
    tenant_id: Uuid,
    request: &enterprise::AuthorizeAdminElevationRequest,
) -> Result<enterprise::AdminElevationAuthorization, Status> {
    privileged_authentication::require(request.authentication.as_ref())?;
    let base_role = request.base_role.trim();
    if base_role != "owner" && base_role != "admin" {
        return Err(Status::permission_denied(
            "an existing owner or admin role is required for admin elevation",
        ));
    }

    let step_up_expires_at = parse_time(&request.step_up_expires_at, "step_up_expires_at")?;
    let session_expires_at = parse_time(&request.session_expires_at, "session_expires_at")?;
    let duration_minutes = requested_duration_minutes(request.duration_minutes);
    let requested_expires_at = Utc::now() + Duration::minutes(i64::from(duration_minutes));
    let expires_at = requested_expires_at
        .min(step_up_expires_at)
        .min(session_expires_at);
    if expires_at <= Utc::now() {
        return Err(Status::failed_precondition(
            "admin elevation requires an active step-up and session",
        ));
    }

    Ok(enterprise::AdminElevationAuthorization {
        tenant_id: tenant_id.to_string(),
        role: "admin".to_string(),
        expires_at: expires_at.to_rfc3339(),
        step_up_expires_at: step_up_expires_at.to_rfc3339(),
        grant_id: String::new(),
        status: "authorized".to_string(),
    })
}

async fn record_admin_elevation(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: &enterprise::AuthorizeAdminElevationRequest,
    reason: &str,
    authorization: &enterprise::AdminElevationAuthorization,
) -> Result<Uuid, Status> {
    let mut tx = db.begin().await.map_err(sql_status)?;
    let expires_at = parse_time(&authorization.expires_at, "expires_at")?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("{tenant_id}:{actor_id}:admin"))
        .execute(&mut *tx)
        .await
        .map_err(sql_status)?;
    sqlx::query(
        r#"
        UPDATE privileged_access_grants
        SET status = 'expired'
        WHERE tenant_id = $1
          AND principal_id = $2
          AND role = 'admin'
          AND status = 'active'
          AND expires_at <= NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .execute(&mut *tx)
    .await
    .map_err(sql_status)?;
    let grant_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO privileged_access_grants (
          tenant_id, principal_id, granted_by, role, reason, break_glass, expires_at
        )
        VALUES ($1, $2, $2, 'admin', $3, $4, $5)
        ON CONFLICT (tenant_id, principal_id, role)
          WHERE status = 'active' AND revoked_at IS NULL
        DO UPDATE SET
          reason = EXCLUDED.reason,
          break_glass = EXCLUDED.break_glass,
          expires_at = LEAST(EXCLUDED.expires_at, privileged_access_grants.expires_at)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(reason)
    .bind(request.break_glass)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(sql_status)?;
    if request.break_glass {
        let reason = non_empty(&request.break_glass_reason, "break_glass_reason")?;
        let procedure_reference = non_empty(
            &request.break_glass_procedure_reference,
            "break_glass_procedure_reference",
        )?;
        touch_break_glass_account(&mut tx, tenant_id, actor_id).await?;
        insert_audit(
            &mut tx,
            tenant_id,
            actor_id,
            "enterprise.break_glass.used",
            serde_json::json!({
                "reason": reason,
                "procedure_reference": procedure_reference,
                "authentication": authentication_evidence(request)
            }),
        )
        .await?;
    }
    insert_audit(
        &mut tx,
        tenant_id,
        actor_id,
        "enterprise.admin_elevation.granted",
        serde_json::json!({
            "break_glass": request.break_glass,
            "authentication": authentication_evidence(request)
        }),
    )
    .await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(grant_id)
}

fn authentication_evidence(
    request: &enterprise::AuthorizeAdminElevationRequest,
) -> serde_json::Value {
    let authentication = request
        .authentication
        .as_ref()
        .expect("privileged authentication was validated before persistence");
    serde_json::json!({
        "acr": authentication.acr,
        "amr": authentication.amr,
        "auth_time": authentication.auth_time,
        "authentication_event_id": authentication.authentication_event_id
    })
}

pub async fn revoke_admin_elevation(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    grant_id: Uuid,
    reason: &str,
) -> Result<enterprise::AdminElevationAuthorization, Status> {
    let reason = non_empty(reason, "reason")?;
    let mut tx = db.begin().await.map_err(sql_status)?;
    let row = sqlx::query(
        r#"
        UPDATE privileged_access_grants
        SET status = 'revoked',
            revoked_at = NOW(),
            revoked_by = $3,
            revocation_reason = $4
        WHERE id = $1
          AND tenant_id = $2
          AND status = 'active'
          AND revoked_at IS NULL
        RETURNING id, tenant_id, role::text AS role, expires_at
        "#,
    )
    .bind(grant_id)
    .bind(tenant_id)
    .bind(actor_id)
    .bind(&reason)
    .fetch_optional(&mut *tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("active admin elevation was not found"))?;
    insert_audit(
        &mut tx,
        tenant_id,
        actor_id,
        "enterprise.admin_elevation.revoked",
        serde_json::json!({"grant_id": grant_id, "reason": reason}),
    )
    .await?;
    tx.commit().await.map_err(sql_status)?;

    use sqlx::Row;
    Ok(enterprise::AdminElevationAuthorization {
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        role: row.get("role"),
        expires_at: row.get::<DateTime<Utc>, _>("expires_at").to_rfc3339(),
        step_up_expires_at: String::new(),
        grant_id: row.get::<Uuid, _>("id").to_string(),
        status: "revoked".to_string(),
    })
}

async fn touch_break_glass_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    actor_id: Uuid,
) -> Result<(), Status> {
    let rows = sqlx::query(
        r#"
        UPDATE tenant_break_glass_accounts
        SET last_used_at = NOW(), updated_at = NOW()
        WHERE tenant_id = $1 AND principal_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?
    .rows_affected();
    if rows == 0 {
        return Err(Status::failed_precondition(
            "active break-glass account was not found",
        ));
    }
    Ok(())
}

async fn insert_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    actor_id: Uuid,
    action: &'static str,
    metadata: serde_json::Value,
) -> Result<(), Status> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id, actor_principal_id, action, target_type, target_id,
          metadata, event_hash, created_at
        )
        VALUES ($1, $2, $3, 'principal', $2, $4, gen_random_uuid()::text, NOW())
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(action)
    .bind(metadata)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(())
}

fn non_empty(value: &str, field: &'static str) -> Result<String, Status> {
    let value = value.trim();
    if value.is_empty() {
        Err(Status::invalid_argument(format!("{field} is required")))
    } else {
        Ok(value.to_string())
    }
}

fn requested_duration_minutes(duration_minutes: i32) -> i32 {
    if duration_minutes <= 0 {
        DEFAULT_ELEVATION_MINUTES
    } else {
        duration_minutes.clamp(1, MAX_ELEVATION_MINUTES)
    }
}

fn parse_time(value: &str, field: &'static str) -> Result<DateTime<Utc>, Status> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| Status::invalid_argument(format!("{field} must be an RFC3339 timestamp")))
}

#[cfg(test)]
#[path = "enterprise.grpc.admin_elevation.contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_expiry_never_outlives_step_up_or_session() {
        let now = Utc::now();
        let response = authorize_admin_elevation_decision(
            Uuid::new_v4(),
            &enterprise::AuthorizeAdminElevationRequest {
                context: None,
                tenant_id: Uuid::new_v4().to_string(),
                base_role: "admin".to_string(),
                duration_minutes: 60,
                step_up_expires_at: (now + Duration::minutes(10)).to_rfc3339(),
                session_expires_at: (now + Duration::minutes(30)).to_rfc3339(),
                break_glass: false,
                break_glass_reason: String::new(),
                break_glass_procedure_reference: String::new(),
                reason: "test elevation".to_string(),
                authentication: Some(enterprise::PrivilegedAuthenticationContext {
                    acr: "aal2".to_string(),
                    amr: vec!["webauthn".to_string()],
                    auth_time: now.timestamp(),
                    authentication_event_id: "unit-authn-event".to_string(),
                }),
            },
        )
        .expect("active admin with step-up should be authorized");

        let expires_at =
            DateTime::parse_from_rfc3339(&response.expires_at).expect("expiry should parse");
        assert!(expires_at <= now + Duration::minutes(10));
    }

    #[test]
    fn authorization_rejects_non_admin_base_role() {
        let now = Utc::now();
        let error = authorize_admin_elevation_decision(
            Uuid::new_v4(),
            &enterprise::AuthorizeAdminElevationRequest {
                context: None,
                tenant_id: Uuid::new_v4().to_string(),
                base_role: "member".to_string(),
                duration_minutes: 15,
                step_up_expires_at: (now + Duration::minutes(10)).to_rfc3339(),
                session_expires_at: (now + Duration::minutes(30)).to_rfc3339(),
                break_glass: false,
                break_glass_reason: String::new(),
                break_glass_procedure_reference: String::new(),
                reason: "test elevation".to_string(),
                authentication: Some(enterprise::PrivilegedAuthenticationContext {
                    acr: "aal2".to_string(),
                    amr: vec!["webauthn".to_string()],
                    auth_time: now.timestamp(),
                    authentication_event_id: "unit-authn-event".to_string(),
                }),
            },
        )
        .expect_err("members should not be authorized");

        assert_eq!(error.code(), tonic::Code::PermissionDenied);
    }
}
