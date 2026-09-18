use redis::AsyncCommands;

use crate::connection::{RedisError, RedisPool};

const PUBSUB_PREFIX: &str = "nvbes:pubsub";

pub async fn publish_event(
    pool: &RedisPool,
    channel: &str,
    message: &str,
) -> Result<i32, RedisError> {
    let full_channel = format!("{PUBSUB_PREFIX}:{channel}");
    let mut conn = pool.get().await?;
    let count: i32 = conn.publish(&full_channel, message).await?;
    Ok(count)
}

pub async fn publish_session_revoked(
    pool: &RedisPool,
    principal_id: &str,
) -> Result<(), RedisError> {
    publish_event(pool, "session:revoked", principal_id).await?;
    Ok(())
}

pub async fn publish_workspace_deleted(
    pool: &RedisPool,
    workspace_id: &str,
) -> Result<(), RedisError> {
    publish_event(pool, "workspace:deleted", workspace_id).await?;
    Ok(())
}

pub async fn publish_user_suspended(pool: &RedisPool, user_id: &str) -> Result<(), RedisError> {
    publish_event(pool, "user:suspended", user_id).await?;
    Ok(())
}

pub async fn publish_cache_invalidate(
    pool: &RedisPool,
    cache_type: &str,
    key: &str,
) -> Result<(), RedisError> {
    let message = format!("{cache_type}:{key}");
    publish_event(pool, "cache:invalidate", &message).await?;
    Ok(())
}

pub async fn publish_workspace_updated(
    pool: &RedisPool,
    workspace_id: &str,
) -> Result<(), RedisError> {
    publish_event(pool, "workspace:updated", workspace_id).await?;
    Ok(())
}

pub async fn publish_workspace_plan_updated(
    pool: &RedisPool,
    workspace_id: &str,
    plan_code: &str,
) -> Result<(), RedisError> {
    let payload = serde_json::json!({
        "workspace_id": workspace_id,
        "plan_code": plan_code,
    })
    .to_string();
    publish_event(pool, "workspace:plan_updated", &payload).await?;
    Ok(())
}

pub async fn publish_rate_limit_triggered(pool: &RedisPool, ip: &str) -> Result<(), RedisError> {
    publish_event(pool, "ratelimit:triggered", ip).await?;
    Ok(())
}
