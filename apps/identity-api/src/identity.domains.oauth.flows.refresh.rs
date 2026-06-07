use chrono::Utc;
use sqlx::Row;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::domains::auth::jwt::JwtService;
use crate::http::error::AppError;
use nvbes_redis::refresh_token as refresh_store;

use super::{ClientAuthentication, TokenView};

/// Refresh an access token.
pub async fn refresh_token(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    auth_refresh_token_ttl_hours: i64,
    refresh_token: &str,
    client_auth: ClientAuthentication,
) -> Result<TokenView, AppError> {
    let claims = jwt.decode_token(refresh_token, "refresh")?;
    if claims.client_id.as_deref() != Some(client_auth.client_id.as_str()) {
        return Err(AppError::unauthorized(
            "invalid_client",
            "Client authentication does not match the refresh token.",
        ));
    }

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::internal("invalid_user_id", &format!("{}", e)))?;
    let session_id = Uuid::parse_str(&claims.sid)
        .map_err(|e| AppError::internal("invalid_session_id", &format!("{}", e)))?;

    let lock_key = format!("oauth-refresh-token:{}", claims.jti);
    let locked = nvbes_redis::lock::acquire(redis, &lock_key, 15)
        .await
        .map_err(|err| AppError::internal("refresh_token_lock_failed", &format!("{}", err)))?;
    if !locked {
        return Err(AppError::conflict(
            "refresh_token_locked",
            "The refresh token is currently being processed.",
        ));
    }

    let result = async {
        let Some(session) = refresh_store::get_refresh_token(redis, &claims.jti)
            .await
            .map_err(|err| AppError::internal("refresh_token_lookup_failed", &format!("{}", err)))?
        else {
            refresh_store::revoke_refresh_family(redis, user_id, session_id, &claims.jti)
                .await
                .map_err(|err| AppError::internal("refresh_token_revoke_failed", &format!("{}", err)))?;
            return Err(AppError::unauthorized(
                "refresh_token_not_registered",
                "Refresh token is not registered",
            ));
        };

        let client_uuid = session.client_id.ok_or_else(|| {
            AppError::unauthorized("invalid_client", "The OAuth client is invalid or revoked.")
        })?;

        let client_row = sqlx::query(
            r#"
            SELECT
                id,
                client_id,
                client_type::text AS client_type,
                client_secret_hash,
                client_assertion_required,
                revoked_at
            FROM oauth_clients
            WHERE id = $1
            LIMIT 1
            "#,
        )
        .bind(client_uuid)
        .fetch_optional(db)
        .await?;

        let Some(client_row) = client_row else {
            refresh_store::revoke_refresh_family(redis, user_id, session_id, &claims.jti)
                .await
                .map_err(|err| AppError::internal("refresh_token_revoke_failed", &format!("{}", err)))?;
            return Err(AppError::unauthorized(
                "invalid_client",
                "The OAuth client is invalid or revoked.",
            ));
        };

        let oauth_client_id: String = client_row.get("client_id");
        let client_revoked_at: Option<chrono::DateTime<Utc>> = client_row.get("revoked_at");
        if oauth_client_id != client_auth.client_id || client_revoked_at.is_some() {
            refresh_store::revoke_refresh_family(redis, user_id, session_id, &claims.jti)
                .await
                .map_err(|err| AppError::internal("refresh_token_revoke_failed", &format!("{}", err)))?;
            return Err(AppError::unauthorized(
                "invalid_client",
                "The OAuth client is invalid or revoked.",
            ));
        }

        let oauth_client_type: String = client_row.get("client_type");
        let client_secret_hash: String = client_row.get("client_secret_hash");
        let client_assertion_required: bool = client_row.get("client_assertion_required");
        if !crate::domains::oauth::validation::is_public_client_type(&oauth_client_type) {
            if client_assertion_required && !client_auth.client_assertion_verified {
                return Err(AppError::unauthorized(
                    "invalid_client",
                    "private_key_jwt client authentication is required.",
                ));
            }

            if !client_auth.client_assertion_verified {
                let secret = client_auth.client_secret.as_deref().ok_or_else(|| {
                    AppError::unauthorized("invalid_client", "Client authentication is required.")
                })?;
                crate::domains::oauth::verify_client_secret(secret, &client_secret_hash)?;
            }
        }

        if session.expires_at < Utc::now() {
            return Err(AppError::unauthorized(
                "refresh_token_expired",
                "Refresh token has expired",
            ));
        }

        if session.revoked_at.is_some()
            || session.reuse_detected_at.is_some()
            || session.replaced_by_jti.is_some()
        {
            refresh_store::revoke_refresh_family(redis, user_id, session_id, &claims.jti)
                .await
                .map_err(|err| AppError::internal("refresh_token_revoke_failed", &format!("{}", err)))?;
            return Err(AppError::unauthorized(
                "refresh_token_reused",
                "Refresh token reuse was detected and the session has been revoked.",
            ));
        }

        let refresh_scope = session.scope.clone();
        let authorization_details = session.authorization_details.clone();
        let session_workspace_id = session.workspace_id;
        let next_workspace_id = if let Some(wid) = claims.workspace_id {
            Some(
                Uuid::parse_str(&wid)
                    .map_err(|e| AppError::internal("invalid_workspace_id", &format!("{}", e)))?,
            )
        } else {
            session_workspace_id
        };

        let refreshed_assurance = crate::domains::oauth::assurance::resolve_assurance_context(
            db,
            redis,
            user_id,
            Some(session_id),
            claims.client_id.as_deref(),
            session.tenant_id,
            session.organization_id,
            next_workspace_id,
        )
        .await?;
        if !refreshed_assurance.sufficient {
            return Err(AppError::forbidden(
                "assurance_level_insufficient",
                "The current session assurance level is insufficient for this client/workspace context.",
            ));
        }

        let workspace_region = if let Some(workspace_id) = next_workspace_id {
            sqlx::query_scalar::<_, Option<String>>(
                "SELECT data_region::text FROM workspaces WHERE id = $1",
            )
            .bind(workspace_id)
            .fetch_optional(db)
            .await?
            .flatten()
        } else {
            None
        };

        let tokens = jwt.generate_token_pair_with_authorization_details(
            user_id,
            next_workspace_id,
            workspace_region,
            &refresh_scope,
            authorization_details.clone(),
            Some(session_id),
            session.tenant_id,
            session.organization_id,
            Some(&refreshed_assurance.acr),
            Some(refreshed_assurance.amr.clone()),
            Some(&client_auth.client_id),
            claims.auth_time,
            None,
        )?;

        refresh_store::mark_refresh_token_used(
            redis,
            &claims.jti,
            Some(&tokens.refresh_jti),
        )
        .await
        .map_err(|err| AppError::internal("refresh_token_update_failed", &format!("{}", err)))?;

        let new_refresh = refresh_store::CachedRefreshToken {
            jti: tokens.refresh_jti.clone(),
            session_id,
            principal_id: user_id,
            tenant_id: session.tenant_id,
            organization_id: session.organization_id,
            workspace_id: next_workspace_id,
            client_id: Some(client_uuid),
            scope: refresh_scope.clone(),
            authorization_details: authorization_details.clone(),
            expires_at: Utc::now() + chrono::Duration::hours(auth_refresh_token_ttl_hours),
            rotated_from_jti: Some(claims.jti.clone()),
            replaced_by_jti: None,
            reuse_detected_at: None,
            last_used_at: None,
            revoked_at: None,
        };

        refresh_store::store_refresh_token(redis, &new_refresh)
            .await
            .map_err(|err| AppError::internal("refresh_token_store_failed", &format!("{}", err)))?;

        Ok(TokenView {
            access_token: tokens.access_token,
            token_type: tokens.token_type,
            expires_in: tokens.expires_in,
            refresh_token: Some(tokens.refresh_token),
            scope: refresh_scope,
            authorization_details,
            issued_token_type: None,
        })
    }
    .await;

    let release_result = nvbes_redis::lock::release(redis, &lock_key)
        .await
        .map_err(|err| AppError::internal("refresh_token_lock_failed", &format!("{}", err)));
    release_result?;

    result
}

/// Revoke a refresh token family.
pub async fn revoke_refresh_family(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
    jti: &str,
) -> Result<(), AppError> {
    refresh_store::revoke_refresh_family(redis, user_id, session_id, jti)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", &format!("{}", err)))
}
