//! Isolated schemas in an explicitly local synthetic test database.
use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn test_pool() -> PgPool {
    super::environment::validate_test_environment(std::env::var("NVBES_ENV").ok().as_deref())
        .expect("safe test environment");
    let url = std::env::var("NVBES_SECURITY_TEST_DATABASE_URL")
        .expect("NVBES_SECURITY_TEST_DATABASE_URL is required; PostgreSQL tests cannot be skipped");
    let parsed = reqwest::Url::parse(&url).expect("database URL");
    let local = super::environment::validate_loopback_url(&url, "PostgreSQL").is_ok();
    let run = std::env::var("GITHUB_RUN_ID").unwrap_or_default();
    let attempt = std::env::var("GITHUB_RUN_ATTEMPT").unwrap_or_default();
    let job = std::env::var("GITHUB_JOB").unwrap_or_default();
    let ci_host = format!("nvbes-ci-postgres-{run}-{attempt}-{job}");
    let isolated_ci = std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && !run.is_empty()
        && run.bytes().all(|b| b.is_ascii_digit())
        && !attempt.is_empty()
        && attempt.bytes().all(|b| b.is_ascii_digit())
        && !job.is_empty()
        && job
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && parsed.host_str() == Some(ci_host.as_str());
    assert!(
        local || isolated_ci,
        "only local or exact job-scoped test PostgreSQL is permitted"
    );
    assert!(matches!(parsed.scheme(), "postgres" | "postgresql"));
    assert!(
        parsed.query().is_none() && parsed.fragment().is_none(),
        "connection overrides are forbidden"
    );
    assert!(
        parsed.port().is_some_and(|port| port > 0),
        "explicit test database port required"
    );
    assert!(
        parsed
            .path()
            .trim_start_matches('/')
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "plain test database name required"
    );
    assert!(
        parsed.path().ends_with("_test"),
        "database must end in _test"
    );
    let schema = format!("security_test_{}", uuid::Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("PostgreSQL reachable");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .expect("isolated schema");
    admin.close().await;
    // Schema names contain only a fixed prefix and UUID hex, never user input.
    // The test harness removes the disposable database/container after the run.
    PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |connection, _| {
            let statement = format!("SET search_path TO {schema}");
            Box::pin(async move {
                sqlx::query(&statement).execute(connection).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("isolated PostgreSQL pool")
}
