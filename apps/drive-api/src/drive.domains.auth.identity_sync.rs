use chrono::{DateTime, TimeZone, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::identity::IdentityIntrospectionResponse;
use super::types::{AuthActorContext, AuthContext, AuthPrincipalKind};
use crate::http::error::AppError;

#[derive(Debug, FromRow)]
pub(crate) struct IdentityUserRecord {
    pub id: Uuid,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub status: String,
    pub identity_subject: Option<String>,
}

impl IdentityUserRecord {
    pub(crate) fn into_auth_context(
        self,
        claims: &IdentityIntrospectionResponse,
        session_id: Option<Uuid>,
        auth_time: Option<DateTime<Utc>>,
    ) -> AuthContext {
        let email_verified_at = claims.email_verified.and_then(|verified| {
            verified.then_some(self.email_verified_at.unwrap_or_else(Utc::now))
        });
        let principal_kind = parse_principal_kind(claims.principal_type.as_deref());
        let actor = parse_actor_context(claims).ok().flatten();

        AuthContext {
            principal_id: required_claim_uuid(claims.sub.as_deref()).unwrap_or(self.id),
            principal_kind,
            user_id: self.id,
            email_verified_at,
            session_id: session_id.unwrap_or_else(Uuid::nil),
            tenant_id: claims.tenant_id,
            organization_id: claims.organization_id,
            workspace_id: claims.workspace_id,
            scope: claims.scope.clone().unwrap_or_default(),
            role: claims.role.clone(),
            amr: claims.amr.clone(),
            actor,
            acr: claims.acr.clone(),
            auth_time,
        }
    }
}

pub(crate) async fn sync_identity_user(
    db: &sqlx::PgPool,
    claims: &IdentityIntrospectionResponse,
) -> Result<IdentityUserRecord, AppError> {
    let subject = required_claim_string(claims.sub.as_deref())?;
    let principal_kind = parse_principal_kind(claims.principal_type.as_deref());
    let email = identity_email(claims, &subject, principal_kind)?;
    let name = identity_name(claims, &email, principal_kind);
    let verified = if principal_kind == AuthPrincipalKind::ServiceAccount {
        true
    } else {
        claims.email_verified.unwrap_or(false)
    };
    let desired_email_verified_at = if verified { Some(Utc::now()) } else { None };
    let desired_status = if verified || principal_kind == AuthPrincipalKind::ServiceAccount {
        "active"
    } else {
        "pending_verification"
    };

    let mut tx = db.begin().await?;

    if let Some(user) = sqlx::query_as::<_, IdentityUserRecord>(
        r#"
        SELECT id, email_verified_at, status::text AS status, identity_subject
        FROM users
        WHERE identity_subject = $1
        LIMIT 1
        "#,
    )
    .bind(&subject)
    .fetch_optional(&mut *tx)
    .await?
    {
        if is_unavailable_status(&user.status) {
            return Err(AppError::forbidden(
                "account_unavailable",
                "This account is not available.",
            ));
        }

        let user = sqlx::query_as::<_, IdentityUserRecord>(
            r#"
            UPDATE users
            SET email = $2,
                display_name = $3,
                email_verified_at = CASE
                    WHEN $4::boolean THEN COALESCE(email_verified_at, NOW())
                    ELSE email_verified_at
                END,
                status = CASE
                    WHEN $4::boolean THEN 'active'::user_status
                    ELSE status
                END,
                identity_subject = COALESCE(identity_subject, $5),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, email_verified_at, status::text AS status, identity_subject
            "#,
        )
        .bind(user.id)
        .bind(&email)
        .bind(&name)
        .bind(verified)
        .bind(&subject)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        return Ok(user);
    }

    if let Some(user) = sqlx::query_as::<_, IdentityUserRecord>(
        r#"
        SELECT id, email_verified_at, status::text AS status, identity_subject
        FROM users
        WHERE email = $1
        LIMIT 1
        "#,
    )
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?
    {
        if is_unavailable_status(&user.status) {
            return Err(AppError::forbidden(
                "account_unavailable",
                "This account is not available.",
            ));
        }

        if user
            .identity_subject
            .as_deref()
            .is_some_and(|value| value != subject)
        {
            return Err(AppError::forbidden(
                "identity_account_mismatch",
                "This account is already linked to a different Identity subject.",
            ));
        }

        let user = sqlx::query_as::<_, IdentityUserRecord>(
            r#"
            UPDATE users
            SET email = $2,
                display_name = $3,
                email_verified_at = CASE
                    WHEN $4::boolean THEN COALESCE(email_verified_at, NOW())
                    ELSE email_verified_at
                END,
                status = CASE
                    WHEN $4::boolean THEN 'active'::user_status
                    ELSE status
                END,
                identity_subject = COALESCE(identity_subject, $5),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, email_verified_at, status::text AS status, identity_subject
            "#,
        )
        .bind(user.id)
        .bind(&email)
        .bind(&name)
        .bind(verified)
        .bind(&subject)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        return Ok(user);
    }

    let user = sqlx::query_as::<_, IdentityUserRecord>(
        r#"
        INSERT INTO users (
          email,
          display_name,
          status,
          email_verified_at,
          identity_subject
        )
        VALUES ($1, $2, $3::user_status, $4, $5)
        RETURNING id, email_verified_at, status::text AS status, identity_subject
        "#,
    )
    .bind(&email)
    .bind(&name)
    .bind(desired_status)
    .bind(desired_email_verified_at)
    .bind(&subject)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(user)
}

pub(crate) fn parse_optional_identity_session_id(
    value: Option<&str>,
) -> Result<Option<Uuid>, AppError> {
    match value {
        Some(value) => Uuid::parse_str(value).map(Some).map_err(|_| {
            AppError::unauthorized("invalid_token", "The access token is invalid or expired.")
        }),
        None => Ok(None),
    }
}

pub(crate) fn parse_identity_auth_time(
    value: Option<i64>,
) -> Result<Option<DateTime<Utc>>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };

    Utc.timestamp_opt(value, 0)
        .single()
        .map(Some)
        .ok_or_else(|| {
            AppError::unauthorized("invalid_token", "The access token is invalid or expired.")
        })
}

fn required_claim_string(value: Option<&str>) -> Result<String, AppError> {
    let value = value.ok_or_else(|| {
        AppError::unauthorized("invalid_token", "The access token is invalid or expired.")
    })?;

    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::unauthorized(
            "invalid_token",
            "The access token is invalid or expired.",
        ));
    }

    Ok(value.to_owned())
}

fn required_claim_uuid(value: Option<&str>) -> Result<Uuid, AppError> {
    let value = required_claim_string(value)?;
    Uuid::parse_str(&value).map_err(|_| {
        AppError::unauthorized("invalid_token", "The access token is invalid or expired.")
    })
}

fn parse_principal_kind(value: Option<&str>) -> AuthPrincipalKind {
    match value.unwrap_or("user") {
        "service_account" => AuthPrincipalKind::ServiceAccount,
        _ => AuthPrincipalKind::User,
    }
}

fn identity_email(
    claims: &IdentityIntrospectionResponse,
    subject: &str,
    principal_kind: AuthPrincipalKind,
) -> Result<String, AppError> {
    if principal_kind == AuthPrincipalKind::ServiceAccount {
        return Ok(format!("service-account+{}@nvbes.machine", subject));
    }

    required_claim_string(claims.email.as_deref())
}

fn identity_name(
    claims: &IdentityIntrospectionResponse,
    email: &str,
    principal_kind: AuthPrincipalKind,
) -> String {
    claims
        .name
        .as_deref()
        .or(claims.username.as_deref())
        .or(claims.client_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            if principal_kind == AuthPrincipalKind::ServiceAccount {
                format!("Service account {email}")
            } else {
                email.to_owned()
            }
        })
}

fn parse_actor_context(
    claims: &IdentityIntrospectionResponse,
) -> Result<Option<AuthActorContext>, AppError> {
    let Some(act) = claims.act.as_ref() else {
        return Ok(None);
    };

    let principal_id = act
        .get("sub")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::unauthorized("invalid_token", "The access token is invalid or expired.")
        })?;
    let principal_id = Uuid::parse_str(principal_id).map_err(|_| {
        AppError::unauthorized("invalid_token", "The access token is invalid or expired.")
    })?;

    Ok(Some(AuthActorContext {
        principal_id,
        principal_kind: parse_principal_kind(claims.actor_principal_type.as_deref()),
        tenant_id: claims.actor_tenant_id,
        organization_id: claims.actor_organization_id,
        workspace_id: claims.actor_workspace_id,
        role: claims.actor_role.clone(),
        client_id: act
            .get("client_id")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
    }))
}

fn is_unavailable_status(status: &str) -> bool {
    matches!(status, "suspended" | "deleted")
}
