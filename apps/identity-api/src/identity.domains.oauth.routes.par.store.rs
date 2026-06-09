use crate::http::error::AppError;

pub(crate) async fn resolve_pushed_parameters(
    redis: &nvbes_redis::RedisPool,
    request_uri: &str,
    client_id: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    let record = nvbes_redis::par::get_pushed_authorization_request(redis, request_uri)
        .await
        .map_err(|err| {
            AppError::internal("pushed_authorization_request_read_failed", err.to_string())
        })?
        .ok_or_else(|| {
            AppError::bad_request("invalid_request_uri", "The request_uri is invalid.")
        })?;

    if record.client_id != client_id {
        return Err(AppError::bad_request(
            "invalid_request_uri",
            "The request_uri does not belong to this client.",
        ));
    }

    if record.expires_at < chrono::Utc::now() || record.used_at.is_some() {
        return Err(AppError::bad_request(
            "invalid_request_uri",
            "The request_uri has expired or has already been used.",
        ));
    }

    Ok(record.parameters)
}

pub(crate) async fn mark_par_used(
    redis: &nvbes_redis::RedisPool,
    request_uri: &str,
) -> Result<(), AppError> {
    nvbes_redis::par::mark_pushed_authorization_request_used(redis, request_uri)
        .await
        .map_err(|err| {
            AppError::internal("pushed_authorization_request_mark_failed", err.to_string())
        })?;
    Ok(())
}
