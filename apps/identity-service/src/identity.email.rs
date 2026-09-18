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
    let mut reset = reqwest::Url::parse(recovery_base_url)?;
    anyhow::ensure!(
        reset.scheme() == "https"
            && reset.username().is_empty()
            && reset.password().is_none()
            && reset.query().is_none()
            && reset.fragment().is_none(),
        "invalid recovery destination"
    );
    reset.set_fragment(Some(&format!("token={}", recovery.token)));
    let reset_url = reset.to_string();
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
