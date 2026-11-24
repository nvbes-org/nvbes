use super::*;
use crate::{
    EmailError, EmailIdempotencyKey, EmailRecipient, EmailRequestContext, EmailTemplate,
    MockEmailSender, SendResult,
};
use chrono::Duration;

fn from() -> EmailAddress {
    EmailAddress {
        email: "no-reply@notify.nvbes.eu".into(),
        name: Some("nvbes".into()),
    }
}

fn reset_command() -> EmailCommand {
    let deliver_before = Utc::now() + Duration::minutes(30);
    let template = EmailTemplate::PasswordResetV1 {
        user_name: "Ada".into(),
        reset_url: "https://id.nvbes.fr/password-recovery#token".into(),
        credential_expires_at: deliver_before,
    };
    EmailCommand {
        context: EmailRequestContext {
            request_id: "request-1".into(),
            correlation_id: "correlation-1".into(),
            actor_principal_id: "principal-1".into(),
        },
        producer: "identity-service".into(),
        idempotency_key: EmailIdempotencyKey::new("email:test:1").unwrap(),
        recipient: EmailRecipient {
            email: "ada@example.com".into(),
            name: Some("Ada".into()),
        },
        category: template.category(),
        template,
        deliver_before,
    }
}

struct Failing(fn() -> EmailError);

#[async_trait]
impl EmailSender for Failing {
    async fn send_message(&self, _message: &EmailMessage) -> Result<SendResult, EmailError> {
        Err((self.0)())
    }
}

#[tokio::test]
async fn renders_and_sends_the_command_to_its_recipient() {
    let mock = Arc::new(MockEmailSender::new());
    let delivery = DirectEmailDelivery::new(mock.clone(), from());
    let command = reset_command();
    let deadline = command.deliver_before;

    let receipt = delivery.send(command).await.expect("accepted");

    assert_eq!(receipt.deliver_before, deadline);
    assert!(!receipt.duplicate);
    assert!(receipt.message_id.starts_with("<account-job-"));
    let sent = mock.sent_emails().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to[0].email, "ada@example.com");
    assert_eq!(sent[0].subject, reset_command().template.render().subject);
    assert_eq!(sent[0].headers[0].0, JOB_ID_HEADER);
}

#[tokio::test]
async fn invalid_commands_are_never_sent() {
    let mock = Arc::new(MockEmailSender::new());
    let delivery = DirectEmailDelivery::new(mock.clone(), from());
    let mut command = reset_command();
    command.deliver_before = Utc::now() - Duration::minutes(1);

    assert!(matches!(
        delivery.send(command).await,
        Err(EmailClientError::InvalidCommand(_))
    ));
    assert_eq!(mock.sent_count().await, 0);
}

#[tokio::test]
async fn transient_failures_are_retryable_and_permanent_ones_final() {
    let transient = DirectEmailDelivery::new(
        Arc::new(Failing(|| EmailError::Api {
            status: 503,
            message: "busy".into(),
        })),
        from(),
    );
    assert!(matches!(
        transient.send(reset_command()).await,
        Err(EmailClientError::Unavailable)
    ));

    let permanent = DirectEmailDelivery::new(
        Arc::new(Failing(|| EmailError::Api {
            status: 422,
            message: "bad recipient".into(),
        })),
        from(),
    );
    assert!(matches!(
        permanent.send(reset_command()).await,
        Err(EmailClientError::Rejected)
    ));
}
