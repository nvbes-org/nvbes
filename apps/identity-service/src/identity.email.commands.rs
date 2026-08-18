use chrono::{DateTime, Utc};
use uuid::Uuid;

use nvbes_email::{
    EmailCommand, EmailIdempotencyKey, EmailRecipient, EmailRequestContext, EmailTemplate,
};

use crate::http::error::AppError;

pub async fn enqueue(
    redis: &nvbes_redis::RedisPool,
    recipient_email: String,
    recipient_name: Option<String>,
    idempotency_key: String,
    template: EmailTemplate,
    deliver_before: DateTime<Utc>,
    actor_principal_id: Option<Uuid>,
) -> Result<(), AppError> {
    let request_id = Uuid::new_v4().to_string();
    let command = EmailCommand {
        context: EmailRequestContext {
            request_id: request_id.clone(),
            correlation_id: request_id,
            actor_principal_id: actor_principal_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
        },
        producer: "identity-service".to_string(),
        idempotency_key: EmailIdempotencyKey::new(idempotency_key).map_err(|_| {
            AppError::internal(
                "email_idempotency_key_invalid",
                "Email command identity is invalid.",
            )
        })?,
        recipient: EmailRecipient {
            email: recipient_email,
            name: recipient_name,
        },
        category: template.category(),
        template,
        deliver_before,
    };
    command.validate(Utc::now()).map_err(|error| {
        tracing::warn!(
            validation_field = error.field_name(),
            "Identity rejected an invalid email command before enqueue"
        );
        AppError::internal("email_command_invalid", "Email command is invalid.")
    })?;
    nvbes_product_identity::email::jobs::enqueue_email_command(redis, command)
        .await
        .map_err(AppError::from)
}

pub fn verification_url(config: &crate::app::AppConfig, token: &str) -> String {
    format!("{}/verify-result?token={token}", config.web_base_url)
}

pub fn password_reset_url(config: &crate::app::AppConfig, token: &str) -> String {
    format!("{}/reset-password?token={token}", config.web_base_url)
}
