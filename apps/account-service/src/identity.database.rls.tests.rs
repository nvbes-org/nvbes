#[path = "identity.database.rls.tests.bootstrap.rs"]
mod bootstrap;
#[path = "identity.database.rls.tests.catalog.rs"]
mod catalog;
#[path = "identity.database.rls.tests.gaps.rs"]
mod gaps;
#[path = "identity.database.rls.tests.support.rs"]
mod support;

use anyhow::ensure;
use sqlx::{Executor, Row};
use support::{RlsSchema, RlsTestDatabase};
use uuid::Uuid;

#[tokio::test]
async fn security_migration_rls_baseline_and_hardened_reference_are_executable() {
    bootstrap::ensure_redis_is_ready().await;

    let baseline = RlsTestDatabase::create(RlsSchema::ProductionBaseline)
        .await
        .expect("isolated baseline PostgreSQL database should be created");
    let baseline_result = async {
        gaps::assert_documented_production_gaps(&baseline.owner).await?;
        bootstrap::assert_existing_owner_bootstrap(&baseline).await
    }
    .await;
    let baseline_cleanup = baseline.cleanup().await;

    baseline_result.expect("production gaps and existing owner bootstrap must remain explicit");
    baseline_cleanup.expect("isolated baseline database and role should be removed");

    let reference = RlsTestDatabase::create(RlsSchema::HardenedReference)
        .await
        .expect("isolated hardened-reference PostgreSQL database should be created");
    let reference_result = exercise_reference_contract(&reference).await;
    let reference_cleanup = reference.cleanup().await;

    reference_result.expect("test-only runtime RLS reference contract must hold");
    reference_cleanup.expect("isolated reference database and role should be removed");
}

async fn exercise_reference_contract(database: &RlsTestDatabase) -> anyhow::Result<()> {
    catalog::assert_runtime_role_is_restricted(&database.runtime).await?;
    catalog::assert_contract(&database.owner).await?;

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let principal_a = Uuid::new_v4();
    let principal_b = Uuid::new_v4();
    seed_tenant_device(&database.owner, tenant_a, principal_a, "tenant-a").await?;
    seed_tenant_device(&database.owner, tenant_b, principal_b, "tenant-b").await?;
    seed_tenant_device(
        &database.owner,
        Uuid::nil(),
        Uuid::new_v4(),
        "system-tenant",
    )
    .await?;

    let role_scoped_runtime = database.connect_role_scoped_runtime().await?;
    let initial_tenant_context: String =
        sqlx::query_scalar("SELECT current_setting('nvbes.tenant_id', true)")
            .fetch_one(&role_scoped_runtime)
            .await?;
    ensure!(
        initial_tenant_context == Uuid::nil().to_string(),
        "the documented runtime-pool sentinel changed; review its system-tenant collision"
    );
    ensure!(
        visible_device_tenants(&role_scoped_runtime).await? == vec![Uuid::nil()],
        "the documented UUID-nil sentinel collision changed; review runtime compatibility"
    );
    role_scoped_runtime.close().await;

    sqlx::query("SELECT set_config('nvbes.tenant_id', '', false)")
        .execute(&database.runtime)
        .await?;
    sqlx::query("SELECT set_config('nvbes.principal_id', '', false)")
        .execute(&database.runtime)
        .await?;
    ensure!(
        visible_device_tenants(&database.runtime).await?.is_empty(),
        "missing context must deny access, including UUID-nil system tenant rows"
    );

    let mut tenant_a_tx = database.runtime.begin().await?;
    nvbes_tenancy::set_transaction_rls_context(
        &mut tenant_a_tx,
        nvbes_tenancy::RlsContext {
            principal_id: Some(principal_a),
            tenant_id: Some(tenant_a),
            ..Default::default()
        },
    )
    .await?;
    ensure!(
        visible_device_tenants(&mut *tenant_a_tx).await? == vec![tenant_a],
        "tenant A must only read tenant A"
    );
    ensure!(
        visible_session_tenants(&mut *tenant_a_tx).await? == vec![tenant_a],
        "session reads must match both tenant A and principal A"
    );
    let same_tenant_principal = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (
            id, tenant_id, principal_kind, status, display_name
         ) VALUES ($1, $2, 'device', 'active', 'allowed device')",
    )
    .bind(same_tenant_principal)
    .bind(tenant_a)
    .execute(&mut *tenant_a_tx)
    .await?;
    sqlx::query(
        "INSERT INTO devices (
            principal_id, tenant_id, device_kind, device_identifier, display_name
         ) VALUES ($1, $2, 'desktop', $3, 'allowed')",
    )
    .bind(same_tenant_principal)
    .bind(tenant_a)
    .bind(format!("allowed-{}", Uuid::new_v4()))
    .execute(&mut *tenant_a_tx)
    .await?;
    let forbidden_write = sqlx::query(
        "INSERT INTO devices (
            principal_id, tenant_id, device_kind, device_identifier, display_name
         ) VALUES ($1, $2, 'desktop', $3, 'forbidden')",
    )
    .bind(Uuid::new_v4())
    .bind(tenant_b)
    .bind(format!("forbidden-{}", Uuid::new_v4()))
    .execute(&mut *tenant_a_tx)
    .await;
    ensure!(
        forbidden_write
            .as_ref()
            .err()
            .and_then(sqlx::Error::as_database_error)
            .and_then(|error| error.code())
            .as_deref()
            == Some("42501"),
        "cross-tenant write must fail with insufficient_privilege"
    );
    tenant_a_tx.rollback().await?;

    ensure!(
        visible_device_tenants(&database.runtime).await?.is_empty(),
        "rollback must not leak tenant A context into the pooled connection"
    );

    let mut tenant_b_tx = database.runtime.begin().await?;
    nvbes_tenancy::set_transaction_rls_context(
        &mut tenant_b_tx,
        nvbes_tenancy::RlsContext {
            principal_id: Some(principal_b),
            tenant_id: Some(tenant_b),
            ..Default::default()
        },
    )
    .await?;
    ensure!(
        visible_device_tenants(&mut *tenant_b_tx).await? == vec![tenant_b],
        "tenant B must only read tenant B"
    );
    ensure!(
        visible_session_tenants(&mut *tenant_b_tx).await? == vec![tenant_b],
        "session reads must match both tenant B and principal B"
    );
    tenant_b_tx.commit().await?;

    ensure!(
        visible_device_tenants(&database.runtime).await?.is_empty(),
        "commit must not leak tenant B context into the pooled connection"
    );

    let system_policy_write =
        sqlx::query("UPDATE system_policies SET max_share_link_ttl_days = 31")
            .execute(&database.runtime)
            .await?;
    ensure!(
        system_policy_write.rows_affected() == 0,
        "system_policy_read must never permit runtime writes"
    );
    Ok(())
}

async fn seed_tenant_device(
    owner: &sqlx::PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
    slug: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status)
         VALUES ($1, 'personal', $2, $3, 'active')",
    )
    .bind(tenant_id)
    .bind(slug)
    .bind(slug)
    .execute(owner)
    .await?;
    sqlx::query(
        "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
         VALUES ($1, $2, 'human', 'active', $3)",
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(slug)
    .execute(owner)
    .await?;
    sqlx::query(
        "INSERT INTO devices (
            principal_id, tenant_id, device_kind, device_identifier, display_name
         ) VALUES ($1, $2, 'desktop', $3, $4)",
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(format!("device-{principal_id}"))
    .bind(slug)
    .execute(owner)
    .await?;
    sqlx::query(
        "INSERT INTO user_sessions (
            session_id, principal_id, tenant_id, expires_at
         ) VALUES ($1, $2, $3, NOW() + INTERVAL '1 hour')",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .bind(tenant_id)
    .execute(owner)
    .await?;
    Ok(())
}

async fn visible_device_tenants<'e, E>(executor: E) -> anyhow::Result<Vec<Uuid>>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let rows = sqlx::query("SELECT tenant_id FROM devices ORDER BY tenant_id")
        .fetch_all(executor)
        .await?;
    Ok(rows.into_iter().map(|row| row.get("tenant_id")).collect())
}

async fn visible_session_tenants<'e, E>(executor: E) -> anyhow::Result<Vec<Uuid>>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let rows = sqlx::query("SELECT tenant_id FROM user_sessions ORDER BY tenant_id")
        .fetch_all(executor)
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| row.get("tenant_id"))
        .collect())
}
