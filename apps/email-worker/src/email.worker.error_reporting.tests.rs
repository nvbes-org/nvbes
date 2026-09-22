use super::*;
use crate::{config::ProviderConfig, test_support};

#[test]
fn init_and_smoke_run_without_dsn() {
    let config = test_support::config(ProviderConfig::Mock);
    let _guard = init(&config);
    let smoke = smoke(&config);
    assert_eq!(smoke.app_name, APP_NAME);
    assert!(!smoke.configured);
    assert_eq!(smoke.status, "skipped");
}

#[test]
fn capture_helpers_accept_operation_and_delivery_errors() {
    let config = test_support::config(ProviderConfig::Mock);
    capture_operation(
        &config,
        "retention",
        &std::io::Error::other("retention failed"),
    );
    capture_operation_message(&config, "queue_dispatch", "dispatch failed");
    capture_delivery(
        &config,
        Uuid::nil(),
        "email.deliver",
        1,
        5,
        &std::io::Error::other("provider timeout"),
    );
}
