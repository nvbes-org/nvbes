use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::http::error::AppError;
use nvbes_core::auth::Aal;
use sqlx::PgPool;

use super::service::{AssuranceContext, SessionAssuranceState};

struct RequiredAssuranceInputs<'a> {
    client_id: Option<&'a str>,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
}

/// Resolve the assurance context for a given user and session.
#[expect(
    clippy::too_many_arguments,
    reason = "Assurance resolution keeps policy scope and session context explicit."
)]
pub async fn resolve_assurance_context(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Option<Uuid>,
    client_id: Option<&str>,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
) -> Result<AssuranceContext, AppError> {
    let required = resolve_required_assurance(
        db,
        user_id,
        RequiredAssuranceInputs {
            client_id,
            tenant_id,
            organization_id,
            workspace_id,
        },
    )
    .await?;

    let session_state = if let Some(session_id) = session_id {
        fetch_session_assurance_state(redis, user_id, session_id).await?
    } else {
        None
    };
    Ok(build_assurance_context(required, session_state.as_ref()))
}

/// Fetch the required ACR from client policy.
pub async fn fetch_policy_required_acr(
    db: &PgPool,
    client_id: &str,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
) -> Result<String, AppError> {
    let client = sqlx::query("SELECT id FROM oauth_clients WHERE client_id = $1 LIMIT 1")
        .bind(client_id)
        .fetch_optional(db)
        .await?;
    let Some(client) = client else {
        return Ok("aal1".to_string());
    };
    let client_uuid: Uuid = client.get("id");
    let row = sqlx::query(
        r#"
        SELECT required_acr::text AS required_acr
        FROM oauth_client_policies
        WHERE client_id = $1
          AND status = 'active'
          AND (
            ($2::uuid IS NOT NULL AND scope_type = 'workspace' AND scope_id = $2)
            OR ($3::uuid IS NOT NULL AND scope_type = 'organization' AND scope_id = $3)
            OR ($4::uuid IS NOT NULL AND scope_type = 'tenant' AND scope_id = $4)
          )
        ORDER BY
          CASE scope_type
            WHEN 'workspace' THEN 0
            WHEN 'organization' THEN 1
            WHEN 'tenant' THEN 2
          END
        LIMIT 1
        "#,
    )
    .bind(client_uuid)
    .bind(workspace_id)
    .bind(organization_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?;
    Ok(row
        .map(|row| row.get("required_acr"))
        .unwrap_or_else(|| "aal1".to_string()))
}

async fn resolve_required_assurance(
    db: &PgPool,
    user_id: Uuid,
    inputs: RequiredAssuranceInputs<'_>,
) -> Result<Aal, AppError> {
    let mut required = Aal::Aal1;

    if let Some(client_id) = inputs.client_id {
        required = std::cmp::max(
            required,
            parse_required_aal(Some(
                fetch_policy_required_acr(
                    db,
                    client_id,
                    inputs.tenant_id,
                    inputs.organization_id,
                    inputs.workspace_id,
                )
                .await?,
            )),
        );
    }

    required = std::cmp::max(
        required,
        parse_required_aal(required_acr_from_risk_tier(db, inputs.tenant_id).await?),
    );
    required = std::cmp::max(
        required,
        parse_required_aal(
            required_acr_from_workspace_role(db, user_id, inputs.workspace_id).await?,
        ),
    );

    Ok(required)
}

/// Calculate required ACR based on tenant risk tier.
pub async fn required_acr_from_risk_tier(
    db: &PgPool,
    tenant_id: Option<Uuid>,
) -> Result<Option<String>, AppError> {
    let Some(tenant_id) = tenant_id else {
        return Ok(None);
    };
    let row = sqlx::query("SELECT security_tier FROM tenants WHERE id = $1 LIMIT 1")
        .bind(tenant_id)
        .fetch_optional(db)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let tier: String = row.get("security_tier");
    let normalized = tier.trim().to_lowercase();
    let required = if normalized.contains("critical") || normalized.contains("max") {
        "aal3"
    } else if normalized.contains("strict")
        || normalized.contains("high")
        || normalized.contains("enterprise")
    {
        "aal2"
    } else {
        "aal1"
    };
    Ok(Some(required.to_string()))
}

/// Calculate required ACR based on workspace role.
pub async fn required_acr_from_workspace_role(
    db: &PgPool,
    user_id: Uuid,
    workspace_id: Option<Uuid>,
) -> Result<Option<String>, AppError> {
    let env = std::env::var("NVBES_ENV").unwrap_or_default();
    if env == "development" {
        return Ok(None);
    }
    let Some(workspace_id) = workspace_id else {
        return Ok(None);
    };
    let row = sqlx::query(
        r#"
        SELECT role::text AS role
        FROM workspace_memberships
        WHERE workspace_id = $1
          AND principal_id = $2
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let role: String = row.get("role");
    let required = if role == "owner" || role == "admin" {
        "aal2"
    } else {
        "aal1"
    };
    Ok(Some(required.to_string()))
}

/// Fetch the assurance state of a session.
pub async fn fetch_session_assurance_state(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<Option<SessionAssuranceState>, AppError> {
    let session = nvbes_redis::session::get_session(redis, &session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_read_failed", err.to_string()))?;
    Ok(session.and_then(|session| {
        if session.principal_id != user_id.to_string()
            || session.revoked_at.is_some()
            || session.expires_at <= Utc::now()
        {
            return None;
        }

        Some(SessionAssuranceState {
            created_at: session.created_at,
            step_up_verified_at: session.step_up_verified_at,
            step_up_expires_at: session.step_up_expires_at,
            acr: session.acr,
            amr: session.amr,
        })
    }))
}

fn build_assurance_context(
    required: Aal,
    session_state: Option<&SessionAssuranceState>,
) -> AssuranceContext {
    let achieved = achieved_assurance_level(session_state);
    let has_step_up = has_valid_step_up(session_state);
    let mut amr = session_state
        .map(|state| state.amr.clone())
        .unwrap_or_else(|| vec!["pwd".to_string()]);

    if amr.is_empty() {
        amr.push("pwd".to_string());
    }
    if has_step_up && !amr.iter().any(|value| value == "otp") {
        amr.push("otp".to_string());
    }

    let auth_time = session_state
        .and_then(|state| state.step_up_verified_at)
        .or_else(|| session_state.map(|state| state.created_at))
        .unwrap_or_else(Utc::now)
        .timestamp();

    AssuranceContext {
        sufficient: achieved >= required,
        acr: achieved.as_str().to_string(),
        amr,
        auth_time,
    }
}

fn achieved_assurance_level(session_state: Option<&SessionAssuranceState>) -> Aal {
    let mut achieved = session_state
        .and_then(|state| {
            state
                .acr
                .as_deref()
                .and_then(|value| value.parse::<Aal>().ok())
        })
        .unwrap_or(Aal::Aal1);

    if has_valid_step_up(session_state) {
        achieved = std::cmp::max(achieved, Aal::Aal2);
    }

    achieved
}

fn has_valid_step_up(session_state: Option<&SessionAssuranceState>) -> bool {
    session_state.is_some_and(|state| {
        state
            .step_up_expires_at
            .is_some_and(|value| value > Utc::now())
    })
}

fn parse_required_aal(value: Option<String>) -> Aal {
    value
        .and_then(|raw| std::str::FromStr::from_str(&raw).ok())
        .unwrap_or(Aal::Aal1)
}

pub fn _assurance_rank(value: &str) -> i32 {
    match value {
        "aal3" => 3,
        "aal2" => 2,
        _ => 1,
    }
}
