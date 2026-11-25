use super::*;
use std::collections::HashMap;

fn lookup(values: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    move |name| map.get(name).cloned()
}

#[test]
fn development_defaults_to_local_smtp() {
    assert_eq!(provider(&lookup(&[]), true), Ok(Provider::Smtp));
    assert!(from_lookup(&lookup(&[]), true).is_ok());
}

#[test]
fn production_names_the_provider_and_the_sender() {
    assert_eq!(
        provider(&lookup(&[]), false),
        Err(ConfigError::Missing("NVBES_EMAIL_PROVIDER"))
    );
    assert!(matches!(
        provider(&lookup(&[("NVBES_EMAIL_PROVIDER", "sendmail")]), true),
        Err(ConfigError::Invalid(_))
    ));
    assert!(matches!(
        from_lookup(&lookup(&[("NVBES_EMAIL_PROVIDER", "smtp")]), false),
        Err(ConfigError::Missing("NVBES_EMAIL_FROM_EMAIL"))
    ));
}

#[test]
fn production_smtp_requires_starttls_and_paired_credentials() {
    let base = [
        ("NVBES_EMAIL_PROVIDER", "smtp"),
        ("NVBES_EMAIL_FROM_EMAIL", "no-reply@notify.nvbes.eu"),
        ("NVBES_SMTP_HOST", "smtp.example.net"),
        ("NVBES_SMTP_PORT", "587"),
    ];
    assert!(matches!(
        from_lookup(&lookup(&base), false),
        Err(ConfigError::Invalid(_))
    ));
    let mut with_user = base.to_vec();
    with_user.extend([
        ("NVBES_SMTP_STARTTLS", "true"),
        ("NVBES_SMTP_USERNAME", "nvbes"),
    ]);
    assert!(matches!(
        from_lookup(&lookup(&with_user), false),
        Err(ConfigError::Invalid(_))
    ));
    with_user.push(("NVBES_SMTP_PASSWORD", "secret"));
    assert!(from_lookup(&lookup(&with_user), false).is_ok());
}

#[test]
fn scaleway_requires_its_credentials() {
    let values = [
        ("NVBES_EMAIL_PROVIDER", "scaleway-tem"),
        ("NVBES_EMAIL_FROM_EMAIL", "no-reply@notify.nvbes.eu"),
    ];
    assert!(matches!(
        from_lookup(&lookup(&values), false),
        Err(ConfigError::Missing("NVBES_SCALEWAY_EMAIL_SECRET_KEY"))
    ));
    let mut complete = values.to_vec();
    complete.extend([
        ("NVBES_SCALEWAY_EMAIL_SECRET_KEY", "secret"),
        ("NVBES_SCALEWAY_EMAIL_PROJECT_ID", "project"),
    ]);
    assert!(from_lookup(&lookup(&complete), false).is_ok());
}

#[test]
fn test_capture_exists_only_in_development() {
    let values = [("NVBES_EMAIL_PROVIDER", "test-capture")];
    assert_eq!(provider(&lookup(&values), true), Ok(Provider::TestCapture));
    assert!(matches!(
        provider(&lookup(&values), false),
        Err(ConfigError::Invalid(_))
    ));
    assert!(matches!(
        from_lookup(&lookup(&values), true),
        Err(ConfigError::Missing("NVBES_EMAIL_TEST_CAPTURE_DIR"))
    ));
}
