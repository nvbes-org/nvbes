use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::{AccountError, AccountResult};

#[path = "account.privacy.data_export.query.rs"]
mod query;

pub const DATA_EXPORT_TTL_SECONDS: u64 = 24 * 60 * 60;

pub fn account_export_cache_key(principal_id: Uuid) -> String {
    format!("privacy:identity:account_export:{principal_id}")
}

pub async fn build_account_export(
    db: &sqlx::PgPool,
    principal_id: Uuid,
    email_activity: JsonValue,
) -> AccountResult<JsonValue> {
    query::build_account_export(db, principal_id, email_activity).await
}

pub async fn store_account_export(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    export: &JsonValue,
) -> AccountResult<()> {
    nvbes_redis::cache::cache_set_json(
        redis,
        &account_export_cache_key(principal_id),
        export,
        DATA_EXPORT_TTL_SECONDS,
    )
    .await
    .map_err(|error| AccountError::internal("data_export_store_failed", error.to_string()))
}

pub async fn load_account_export(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
) -> AccountResult<Option<JsonValue>> {
    nvbes_redis::cache::cache_get_json(redis, &account_export_cache_key(principal_id))
        .await
        .map_err(|error| AccountError::internal("data_export_load_failed", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{DATA_EXPORT_TTL_SECONDS, account_export_cache_key};
    use uuid::Uuid;

    #[test]
    fn account_export_cache_key_is_subject_scoped() {
        assert_eq!(
            account_export_cache_key(Uuid::nil()),
            "privacy:identity:account_export:00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn account_export_ttl_is_one_day() {
        assert_eq!(DATA_EXPORT_TTL_SECONDS, 86_400);
    }
}
