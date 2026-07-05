use super::revocations::RevokedOAuthClient;
use crate::http::error::AppError;

pub async fn revoke_oauth_client(
    redis: &nvbes_redis::RedisPool,
    client: &RevokedOAuthClient,
) -> Result<(), AppError> {
    nvbes_redis::refresh_token::revoke_all_client_refresh_tokens(redis, client.id)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", err.to_string()))?;
    nvbes_redis::par::revoke_pushed_authorization_requests_for_client(redis, &client.client_id)
        .await
        .map_err(|err| {
            AppError::internal(
                "pushed_authorization_request_revoke_failed",
                err.to_string(),
            )
        })?;
    crate::domains::oauth::authorization_codes::revoke_authorization_codes_for_client(
        redis,
        &client.client_id,
    )
    .await?;
    crate::domains::oauth::device_codes::revoke_device_codes_for_client(redis, &client.client_id)
        .await?;
    Ok(())
}
