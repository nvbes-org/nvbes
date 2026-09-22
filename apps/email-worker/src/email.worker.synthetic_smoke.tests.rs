use chrono::TimeZone;
use std::sync::Mutex;

use super::*;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn builds_an_explicit_bounded_operational_message() {
    let now = Utc.with_ymd_and_hms(2026, 8, 25, 20, 0, 0).unwrap();
    let command = command("operator@example.com", "deploy-123", now).unwrap();

    assert_eq!(command.category, EmailCategory::Operational);
    assert_eq!(command.deliver_before, now + Duration::minutes(30));
    let rendered = command.template.render();
    assert!(rendered.subject.contains("no action required"));
    assert!(rendered.text_body.contains("deploy-123"));
    assert!(!rendered.text_body.contains("operator@example.com"));
}

#[test]
fn rejects_an_unbounded_or_unsafe_run_identifier() {
    let now = Utc.with_ymd_and_hms(2026, 8, 25, 20, 0, 0).unwrap();
    assert!(command("operator@example.com", "contains spaces", now).is_err());
}

#[test]
fn cold_start_retry_schedule_is_bounded() {
    assert_eq!(MAX_COLD_START_ATTEMPTS, 5);
    assert_eq!(COLD_START_RETRY_SECONDS, [1, 2, 4, 8]);
    assert_eq!(COLD_START_RETRY_SECONDS.iter().sum::<u64>(), 15);
}

#[test]
fn delivery_poll_is_bounded_and_rejects_terminal_failures() {
    assert_eq!(DELIVERY_POLL_ATTEMPTS, 60);
    assert_eq!(DELIVERY_POLL_SECONDS, 5);
    for state in [
        "hard_bounced",
        "complained",
        "suppressed",
        "expired",
        "dropped",
        "failed",
    ] {
        assert!(is_terminal_failure(state), "{state}");
    }
    assert!(!is_terminal_failure("provider_accepted"));
    assert!(!is_terminal_failure("delivered"));
}

#[test]
fn required_env_rejects_missing_and_blank_values() {
    let _lock = ENV_LOCK.lock().unwrap();
    let name = "NVBES_EMAIL_SYNTHETIC_TEST_ONLY_VAR";
    let previous = std::env::var_os(name);
    unsafe { std::env::remove_var(name) };
    assert!(required_env(name).is_err());
    unsafe { std::env::set_var(name, "   ") };
    assert!(required_env(name).is_err());
    unsafe { std::env::set_var(name, "ok-value") };
    assert_eq!(required_env(name).unwrap(), "ok-value");
    match previous {
        Some(value) => unsafe { std::env::set_var(name, value) },
        None => unsafe { std::env::remove_var(name) },
    }
}
