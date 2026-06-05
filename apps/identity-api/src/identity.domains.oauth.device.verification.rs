use axum::http::HeaderMap;
use chrono::Utc;
use sqlx::Row;

use super::device_codes::{
    CachedDeviceCode, get_device_code_by_user_code, is_expired, save_device_code,
};
use super::device_validation::{
    DeviceApprovalContextInput, enforce_device_action_rate_limit_db,
    ensure_device_approval_context, ensure_device_deny_context,
};
use super::service::types::{DeviceApprovalInput, DeviceVerificationInput, DeviceVerificationView};
use crate::app::AppState;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

async fn load_active_device_code(
    state: &AppState,
    user_code: &str,
) -> Result<CachedDeviceCode, AppError> {
    let code = get_device_code_by_user_code(&state.redis, user_code)
        .await?
        .ok_or_else(|| AppError::not_found("invalid_code", "The user code is invalid."))?;

    if is_expired(code.expires_at) {
        return Err(AppError::bad_request(
            "code_expired",
            "The user code has expired.",
        ));
    }

    let client = sqlx::query(
        r#"
        SELECT revoked_at
        FROM oauth_clients
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(code.client_uuid)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("client_not_found", "The OAuth client was not found."))?;

    if client
        .get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
        .is_some()
    {
        return Err(AppError::not_found(
            "client_not_found",
            "The OAuth client was not found.",
        ));
    }

    Ok(code)
}

pub async fn verify_device_code(
    state: &AppState,
    _headers: &HeaderMap,
    input: DeviceVerificationInput,
) -> Result<DeviceVerificationView, AppError> {
    enforce_device_action_rate_limit_db(
        &state.redis,
        "verify_device_code",
        uuid::Uuid::nil(),
        &input.user_code,
    )
    .await?;

    let code = load_active_device_code(state, &input.user_code).await?;

    if code.approved_at.is_some() || code.denied_at.is_some() {
        return Err(AppError::bad_request(
            "code_already_processed",
            "This code has already been used.",
        ));
    }

    Ok(DeviceVerificationView {
        client_name: code.client_name,
        scope: code.scope,
        tenant_id: code.tenant_id,
    })
}

pub async fn approve_device_code(
    state: &AppState,
    auth: &AuthContext,
    input: DeviceApprovalInput,
) -> Result<(), AppError> {
    let mut code = load_active_device_code(state, &input.user_code).await?;

    let lock_key = format!("oauth-device-code:{}", code.device_code);
    let locked = nvbes_redis::lock::acquire(&state.redis, &lock_key, 15)
        .await
        .map_err(|err| AppError::internal("device_code_lock_failed", &format!("{}", err)))?;
    if !locked {
        return Err(AppError::conflict(
            "device_code_locked",
            "The device code is currently being processed.",
        ));
    }

    let result = async {
        code = load_active_device_code(state, &input.user_code).await?;

        if code.approved_at.is_some() || code.denied_at.is_some() {
            return Err(AppError::bad_request(
                "code_already_processed",
                "This code has already been used.",
            ));
        }

        let approval_context = ensure_device_approval_context(
            &state.db,
            &state.redis,
            DeviceApprovalContextInput {
                auth,
                client_id: &code.client_id,
                client_uuid: code.client_uuid,
                client_tenant_id: code.tenant_id,
                workspace_id: input.workspace_id,
                organization_id: input.organization_id,
                scopes: &code.scope.join(" "),
                audience: code.audience.as_deref(),
                resource_indicators: &code.resource_indicators,
                consent_action: input.consent_action.as_deref(),
            },
        )
        .await?;

        code.principal_id = Some(auth.user_id);
        code.session_id = Some(auth.session_id);
        code.tenant_id = approval_context.tenant_id;
        code.organization_id = approval_context.organization_id;
        code.workspace_id = Some(input.workspace_id);
        code.approved_at = Some(Utc::now());
        save_device_code(&state.redis, &code).await?;

        Ok(())
    }
    .await;

    let release_result = nvbes_redis::lock::release(&state.redis, &lock_key)
        .await
        .map_err(|err| AppError::internal("device_code_lock_failed", &format!("{}", err)));
    release_result?;

    result
}

pub async fn deny_device_code(
    state: &AppState,
    auth: &AuthContext,
    input: DeviceVerificationInput,
) -> Result<(), AppError> {
    let mut code = load_active_device_code(state, &input.user_code).await?;

    let lock_key = format!("oauth-device-code:{}", code.device_code);
    let locked = nvbes_redis::lock::acquire(&state.redis, &lock_key, 15)
        .await
        .map_err(|err| AppError::internal("device_code_lock_failed", &format!("{}", err)))?;
    if !locked {
        return Err(AppError::conflict(
            "device_code_locked",
            "The device code is currently being processed.",
        ));
    }

    let result = async {
        code = load_active_device_code(state, &input.user_code).await?;

        if code.approved_at.is_some() || code.denied_at.is_some() {
            return Err(AppError::bad_request(
                "code_already_processed",
                "This code has already been used.",
            ));
        }

        ensure_device_deny_context(&state.redis, auth, &code.client_id).await?;

        code.principal_id = Some(auth.user_id);
        code.denied_at = Some(Utc::now());
        save_device_code(&state.redis, &code).await?;

        Ok(())
    }
    .await;

    let release_result = nvbes_redis::lock::release(&state.redis, &lock_key)
        .await
        .map_err(|err| AppError::internal("device_code_lock_failed", &format!("{}", err)));
    release_result?;

    result
}
