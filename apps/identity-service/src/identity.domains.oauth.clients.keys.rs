use chrono::{Duration, Utc};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::domains::oauth::logic::OAuthManagementAuth;
use crate::http::error::AppError;

#[path = "identity.domains.oauth.clients.keys.models.rs"]
mod models;
#[path = "identity.domains.oauth.clients.keys.validation.rs"]
mod validation;

pub use models::{OAuthClientKeyPurpose, OAuthClientKeyView, RotateOAuthClientKeyInput};
use validation::{invalid_key, key_id, validate_public_jwk};

const MAX_ROTATION_GRACE_SECONDS: i64 = 86_400;

pub async fn active_jwks(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    client_id: &str,
    purpose: OAuthClientKeyPurpose,
) -> Result<serde_json::Value, AppError> {
    let mut tx = db.begin().await?;
    set_tenant_context(&mut tx, tenant_id).await?;
    let keys = sqlx::query_scalar::<_, serde_json::Value>(
        r#"
        SELECT jwk
        FROM oauth_client_keys
        WHERE tenant_id = $1
          AND client_id = $2
          AND purpose = $3::oauth_client_key_purpose
          AND (
            status = 'active'
            OR (status = 'retiring' AND retire_at > NOW())
          )
        ORDER BY activated_at DESC
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(purpose.as_str())
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(serde_json::json!({ "keys": keys }))
}

pub async fn insert_initial_keys(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    client_id: &str,
    client_assertion_jwk: Option<&serde_json::Value>,
    request_object_jwks: Option<&serde_json::Value>,
    high_assurance: bool,
) -> Result<(), AppError> {
    set_tenant_context(tx, tenant_id).await?;
    if let Some(jwk) = client_assertion_jwk {
        insert_key(
            tx,
            tenant_id,
            client_id,
            OAuthClientKeyPurpose::ClientAuthentication,
            jwk,
            high_assurance,
        )
        .await?;
    }
    if let Some(jwks) = request_object_jwks {
        for jwk in jwks
            .get("keys")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| invalid_key("request_object_signing_jwks must contain a keys array."))?
        {
            insert_key(
                tx,
                tenant_id,
                client_id,
                OAuthClientKeyPurpose::RequestObject,
                jwk,
                high_assurance,
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn list_keys(
    db: &sqlx::PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    client_id: &str,
) -> Result<Vec<OAuthClientKeyView>, AppError> {
    let tenant_id = super::require_oauth_management_tenant(db, auth).await?;
    let mut tx = db.begin().await?;
    set_tenant_context(&mut tx, tenant_id).await?;
    ensure_client_owned(&mut tx, tenant_id, client_id).await?;
    let rows = sqlx::query(
        r#"
        SELECT id, purpose::text AS purpose, kid, jwk, status::text AS status,
               activated_at, retire_at, revoked_at
        FROM oauth_client_keys
        WHERE tenant_id = $1 AND client_id = $2
        ORDER BY purpose, activated_at DESC
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows.iter().map(map_key).collect())
}

pub async fn rotate_key(
    db: &sqlx::PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    client_id: &str,
    input: RotateOAuthClientKeyInput,
) -> Result<OAuthClientKeyView, AppError> {
    if !(0..=MAX_ROTATION_GRACE_SECONDS).contains(&input.retire_previous_after_seconds) {
        return Err(invalid_key(
            "retire_previous_after_seconds must be between 0 and 86400.",
        ));
    }
    let purpose = OAuthClientKeyPurpose::parse(&input.purpose)?;
    let tenant_id = super::require_oauth_management_tenant(db, auth).await?;
    let mut tx = db.begin().await?;
    set_tenant_context(&mut tx, tenant_id).await?;
    let high_assurance = ensure_client_owned(&mut tx, tenant_id, client_id).await?;
    validate_public_jwk(purpose, &input.jwk, high_assurance)?;
    let kid = key_id(&input.jwk)?;
    let duplicate = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM oauth_client_keys WHERE client_id = $1 AND purpose = $2::oauth_client_key_purpose AND kid = $3)",
    )
    .bind(client_id)
    .bind(purpose.as_str())
    .bind(kid)
    .fetch_one(&mut *tx)
    .await?;
    if duplicate {
        return Err(invalid_key(
            "A key with this purpose and kid already exists.",
        ));
    }

    retire_current_keys(
        &mut tx,
        tenant_id,
        client_id,
        purpose,
        input.retire_previous_after_seconds,
    )
    .await?;
    let row = insert_key(
        &mut tx,
        tenant_id,
        client_id,
        purpose,
        &input.jwk,
        high_assurance,
    )
    .await?;
    tx.commit().await?;
    Ok(map_key(&row))
}

pub async fn revoke_key(
    db: &sqlx::PgPool,
    auth: &(impl OAuthManagementAuth + crate::domains::authz::TenantManagementAuth),
    client_id: &str,
    key_id: Uuid,
) -> Result<OAuthClientKeyView, AppError> {
    let tenant_id = super::require_oauth_management_tenant(db, auth).await?;
    let mut tx = db.begin().await?;
    set_tenant_context(&mut tx, tenant_id).await?;
    let high_assurance = ensure_client_owned(&mut tx, tenant_id, client_id).await?;
    let row = sqlx::query(
        r#"
        UPDATE oauth_client_keys
        SET status = 'revoked', revoked_at = NOW(), retire_at = NULL
        WHERE id = $1 AND tenant_id = $2 AND client_id = $3 AND status <> 'revoked'
        RETURNING id, purpose::text AS purpose, kid, jwk, status::text AS status,
                  activated_at, retire_at, revoked_at
        "#,
    )
    .bind(key_id)
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("oauth_client_key_not_found", "The key was not found."))?;
    if high_assurance {
        let purpose: String = row.get("purpose");
        let remaining = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
              SELECT 1 FROM oauth_client_keys
              WHERE tenant_id = $1 AND client_id = $2
                AND purpose = $3::oauth_client_key_purpose
                AND (status = 'active' OR (status = 'retiring' AND retire_at > NOW()))
            )
            "#,
        )
        .bind(tenant_id)
        .bind(client_id)
        .bind(&purpose)
        .fetch_one(&mut *tx)
        .await?;
        if !remaining {
            return Err(AppError::conflict(
                "last_high_assurance_key",
                "A high-assurance client must retain a usable key for each configured purpose.",
            ));
        }
    }
    tx.commit().await?;
    Ok(map_key(&row))
}

async fn insert_key<'a>(
    tx: &mut Transaction<'a, Postgres>,
    tenant_id: Uuid,
    client_id: &str,
    purpose: OAuthClientKeyPurpose,
    jwk: &serde_json::Value,
    high_assurance: bool,
) -> Result<sqlx::postgres::PgRow, AppError> {
    validate_public_jwk(purpose, jwk, high_assurance)?;
    sqlx::query(
        r#"
        INSERT INTO oauth_client_keys (tenant_id, client_id, purpose, kid, jwk)
        VALUES ($1, $2, $3::oauth_client_key_purpose, $4, $5)
        RETURNING id, purpose::text AS purpose, kid, jwk, status::text AS status,
                  activated_at, retire_at, revoked_at
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(purpose.as_str())
    .bind(key_id(jwk)?)
    .bind(jwk)
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::from)
}

async fn retire_current_keys(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    client_id: &str,
    purpose: OAuthClientKeyPurpose,
    grace_seconds: i64,
) -> Result<(), AppError> {
    if grace_seconds == 0 {
        sqlx::query(
            "UPDATE oauth_client_keys SET status = 'revoked', revoked_at = NOW(), retire_at = NULL WHERE tenant_id = $1 AND client_id = $2 AND purpose = $3::oauth_client_key_purpose AND status IN ('active', 'retiring')",
        )
        .bind(tenant_id)
        .bind(client_id)
        .bind(purpose.as_str())
        .execute(&mut **tx)
        .await?;
    } else {
        let retire_at = Utc::now() + Duration::seconds(grace_seconds);
        sqlx::query(
            "UPDATE oauth_client_keys SET status = 'retiring', retire_at = $4, revoked_at = NULL WHERE tenant_id = $1 AND client_id = $2 AND purpose = $3::oauth_client_key_purpose AND status = 'active'",
        )
        .bind(tenant_id)
        .bind(client_id)
        .bind(purpose.as_str())
        .bind(retire_at)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn ensure_client_owned(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<bool, AppError> {
    sqlx::query_scalar::<_, String>(
        "SELECT security_profile::text FROM oauth_clients WHERE tenant_id = $1 AND client_id = $2 AND revoked_at IS NULL",
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(&mut **tx)
    .await?
    .map(|profile| profile == "high_assurance")
    .ok_or_else(|| AppError::not_found("oauth_client_not_found", "The OAuth client was not found."))
}

async fn set_tenant_context(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    nvbes_tenancy::set_transaction_rls_context(
        tx,
        nvbes_tenancy::RlsContext {
            tenant_id: Some(tenant_id),
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

fn map_key(row: &sqlx::postgres::PgRow) -> OAuthClientKeyView {
    OAuthClientKeyView {
        id: row.get("id"),
        purpose: row.get("purpose"),
        kid: row.get("kid"),
        jwk: row.get("jwk"),
        status: row.get("status"),
        activated_at: row.get("activated_at"),
        retire_at: row.get("retire_at"),
        revoked_at: row.get("revoked_at"),
    }
}

#[cfg(test)]
#[path = "identity.domains.oauth.clients.keys.tests.rs"]
mod tests;
