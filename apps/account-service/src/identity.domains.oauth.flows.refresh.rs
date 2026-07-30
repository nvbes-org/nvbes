use chrono::Utc;
use sqlx::Row;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;
use crate::{cloud_boundary::workspace_port, domains::auth::jwt::JwtService};
use nvbes_redis::refresh_token as refresh_store;

use super::{ClientAuthentication, TokenView};

/// Refresh an access token.
#[expect(
    clippy::too_many_arguments,
    reason = "the OAuth refresh boundary keeps storage, signer, client authentication, security profile, and sender binding explicit"
)]
pub async fn refresh_token(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    auth_refresh_token_ttl_hours: i64,
    refresh_token: &str,
    client_auth: ClientAuthentication,
    security_profile: crate::domains::oauth::profiles::OAuthSecurityProfile,
    token_confirmation: Option<crate::domains::auth::jwt::TokenConfirmation>,
) -> Result<TokenView, AppError> {
    let claims = jwt.decode_token(refresh_token, "refresh")?;
    validate_refresh_sender_binding(claims.cnf.as_ref(), token_confirmation.as_ref())?;
    if claims.client_id.as_deref() != Some(client_auth.client_id.as_str()) {
        return Err(AppError::unauthorized(
            "invalid_client",
            "Client authentication does not match the refresh token.",
        ));
    }

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::internal("invalid_user_id", format!("{}", e)))?;
    let session_id = Uuid::parse_str(&claims.sid)
        .map_err(|e| AppError::internal("invalid_session_id", format!("{}", e)))?;

    let lock_key = format!("oauth-refresh-token:{}", claims.jti);
    let locked = nvbes_redis::lock::acquire(redis, &lock_key, 15)
        .await
        .map_err(|err| AppError::internal("refresh_token_lock_failed", format!("{}", err)))?;
    if !locked {
        return Err(AppError::conflict(
            "refresh_token_locked",
            "The refresh token is currently being processed.",
        ));
    }

    let result = async {
        let Some(session) = refresh_store::get_refresh_token(redis, &claims.jti)
            .await
            .map_err(|err| AppError::internal("refresh_token_lookup_failed", format!("{}", err)))?
        else {
            let _ = crate::domains::oauth::security_events::enqueue_session_revoked(
                db, redis, user_id, session_id,
            )
            .await;
            refresh_store::revoke_refresh_family(redis, user_id, session_id, &claims.jti)
                .await
                .map_err(|err| AppError::internal("refresh_token_revoke_failed", format!("{}", err)))?;
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
                tenant_id,
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
                .map_err(|err| AppError::internal("refresh_token_revoke_failed", format!("{}", err)))?;
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
                .map_err(|err| AppError::internal("refresh_token_revoke_failed", format!("{}", err)))?;
            return Err(AppError::unauthorized(
                "invalid_client",
                "The OAuth client is invalid or revoked.",
            ));
        }

        let oauth_client_type: String = client_row.get("client_type");
        let client_tenant_id: Uuid = client_row.get("tenant_id");
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
                crate::domains::oauth::verify_client_secret_with_overlap(
                    client_tenant_id,
                    &client_auth.client_id,
                    secret,
                    &client_secret_hash,
                )
                .await?;
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
            let _ = crate::domains::oauth::security_events::enqueue_refresh_token_compromise(
                db, redis, user_id, session_id,
            )
            .await;
            let _ = crate::domains::oauth::security_events::enqueue_session_revoked(
                db, redis, user_id, session_id,
            )
            .await;
            refresh_store::revoke_refresh_family(redis, user_id, session_id, &claims.jti)
                .await
                .map_err(|err| AppError::internal("refresh_token_revoke_failed", format!("{}", err)))?;
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
                    .map_err(|e| AppError::internal("invalid_workspace_id", format!("{}", e)))?,
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
            workspace_port::get_workspace(session.tenant_id, workspace_id, user_id)
                .await?
                .data_region
        } else {
            None
        };

        let tokens = jwt
            .issue_token_pair(crate::domains::auth::jwt::TokenPairIssueRequest {
                user_id,
                workspace_id: next_workspace_id,
                workspace_region,
                scope: &refresh_scope,
                authorization_details: authorization_details.clone(),
                session_id: Some(session_id),
                tenant_id: session.tenant_id,
                organization_id: session.organization_id,
                acr: Some(&refreshed_assurance.acr),
                amr: Some(refreshed_assurance.amr.clone()),
                client_id: Some(&client_auth.client_id),
                auth_time: claims.auth_time,
                confirmation: token_confirmation,
            })
            .await?;

        let returned_refresh_token = if security_profile
            == crate::domains::oauth::profiles::OAuthSecurityProfile::HighAssurance
        {
            refresh_store::touch_refresh_token(redis, &claims.jti)
                .await
                .map_err(|err| {
                    AppError::internal("refresh_token_update_failed", format!("{}", err))
                })?;
            refresh_token.to_string()
        } else {
            refresh_store::mark_refresh_token_used(
                redis,
                &claims.jti,
                Some(&tokens.refresh_jti),
            )
            .await
            .map_err(|err| {
                AppError::internal("refresh_token_update_failed", format!("{}", err))
            })?;

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
                .map_err(|err| {
                    AppError::internal("refresh_token_store_failed", format!("{}", err))
                })?;
            tokens.refresh_token
        };

        Ok(TokenView {
            access_token: tokens.access_token,
            token_type: tokens.token_type,
            expires_in: tokens.expires_in,
            refresh_token: Some(returned_refresh_token),
            id_token: None,
            scope: refresh_scope,
            authorization_details,
            issued_token_type: None,
        })
    }
    .await;

    let release_result = nvbes_redis::lock::release(redis, &lock_key)
        .await
        .map_err(|err| AppError::internal("refresh_token_lock_failed", format!("{}", err)));
    release_result?;

    result
}

fn validate_refresh_sender_binding(
    expected: Option<&crate::domains::auth::jwt::TokenConfirmation>,
    actual: Option<&crate::domains::auth::jwt::TokenConfirmation>,
) -> Result<(), AppError> {
    let matches = match (expected, actual) {
        (None, None) => true,
        (Some(expected), Some(actual)) => {
            expected.jkt == actual.jkt && expected.x5t_s256 == actual.x5t_s256
        }
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        Err(AppError::unauthorized(
            "sender_constraint_mismatch",
            "The refresh token sender constraint does not match the current request.",
        ))
    }
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
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", format!("{}", err)))
}

#[cfg(test)]
mod tests {
    use super::validate_refresh_sender_binding;
    use crate::domains::auth::jwt::TokenConfirmation;

    #[test]
    fn refresh_replay_with_another_dpop_key_is_rejected() {
        let expected = TokenConfirmation::dpop("expected".to_string());
        let replay = TokenConfirmation::dpop("attacker".to_string());

        let error = validate_refresh_sender_binding(Some(&expected), Some(&replay))
            .expect_err("another DPoP key must not replay a refresh token");
        assert_eq!(error.code, "sender_constraint_mismatch");
    }

    #[test]
    fn refresh_replay_with_another_certificate_is_rejected() {
        let expected = TokenConfirmation::mtls("expected".to_string());
        let replay = TokenConfirmation::mtls("attacker".to_string());

        let error = validate_refresh_sender_binding(Some(&expected), Some(&replay))
            .expect_err("another certificate must not replay a refresh token");
        assert_eq!(error.code, "sender_constraint_mismatch");
    }
}
