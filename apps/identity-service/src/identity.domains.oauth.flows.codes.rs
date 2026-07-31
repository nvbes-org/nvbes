use crate::domains::oauth::authorization_codes::{
    CachedAuthorizationCode, delete_authorization_code, get_authorization_code,
    mark_authorization_code_consumed, store_authorization_code,
};
use crate::domains::oauth::service::ConsentRequirementInput;
use crate::domains::oauth::validation::validate_redirect_uri_match;
use crate::http::error::AppError;
use crate::{cloud_boundary::workspace_port, domains::auth::jwt::JwtService};
use chrono::Utc;
use nvbes_redis::refresh_token as refresh_store;
use sqlx::Row;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use super::{AuthorizationCodeView, CreateAuthorizationCodeInput, ExchangeCodeInput, TokenView};

/// Create an authorization code.
pub async fn create_authorization_code(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
    input: CreateAuthorizationCodeInput,
) -> Result<AuthorizationCodeView, AppError> {
    let policy = crate::domains::oauth::policies_eval::ensure_client_policy(
        db,
        &input.client_id,
        input.tenant_id,
        input.organization_id,
        input.workspace_id,
        &input.scope,
        input.audience.as_deref(),
        &input.resource_indicators,
    )
    .await?;

    let client_uuid =
        crate::domains::oauth::logic::client_uuid_by_client_id(db, None, &input.client_id).await?;

    crate::domains::oauth::consent::ensure_consent(
        db,
        ConsentRequirementInput {
            user_id,
            client_id: client_uuid,
            tenant_id: input.tenant_id,
            organization_id: input.organization_id,
            workspace_id: input.workspace_id,
            scope: policy.normalized_scope.clone(),
            audience: input.audience.clone(),
            resource_indicators: input.resource_indicators.clone(),
            policy_status: policy.status,
            consent_action: input.consent_action.clone(),
        },
    )
    .await?;

    let code = format!("gxac_{}", Uuid::new_v4().simple());
    let high_assurance = sqlx::query_scalar::<_, bool>(
        "SELECT security_profile = 'high_assurance' FROM oauth_clients WHERE id = $1",
    )
    .bind(client_uuid)
    .fetch_one(db)
    .await?;
    let expires_at = Utc::now()
        + if high_assurance {
            chrono::Duration::seconds(60)
        } else {
            chrono::Duration::minutes(10)
        };
    let scope = policy.normalized_scope.join(" ");

    store_authorization_code(
        redis,
        &CachedAuthorizationCode {
            code: code.clone(),
            client_id: input.client_id.clone(),
            client_uuid,
            user_id,
            client_session_id: Some(session_id),
            redirect_uri: input.redirect_uri.clone(),
            nonce: input.nonce.clone(),
            scope,
            audience: input.audience.clone(),
            resource_indicators: input.resource_indicators.clone(),
            authorization_details: input.authorization_details.clone(),
            code_challenge: input.code_challenge.clone(),
            code_challenge_method: input.code_challenge_method.clone(),
            dpop_jkt: input.dpop_jkt.clone(),
            tenant_id: input.tenant_id,
            organization_id: input.organization_id,
            workspace_id: input.workspace_id,
            expires_at,
            consumed_at: None,
        },
    )
    .await?;

    Ok(AuthorizationCodeView { code, expires_at })
}

/// Exchange an authorization code for tokens.
pub async fn exchange_code(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    auth_refresh_token_ttl_hours: i64,
    input: ExchangeCodeInput,
) -> Result<TokenView, AppError> {
    let lock_key = format!("oauth-authorization-code:{}", input.code);
    let locked = nvbes_redis::lock::acquire(redis, &lock_key, 15)
        .await
        .map_err(|err| AppError::internal("authorization_code_lock_failed", format!("{}", err)))?;
    if !locked {
        return Err(AppError::conflict(
            "authorization_code_locked",
            "The authorization code is currently being exchanged.",
        ));
    }

    let result = async {
        let Some(code) = get_authorization_code(redis, &input.code).await? else {
            return Err(AppError::bad_request("invalid_grant", "Invalid code."));
        };

        if code.expires_at < Utc::now() || code.consumed_at.is_some() {
            return Err(AppError::bad_request(
                "invalid_grant",
                "Code expired or already used.",
            ));
        }
        if code.client_id != input.client_id {
            return Err(AppError::bad_request("invalid_client", "Client mismatch."));
        }

        let client_row = sqlx::query(
            r#"
            SELECT
                id,
                client_secret_hash,
                client_assertion_required,
                client_type::text AS client_type,
                tenant_id,
                revoked_at
            FROM oauth_clients
            WHERE client_id = $1
            LIMIT 1
            "#,
        )
        .bind(&code.client_id)
        .fetch_optional(db)
        .await?;

        let Some(client_row) = client_row else {
            return Err(AppError::not_found(
                "client_not_found",
                "The OAuth client was not found.",
            ));
        };

        if client_row
            .get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
            .is_some()
        {
            return Err(AppError::not_found(
                "client_not_found",
                "The OAuth client was not found.",
            ));
        }

        let client_uuid: Uuid = client_row.get("id");
        if client_uuid != code.client_uuid {
            return Err(AppError::bad_request("invalid_client", "Client mismatch."));
        }

        let client_type: String = client_row.get("client_type");
        let client_tenant_id: Uuid = client_row.get("tenant_id");
        let client_secret_hash: String = client_row.get("client_secret_hash");
        let client_assertion_required: bool = client_row.get("client_assertion_required");
        if !crate::domains::oauth::validation::is_public_client_type(&client_type) {
            if client_assertion_required && !input.client_assertion_verified {
                return Err(AppError::unauthorized(
                    "invalid_client",
                    "private_key_jwt client authentication is required.",
                ));
            }

            if !input.client_assertion_verified {
                let client_secret = input.client_secret.as_deref().ok_or_else(|| {
                    AppError::unauthorized("invalid_client", "Client authentication is required.")
                })?;
                crate::domains::oauth::verify_client_secret_with_overlap(
                    client_tenant_id,
                    &code.client_id,
                    client_secret,
                    &client_secret_hash,
                )
                .await?;
            }
        }

        validate_redirect_uri_match(&code.redirect_uri, input.redirect_uri.as_deref())?;

        crate::domains::oauth::verify_pkce(
            &client_type,
            code.code_challenge.as_deref(),
            code.code_challenge_method.as_deref(),
            input.code_verifier.as_deref(),
        )?;
        validate_authorization_code_sender_binding(
            code.dpop_jkt.as_deref(),
            input.token_confirmation.as_ref(),
        )?;

        crate::domains::oauth::policies_eval::ensure_client_policy(
            db,
            &code.client_id,
            code.tenant_id,
            code.organization_id,
            code.workspace_id,
            &code.scope,
            code.audience.as_deref(),
            &code.resource_indicators,
        )
        .await?;
        let access_token_audience =
            crate::domains::oauth::validation::resolve_access_token_audience(
                code.audience.as_deref(),
                &code.resource_indicators,
            )?;
        crate::domains::oauth::system_clients::validate_system_client_audience(
            &code.client_id,
            &access_token_audience,
        )?;

        mark_authorization_code_consumed(redis, &input.code).await?;

        let assurance = crate::domains::oauth::assurance::resolve_assurance_context(
            db,
            redis,
            code.user_id,
            code.client_session_id,
            Some(&code.client_id),
            code.tenant_id,
            code.organization_id,
            code.workspace_id,
        )
        .await?;
        if !assurance.sufficient {
            return Err(AppError::forbidden(
                "assurance_level_insufficient",
                "The current session assurance level is insufficient for this client/workspace context.",
            ));
        }

        let id_token = if code.scope.split_whitespace().any(|scope| scope == "openid") {
            let nonce = code.nonce.as_deref().ok_or_else(|| {
                AppError::bad_request(
                    "nonce_required",
                    "An OIDC authorization code must contain a nonce.",
                )
            })?;
            Some(
                jwt.generate_id_token(
                    code.user_id,
                    &code.client_id,
                    nonce,
                    assurance.auth_time,
                )
                .await?,
            )
        } else {
            None
        };

        let workspace_region = if let Some(workspace_id) = code.workspace_id {
            workspace_port::get_workspace(code.tenant_id, workspace_id, code.user_id)
                .await?
                .data_region
        } else {
            None
        };

        let tokens = jwt
            .issue_token_pair(crate::domains::auth::jwt::TokenPairIssueRequest {
                user_id: code.user_id,
                access_token_audience: &access_token_audience,
                workspace_id: code.workspace_id,
                workspace_region,
                scope: &code.scope,
                authorization_details: code.authorization_details.clone(),
                session_id: code.client_session_id,
                tenant_id: code.tenant_id,
                organization_id: code.organization_id,
                acr: Some(&assurance.acr),
                amr: Some(assurance.amr.clone()),
                client_id: Some(&code.client_id),
                auth_time: Some(assurance.auth_time),
                confirmation: input.token_confirmation,
            })
            .await?;

        refresh_store::store_refresh_token(
            redis,
            &refresh_store::CachedRefreshToken {
                jti: tokens.refresh_jti.clone(),
                session_id: tokens.session_id,
                principal_id: code.user_id,
                tenant_id: code.tenant_id,
                organization_id: code.organization_id,
                workspace_id: code.workspace_id,
                client_id: Some(code.client_uuid),
                scope: code.scope.clone(),
                audience: code.audience.clone(),
                resource_indicators: code.resource_indicators.clone(),
                authorization_details: code.authorization_details.clone(),
                expires_at: Utc::now() + chrono::Duration::hours(auth_refresh_token_ttl_hours),
                rotated_from_jti: None,
                replaced_by_jti: None,
                reuse_detected_at: None,
                last_used_at: None,
                revoked_at: None,
            },
        )
        .await
        .map_err(|err| AppError::internal("refresh_token_store_failed", format!("{}", err)))?;

        let _ = delete_authorization_code(redis, &code).await;

        Ok(TokenView {
            access_token: tokens.access_token,
            token_type: tokens.token_type,
            expires_in: tokens.expires_in,
            refresh_token: Some(tokens.refresh_token),
            scope: code.scope,
            authorization_details: code.authorization_details,
            id_token,
            issued_token_type: None,
        })
    }
    .await;

    let release_result = nvbes_redis::lock::release(redis, &lock_key)
        .await
        .map_err(|err| AppError::internal("authorization_code_lock_failed", format!("{}", err)));
    release_result?;

    result
}

fn validate_authorization_code_sender_binding(
    expected_jkt: Option<&str>,
    actual: Option<&crate::domains::auth::jwt::TokenConfirmation>,
) -> Result<(), AppError> {
    let actual_jkt = actual.and_then(|confirmation| confirmation.jkt.as_deref());
    if expected_jkt.is_none() || expected_jkt == actual_jkt {
        Ok(())
    } else {
        Err(AppError::bad_request(
            "invalid_grant",
            "The authorization code is bound to a different DPoP key.",
        ))
    }
}

#[cfg(test)]
mod sender_binding_tests {
    use super::validate_authorization_code_sender_binding;
    use crate::domains::auth::jwt::TokenConfirmation;

    #[test]
    fn authorization_code_replay_with_another_dpop_key_is_rejected() {
        let confirmation = TokenConfirmation::dpop("attacker".to_string());

        let error =
            validate_authorization_code_sender_binding(Some("expected"), Some(&confirmation))
                .expect_err("another DPoP key must not redeem the code");
        assert_eq!(error.code, "invalid_grant");
    }

    #[test]
    fn authorization_code_requires_the_bound_dpop_proof() {
        let error = validate_authorization_code_sender_binding(Some("expected"), None)
            .expect_err("a bound code must not be redeemable without its DPoP proof");
        assert_eq!(error.code, "invalid_grant");
    }
}
