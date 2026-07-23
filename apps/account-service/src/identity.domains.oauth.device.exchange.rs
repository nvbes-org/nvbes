use chrono::Utc;
use sqlx::Row;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use super::device_codes::{
    delete_device_code, get_device_code_by_device_code, is_expired, save_device_code,
};
use super::service::types::{ExchangeDeviceCodeInput, TokenView};
use crate::http::error::AppError;
use crate::{cloud_boundary::workspace_port, domains::auth::jwt::JwtService};
use nvbes_redis::refresh_token as refresh_store;

pub async fn exchange_device_code(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    auth_refresh_token_ttl_hours: i64,
    input: ExchangeDeviceCodeInput,
) -> Result<TokenView, AppError> {
    let lock_key = format!("oauth-device-code:{}", input.device_code);
    let locked = nvbes_redis::lock::acquire(redis, &lock_key, 15)
        .await
        .map_err(|err| AppError::internal("device_code_lock_failed", format!("{}", err)))?;
    if !locked {
        return Err(AppError::conflict(
            "device_code_locked",
            "The device code is currently being processed.",
        ));
    }

    let result = async {
        let Some(mut code) = get_device_code_by_device_code(redis, &input.device_code).await? else {
            return Err(AppError::unauthorized("invalid_grant", "The device code is invalid."));
        };

        if code.client_id != input.client_id {
            return Err(AppError::unauthorized(
                "invalid_client",
                "The device code was not issued to this OAuth client.",
            ));
        }

        let client = sqlx::query(
            r#"
            SELECT id, client_id as client_id_str, revoked_at
            FROM oauth_clients
            WHERE client_id = $1
            LIMIT 1
            "#,
        )
        .bind(&input.client_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::unauthorized("invalid_client", "Client not found."))?;

        if client
            .get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
            .is_some()
        {
            return Err(AppError::unauthorized(
                "invalid_client",
                "The OAuth client was revoked.",
            ));
        }

        let client_uuid: Uuid = client.get("id");
        if client_uuid != code.client_uuid {
            return Err(AppError::unauthorized(
                "invalid_client",
                "The device code was not issued to this OAuth client.",
            ));
        }

        if is_expired(code.expires_at) {
            return Err(AppError::unauthorized(
                "expired_token",
                "The device code has expired.",
            ));
        }

        if code.denied_at.is_some() {
            return Err(AppError::unauthorized(
                "access_denied",
                "The user denied the authorization.",
            ));
        }

        if code.approved_at.is_none() {
            if let Some(last) = code.last_polled_at
                && last + chrono::Duration::seconds(code.interval_seconds as i64) > Utc::now()
            {
                return Err(AppError::unauthorized(
                    "slow_down",
                    "Polling too frequently.",
                ));
            }

            code.last_polled_at = Some(Utc::now());
            save_device_code(redis, &code).await?;

            return Err(AppError::unauthorized(
                "authorization_pending",
                "The user has not yet approved the device.",
            ));
        }

        let principal_id: Uuid = code
            .principal_id
            .ok_or_else(|| AppError::unauthorized("invalid_grant", "The device code is incomplete."))?;
        let tenant_id: Uuid = code.tenant_id;
        let organization_id: Option<Uuid> = code.organization_id;
        let workspace_id: Uuid = code
            .workspace_id
            .ok_or_else(|| AppError::unauthorized("invalid_grant", "The device code is incomplete."))?;
        let session_id: Uuid = code
            .session_id
            .ok_or_else(|| AppError::unauthorized("invalid_grant", "The device code is incomplete."))?;

        let scopes = code.scope.join(" ");

        crate::domains::oauth::policies_eval::ensure_client_policy(
            db,
            &code.client_id,
            Some(tenant_id),
            organization_id,
            Some(workspace_id),
            &scopes,
            code.audience.as_deref(),
            &code.resource_indicators,
        )
        .await?;

        let assurance = crate::domains::oauth::assurance::resolve_assurance_context(
            db,
            redis,
            principal_id,
            Some(session_id),
            Some(&code.client_id),
            Some(tenant_id),
            organization_id,
            Some(workspace_id),
        )
        .await?;
        if !assurance.sufficient {
            return Err(AppError::forbidden(
                "assurance_level_insufficient",
                "The approving session assurance level is insufficient for this client/workspace context.",
            ));
        }

        let workspace_region = workspace_port::get_workspace(Some(tenant_id), workspace_id, principal_id)
            .await?
            .data_region;

        let token_pair = jwt.generate_token_pair_with_session(
            principal_id,
            Some(workspace_id),
            workspace_region,
            &scopes,
            Some(session_id),
            Some(tenant_id),
            organization_id,
            Some(&assurance.acr),
            Some(assurance.amr.clone()),
            Some(&code.client_id),
            Some(assurance.auth_time),
            None,
        )?;

        refresh_store::store_refresh_token(
            redis,
            &refresh_store::CachedRefreshToken {
                jti: token_pair.refresh_jti.clone(),
                session_id: token_pair.session_id,
                principal_id,
                tenant_id: Some(tenant_id),
                organization_id,
                workspace_id: Some(workspace_id),
                client_id: Some(client_uuid),
                scope: scopes.clone(),
                authorization_details: Vec::new(),
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

        delete_device_code(redis, &code).await?;

        let audience = code.audience.clone();

        Ok(TokenView {
            access_token: token_pair.access_token,
            token_type: token_pair.token_type,
            expires_in: token_pair.expires_in,
            refresh_token: Some(token_pair.refresh_token),
            id_token: None,
            scope: audience.map_or(scopes.clone(), |value| format!("{scopes} audience:{value}")),
            authorization_details: Vec::new(),
            issued_token_type: None,
        })
    }
    .await;

    let release_result = nvbes_redis::lock::release(redis, &lock_key)
        .await
        .map_err(|err| AppError::internal("device_code_lock_failed", format!("{}", err)));
    release_result?;

    result
}
