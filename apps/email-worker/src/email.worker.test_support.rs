use std::{collections::HashMap, net::SocketAddr};

#[cfg(feature = "database-tests")]
use chrono::{Duration, Utc};
#[cfg(feature = "database-tests")]
use nvbes_email::{
    AccountSecurityEvent, EmailCategory, EmailCommand, EmailIdempotencyKey, EmailRecipient,
    EmailRequestContext, EmailTemplate,
    proto::nvbes::{
        email::v1::{EmailCallerContext, EmailOperatorContext},
        platform::v1::RequestContext as ProtoRequestContext,
    },
};
use sqlx::PgPool;
#[cfg(feature = "database-tests")]
use std::path::PathBuf;

use crate::{
    config::{DispatchMode, EmailWorkerConfig, ProviderConfig, RetentionConfig, RuntimeRole},
    state::EmailWorkerState,
};

pub const INTERNAL_TOKEN: &str = "email-worker-internal-token-32-value";

pub fn config(provider: ProviderConfig) -> EmailWorkerConfig {
    EmailWorkerConfig {
        environment: "test".to_string(),
        database_url: "postgres://unused/nvbes_email_test".to_string(),
        http_bind_addr: "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
        producer_tokens: HashMap::from([
            ("identity-service".to_string(), INTERNAL_TOKEN.to_string()),
            ("backoffice-service".to_string(), INTERNAL_TOKEN.to_string()),
        ]),
        data_encryption_key: [7; 32],
        recipient_hmac_key: [9; 32],
        from_email: "no-reply@nvbes.fr".to_string(),
        from_name: "nvbes".to_string(),
        reply_to: Some("support@nvbes.fr".to_string()),
        message_id_domain: "nvbes.fr".to_string(),
        provider,
        dispatch: DispatchMode::InMemory,
        runtime_role: RuntimeRole::All,
        webhook: None,
        retention: RetentionConfig {
            payload_days: 30,
            ledger_days: 400,
        },
    }
}

pub fn state(pool: PgPool) -> EmailWorkerState {
    EmailWorkerState::new(config(ProviderConfig::Mock), pool).unwrap()
}

#[cfg(feature = "database-tests")]
pub fn command(idempotency_key: &str, recipient: &str) -> EmailCommand {
    EmailCommand {
        context: EmailRequestContext {
            request_id: format!("request-{idempotency_key}"),
            correlation_id: format!("correlation-{idempotency_key}"),
            actor_principal_id: "principal-1".to_string(),
        },
        producer: "identity-service".to_string(),
        idempotency_key: EmailIdempotencyKey::new(idempotency_key.to_string()).unwrap(),
        recipient: EmailRecipient {
            email: recipient.to_string(),
            name: Some("Ada".to_string()),
        },
        category: EmailCategory::AccountSecurity,
        template: EmailTemplate::AccountSecurityV1 {
            event: AccountSecurityEvent::AccountRecovered,
            affected_email: None,
            previous_email: None,
            security_url: Some("https://identity.nvbes.fr/security".to_string()),
        },
        deliver_before: Utc::now() + Duration::hours(1),
    }
}

#[cfg(feature = "database-tests")]
pub fn caller(name: &str) -> EmailCallerContext {
    EmailCallerContext {
        request_context: Some(ProtoRequestContext {
            request_id: "request-1".to_string(),
            correlation_id: "correlation-1".to_string(),
            actor_principal_id: "principal-1".to_string(),
            tenant: None,
        }),
        caller: name.to_string(),
    }
}

#[cfg(feature = "database-tests")]
pub fn operator() -> EmailOperatorContext {
    EmailOperatorContext {
        caller: Some(caller("backoffice-service")),
        actor: "00000000-0000-0000-0000-000000000001".to_string(),
        reason: "ticket EMAIL-123 approved".to_string(),
    }
}

#[cfg(feature = "database-tests")]
pub fn authorize<T>(request: &mut tonic::Request<T>) {
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {INTERNAL_TOKEN}").parse().unwrap(),
    );
}

#[cfg(feature = "database-tests")]
pub fn capture_directory(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nvbes-email-worker-{suffix}-{}",
        uuid::Uuid::new_v4()
    ))
}
