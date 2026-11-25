use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::collections::HashMap;

fn lookup(values: &[(&str, String)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect();
    move |name| map.get(name).cloned()
}

#[test]
fn names_round_trip_and_only_batches_recur() {
    for task in WorkerTask::ALL {
        assert_eq!(task.name().parse::<WorkerTask>(), Ok(task));
        assert_eq!(task.to_string(), task.name());
        assert_eq!(task.interval_variable().is_some(), task.recurring());
    }
    assert!(!WorkerTask::Migrate.recurring());
    assert!(!WorkerTask::RotateMfaKey.recurring());
    assert_eq!(
        "billing-outbox".parse::<WorkerTask>(),
        Err(UnknownTask("billing-outbox".into()))
    );
}

#[test]
fn tasks_without_secrets_prepare_from_nothing() {
    for task in [WorkerTask::Migrate, WorkerTask::PrivacyRetention] {
        let prepared = PreparedTask::prepare(task, &lookup(&[]), false).unwrap();
        assert_eq!(prepared.task(), task);
    }
}

#[test]
fn mfa_tasks_never_invent_a_key() {
    for task in [WorkerTask::IdentityEmails, WorkerTask::RotateMfaKey] {
        assert!(matches!(
            PreparedTask::prepare(task, &lookup(&[]), true),
            Err(ConfigError::Invalid(_))
        ));
    }
    let key = [(
        "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
        STANDARD.encode([3u8; 32]),
    )];
    let prepared = PreparedTask::prepare(WorkerTask::IdentityEmails, &lookup(&key), true).unwrap();
    assert_eq!(prepared.task(), WorkerTask::IdentityEmails);
}

/// A fresh database migrates twice without error, then the account schema
/// serves a privacy batch, proving each domain migrated into its own schema.
#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = false)]
async fn migrate_is_idempotent_and_prepares_the_domain_schemas(pool: PgPool) {
    let migrate = PreparedTask::prepare(WorkerTask::Migrate, &lookup(&[]), true).unwrap();
    migrate.execute(&pool).await.unwrap();
    migrate.execute(&pool).await.unwrap();

    let schemas: Vec<String> = sqlx::query_scalar(
        "SELECT table_schema FROM information_schema.tables WHERE table_name='_sqlx_migrations' ORDER BY table_schema",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(schemas, vec!["account".to_owned(), "identity".to_owned()]);

    // The API and worker runtimes resolve domain tables through this path.
    let runtime = sqlx::postgres::PgPoolOptions::new()
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET search_path TO identity, account, public")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let privacy = PreparedTask::prepare(WorkerTask::PrivacyRetention, &lookup(&[]), true).unwrap();
    let summary = privacy.execute(&runtime).await.unwrap();
    assert_eq!(summary["exports_completed"], 0);
}
