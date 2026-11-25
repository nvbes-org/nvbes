use super::EmailWorkerState;
use crate::{
    config::{ProviderConfig, ScalewayConfig, WebhookTrustConfig},
    test_support,
};

#[sqlx::test(migrations = "./migrations")]
async fn state_constructs_every_provider_and_owns_one_local_receiver(pool: sqlx::PgPool) {
    let mock =
        EmailWorkerState::new(test_support::config(ProviderConfig::Mock), pool.clone()).unwrap();
    assert!(mock.take_local_dispatch_receiver().is_some());
    assert!(mock.take_local_dispatch_receiver().is_none());

    let smtp = ProviderConfig::Smtp(nvbes_email::SmtpEmailConfig {
        host: "127.0.0.1".to_string(),
        port: 1025,
        username: Some("user".to_string()),
        password: Some("password".to_string()),
        starttls: false,
    });
    assert!(EmailWorkerState::new(test_support::config(smtp), pool.clone()).is_ok());

    let capture_directory = test_support::capture_directory("state");
    let capture = ProviderConfig::TestCapture(capture_directory.clone());
    assert!(EmailWorkerState::new(test_support::config(capture), pool.clone()).is_ok());
    std::fs::remove_dir_all(capture_directory).unwrap();

    let scaleway = ProviderConfig::Scaleway(ScalewayConfig {
        secret_key: "secret".to_string(),
        project_id: "project".to_string(),
        region: "fr-par".to_string(),
    });
    assert!(EmailWorkerState::new(test_support::config(scaleway), pool.clone()).is_ok());

    let mut invalid_webhook = test_support::config(ProviderConfig::Mock);
    invalid_webhook.webhook = Some(WebhookTrustConfig {
        topic_arn: "arn:scw:sns:fr-par:test:topic".to_string(),
        ca_bundle_pem: b"not-a-certificate".to_vec(),
        signing_certificate_host: "messaging.example.test".to_string(),
        confirmation_host: "confirmation.example.test".to_string(),
    });
    assert!(EmailWorkerState::new(invalid_webhook, pool).is_err());
}
