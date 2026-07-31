use anyhow::ensure;

const TENANT_TABLES_WITHOUT_RLS: [&str; 6] = [
    "devices",
    "saml_assertion_ids",
    "saml_pending_requests",
    "saml_sp_config",
    "security_event_deliveries",
    "user_sessions",
];

pub async fn assert_documented_production_gaps(owner: &sqlx::PgPool) -> anyhow::Result<()> {
    let tenant_tables_without_rls = sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT class.relname::text
        FROM pg_class AS class
        INNER JOIN pg_namespace AS namespace ON namespace.oid = class.relnamespace
        INNER JOIN pg_attribute AS attribute ON attribute.attrelid = class.oid
        WHERE namespace.nspname = 'public'
          AND class.relkind IN ('r', 'p')
          AND attribute.attname = 'tenant_id'
          AND NOT attribute.attisdropped
          AND NOT class.relrowsecurity
        ORDER BY 1
        "#,
    )
    .fetch_all(owner)
    .await?;
    ensure!(
        tenant_tables_without_rls == TENANT_TABLES_WITHOUT_RLS,
        "the documented no-RLS inventory changed; review the production gap and reference contract: \
         {tenant_tables_without_rls:?}"
    );

    let rls_without_force = sqlx::query_scalar::<_, String>(
        r#"
        SELECT class.relname::text
        FROM pg_class AS class
        INNER JOIN pg_namespace AS namespace ON namespace.oid = class.relnamespace
        WHERE namespace.nspname = 'public'
          AND class.relkind IN ('r', 'p')
          AND class.relrowsecurity
          AND NOT class.relforcerowsecurity
        ORDER BY class.relname
        "#,
    )
    .fetch_all(owner)
    .await?;
    for known_gap in ["oauth_clients", "tenants", "workspaces"] {
        ensure!(
            rls_without_force.iter().any(|table| table == known_gap),
            "{known_gap} unexpectedly left the documented non-FORCE inventory; \
             review whether production RLS remediation is now complete"
        );
    }

    let system_policy_command = sqlx::query_scalar::<_, String>(
        "SELECT polcmd::text
         FROM pg_policy
         WHERE polrelid = 'system_policies'::regclass
           AND polname = 'system_policy_read'",
    )
    .fetch_one(owner)
    .await?;
    ensure!(
        system_policy_command == "*",
        "system_policy_read changed; review and update the documented write-policy gap"
    );
    Ok(())
}
