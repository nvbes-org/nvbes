use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use sqlx::Row;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use super::device_validation::enforce_device_authorization_rate_limit_db;
use super::logic::generate_user_code;
use super::service::types::{DeviceAuthorizationInput, DeviceAuthorizationView};
use crate::domains::auth::jwt::JwtService;
use crate::domains::oauth::device_codes::{CachedDeviceCode, store_device_code};
use crate::http::error::AppError;

pub async fn device_authorization(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    _jwt: &JwtService,
    web_base_url: &str,
    headers: &HeaderMap,
    input: DeviceAuthorizationInput,
) -> Result<DeviceAuthorizationView, AppError> {
    enforce_device_authorization_rate_limit_db(redis, headers, &input.client_id).await?;

    let client = sqlx::query(
        r#"
        SELECT id, client_id, name, client_type::text as client_type, tenant_id, revoked_at
        FROM oauth_clients
        WHERE client_id = $1
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
        return Err(AppError::not_found(
            "client_not_found",
            "The OAuth client was not found.",
        ));
    }

    let client_uuid: Uuid = client.get("id");
    let tenant_id: Uuid = client.get("tenant_id");
    let client_name: String = client.get("name");

    let device_code = format!("gxdc_{}", Uuid::new_v4().simple());
    let user_code = generate_user_code();

    let expires_in = 300;
    let expires_at = Utc::now() + Duration::seconds(expires_in);
    let interval = 5_i64;

    let verification_uri = format!("{}/activate", web_base_url);
    let verification_uri_complete =
        Some(format!("{}/activate?user_code={}", web_base_url, user_code));

    store_device_code(
        redis,
        &CachedDeviceCode {
            device_code: device_code.clone(),
            user_code: user_code.clone(),
            client_uuid,
            client_id: input.client_id.clone(),
            client_name,
            tenant_id,
            scope: input.scope.split_whitespace().map(String::from).collect(),
            audience: input.audience.clone(),
            resource_indicators: input.resource_indicators.clone(),
            verification_uri: verification_uri.clone(),
            verification_uri_complete: verification_uri_complete.clone(),
            expires_at,
            interval_seconds: interval as i32,
            principal_id: None,
            session_id: None,
            organization_id: None,
            workspace_id: None,
            approved_at: None,
            denied_at: None,
            last_polled_at: None,
        },
    )
    .await?;

    Ok(DeviceAuthorizationView {
        device_code,
        user_code,
        verification_uri,
        verification_uri_complete,
        expires_in,
        interval,
    })
}
