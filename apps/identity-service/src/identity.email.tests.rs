use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::auth::RecoveryNotification;

use super::recovery_command;

#[test]
fn recovery_command_is_bounded_correlated_and_contains_no_password() {
    let recovery = RecoveryNotification {
        challenge_id: Uuid::new_v4(),
        principal_id: Uuid::new_v4(),
        email: "person@example.com".into(),
        token: "opaque_recovery_token".into(),
        expires_at: Utc::now() + Duration::minutes(15),
    };
    let command = recovery_command("https://identity.nvbes.eu/recover", &recovery).unwrap();

    assert_eq!(command.producer, "identity-service");
    assert_eq!(
        command.context.correlation_id,
        recovery.challenge_id.to_string()
    );
    assert_eq!(command.deliver_before, recovery.expires_at);
    let serialized = serde_json::to_string(&command).unwrap();
    assert!(serialized.contains("opaque_recovery_token"));
    assert!(!serialized.contains("password"));
}

#[test]
fn recovery_command_rejects_a_non_https_destination() {
    let recovery = RecoveryNotification {
        challenge_id: Uuid::new_v4(),
        principal_id: Uuid::new_v4(),
        email: "person@example.com".into(),
        token: "opaque_recovery_token".into(),
        expires_at: Utc::now() + Duration::minutes(15),
    };
    assert!(recovery_command("http://identity.test/recover", &recovery).is_err());
}
