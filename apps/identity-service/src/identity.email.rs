use nvbes_email::{
    EmailCategory, EmailClient, EmailCommand, EmailIdempotencyKey, EmailRecipient,
    EmailRequestContext, EmailTemplate,
};

use crate::auth::RecoveryNotification;

pub async fn deliver_recovery(
    client: &EmailClient,
    recovery_base_url: &str,
    recovery: RecoveryNotification,
) -> anyhow::Result<()> {
    let command = recovery_command(recovery_base_url, &recovery)?;
    client.send(command).await?;
    Ok(())
}

pub(crate) fn recovery_command(
    recovery_base_url: &str,
    recovery: &RecoveryNotification,
) -> anyhow::Result<EmailCommand> {
    let separator = if recovery_base_url.contains('?') {
        '&'
    } else {
        '?'
    };
    let reset_url = format!("{recovery_base_url}{separator}token={}", recovery.token);
    let correlation_id = recovery.challenge_id.to_string();
    let command = EmailCommand {
        context: EmailRequestContext {
            request_id: correlation_id.clone(),
            correlation_id,
            actor_principal_id: recovery.principal_id.to_string(),
        },
        producer: "identity-service".into(),
        idempotency_key: EmailIdempotencyKey::new(format!(
            "identity:recovery:{}",
            recovery.challenge_id
        ))?,
        recipient: EmailRecipient {
            email: recovery.email.clone(),
            name: None,
        },
        category: EmailCategory::Credential,
        template: EmailTemplate::PasswordResetV1 {
            user_name: "nvbes user".into(),
            reset_url,
            credential_expires_at: recovery.expires_at,
        },
        deliver_before: recovery.expires_at,
    };
    command.validate(chrono::Utc::now())?;
    Ok(command)
}

#[cfg(test)]
#[path = "identity.email.tests.rs"]
mod tests;
