use std::process::{Command, Output};

fn run(args: &[&str], database_url: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-billing-service"));
    command.env_clear().args(args);
    apply_development_env(&mut command);
    if let Some(url) = database_url {
        command.env("NVBES_BILLING_DATABASE_URL", url);
    }
    command.output().expect("billing binary runs")
}

fn apply_development_env(command: &mut Command) {
    command
        .env(
            "NVBES_BILLING_DATABASE_URL",
            std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@127.0.0.1:5432/nvbes_billing".into()
            }),
        )
        .env("NVBES_STRIPE_SECRET_KEY", "sk_test_dummy_key_for_testing")
        .env("NVBES_STRIPE_WEBHOOK_SECRET", "whsec_test_dummy_secret")
        .env("NVBES_APP_URL", "https://nvbes.test");
}

#[test]
fn validate_runtime_accepts_development_defaults() {
    let output = run(&["validate-runtime"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("billing runtime configuration is valid"));
}

#[test]
fn unknown_cli_action_reports_usage() {
    let output = run(&["not-a-real-command"], None);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("usage: nvbes-billing-service"), "{error}");
}

#[test]
fn migrate_applies_billing_schema() {
    let output = run(&["migrate"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("billing database migrations applied"));
}

#[test]
fn check_stripe_mappings_emits_json_report() {
    let output = run(&["check-stripe-mappings"], None);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json report");
    assert!(report.get("plans_checked").is_some());
    if report["failures"].as_array().is_some_and(|f| f.is_empty()) {
        assert!(output.status.success());
    } else {
        assert!(!output.status.success());
    }
}

#[test]
fn synthetic_billing_smoke_runs_end_to_end() {
    let output = run(&["synthetic-billing-smoke"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json smoke");
    assert!(payload["subscription_status"].as_str().is_some());
}

#[test]
fn publish_outbox_reports_zero_when_empty() {
    let output = run(&["publish-outbox"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("published"));
}

#[test]
fn live_stripe_key_is_rejected_at_config_load() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-billing-service"));
    command
        .env_clear()
        .arg("validate-runtime")
        .env("NVBES_STRIPE_SECRET_KEY", "sk_live_forbidden");
    let output = command.output().expect("billing binary runs");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("FINOPS"), "{error}");
}
