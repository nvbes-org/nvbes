use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::domains::auth::types::AuthContext;
use crate::domains::auth::types::StepUpSubject;
use crate::domains::auth::{check_rate_limit, verification};
use crate::http::error::AppError;

pub use nvbes_product_account::auth::data_export::{
    DATA_EXPORT_TTL_SECONDS, account_export_cache_key,
};

pub async fn build_account_export(
    db: &sqlx::PgPool,
    principal_id: Uuid,
) -> Result<JsonValue, AppError> {
    nvbes_product_account::auth::data_export::build_account_export(db, principal_id)
        .await
        .map_err(AppError::from)
}

pub async fn store_account_export(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    export: &JsonValue,
) -> Result<(), AppError> {
    nvbes_product_account::auth::data_export::store_account_export(redis, principal_id, export)
        .await
        .map_err(AppError::from)
}

pub async fn load_account_export(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
) -> Result<Option<JsonValue>, AppError> {
    nvbes_product_account::auth::data_export::load_account_export(redis, principal_id)
        .await
        .map_err(AppError::from)
}

pub async fn request_account_export(
    db: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
) -> Result<(), AppError> {
    check_rate_limit(
        redis,
        "auth_me_export",
        &format!("user:{}", auth.user_id()),
        3,
        std::time::Duration::from_secs(86400),
    )
    .await?;

    verification::require_recent_step_up(redis, auth, None).await?;

    let display_name = auth.display_name.as_str();
    let html_body = format!(
        "<p>Bonjour {},</p><p>Votre demande d'export de donnees personnelles a bien ete enregistree. Une notification vous sera envoyee quand le fichier sera pret. Le fichier devra etre recupere depuis votre session authentifiee.</p><p>L'equipe nvbes</p>",
        display_name
    );

    crate::email::jobs::enqueue_email_job_tx(
        db,
        redis,
        crate::email::jobs::EmailSendPayload {
            to_email: auth.user_email.clone(),
            to_name: Some(display_name.to_string()),
            subject: "Demande d'export de donnees - nvbes".to_string(),
            html_body,
            text_body: None,
            business_type: "data_export".to_string(),
        },
        &format!("export-confirm:{}", auth.user_id()),
    )
    .await?;

    crate::email::jobs::enqueue_data_export_job_tx(redis, auth.user_id(), &auth.user_email)
        .await
        .map_err(AppError::from)
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
