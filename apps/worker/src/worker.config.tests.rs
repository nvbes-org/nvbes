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
fn development_defaults_to_a_local_database() {
    let config = WorkerConfig::from_lookup(&lookup(&[])).unwrap();
    assert!(config.development());
    assert_eq!(config.database_url, "postgres://localhost/nvbes_monolith");
    assert_eq!(config.database_max_connections, 2);
}

#[test]
fn production_requires_a_database_url() {
    assert_eq!(
        WorkerConfig::from_lookup(&lookup(&[("NVBES_ENVIRONMENT", "production")])).err(),
        Some(ConfigError::Missing("DATABASE_URL"))
    );
}

#[test]
fn development_serves_both_recurring_tasks_by_default() {
    let serve = ServeConfig::from_lookup(&lookup(&[]), true).unwrap();
    assert_eq!(serve.bind_addr.to_string(), "127.0.0.1:8081");
    assert_eq!(
        serve.schedules,
        vec![
            Schedule {
                task: WorkerTask::IdentityEmails,
                every: Duration::from_secs(15),
            },
            Schedule {
                task: WorkerTask::PrivacyRetention,
                every: Duration::from_secs(300),
            },
        ]
    );
}

#[test]
fn production_names_its_tasks_and_may_tune_intervals() {
    assert_eq!(
        ServeConfig::from_lookup(&lookup(&[]), false).err(),
        Some(ConfigError::Missing("NVBES_WORKER_TASKS"))
    );
    let serve = ServeConfig::from_lookup(
        &lookup(&[
            ("NVBES_WORKER_TASKS", "privacy-retention"),
            ("NVBES_WORKER_PRIVACY_RETENTION_INTERVAL_SECS", "60"),
            ("PORT", "9000"),
        ]),
        false,
    )
    .unwrap();
    assert_eq!(serve.bind_addr.to_string(), "0.0.0.0:9000");
    assert_eq!(serve.schedules[0].every, Duration::from_secs(60));
}

#[test]
fn serve_rejects_one_shot_unknown_duplicate_or_zero_interval_tasks() {
    for (tasks, interval) in [
        ("migrate", None),
        ("rotate-mfa-key", None),
        ("billing-outbox", None),
        ("identity-emails,identity-emails", None),
        (" , ", None),
        ("identity-emails", Some("0")),
        ("identity-emails", Some("soon")),
    ] {
        let mut values = vec![("NVBES_WORKER_TASKS", tasks)];
        if let Some(interval) = interval {
            values.push(("NVBES_WORKER_IDENTITY_EMAILS_INTERVAL_SECS", interval));
        }
        assert!(
            matches!(
                ServeConfig::from_lookup(&lookup(&values), true),
                Err(ConfigError::Invalid(_))
            ),
            "{tasks} {interval:?}"
        );
    }
}
