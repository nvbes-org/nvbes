use anyhow::ensure;

const REQUIRED_TABLES: [&str; 6] = [
    "devices",
    "saml_assertion_ids",
    "saml_pending_requests",
    "saml_sp_config",
    "security_event_deliveries",
    "user_sessions",
];

pub async fn assert_contract(owner: &sqlx::PgPool) -> anyhow::Result<()> {
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
        tenant_tables_without_rls.is_empty(),
        "tenant-scoped tables without RLS: {tenant_tables_without_rls:?}"
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
    ensure!(
        rls_without_force.is_empty(),
        "RLS tables without FORCE: {rls_without_force:?}"
    );

    let rls_tables_without_policy = sqlx::query_scalar::<_, String>(
        r#"
        SELECT class.relname::text
        FROM pg_class AS class
        INNER JOIN pg_namespace AS namespace ON namespace.oid = class.relnamespace
        WHERE namespace.nspname = 'public'
          AND class.relkind IN ('r', 'p')
          AND class.relrowsecurity
          AND NOT EXISTS (
              SELECT 1 FROM pg_policy WHERE polrelid = class.oid
          )
        ORDER BY class.relname
        "#,
    )
    .fetch_all(owner)
    .await?;
    ensure!(
        rls_tables_without_policy.is_empty(),
        "RLS tables without a policy: {rls_tables_without_policy:?}"
    );

    for table in REQUIRED_TABLES {
        let policy_commands = sqlx::query_scalar::<_, String>(
            "SELECT policy.polcmd::text
             FROM pg_policy AS policy
             WHERE policy.polrelid = $1::regclass",
        )
        .bind(table)
        .fetch_all(owner)
        .await?;
        ensure!(
            policy_commands == vec!["*"],
            "{table} must have one FOR ALL isolation policy"
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
        system_policy_command == "r",
        "system_policy_read must be SELECT-only"
    );
    Ok(())
}

pub async fn assert_runtime_role_is_restricted(runtime: &sqlx::PgPool) -> anyhow::Result<()> {
    let (
        is_superuser,
        bypasses_rls,
        creates_databases,
        creates_roles,
        inherits_roles,
        replicates,
        owns_database,
        owns_public_table,
    ) = sqlx::query_as::<_, (bool, bool, bool, bool, bool, bool, bool, bool)>(
        r#"
        SELECT role.rolsuper,
               role.rolbypassrls,
               role.rolcreatedb,
               role.rolcreaterole,
               role.rolinherit,
               role.rolreplication,
               database.datdba = role.oid,
               EXISTS (
                   SELECT 1
                   FROM pg_class AS class
                   INNER JOIN pg_namespace AS namespace
                       ON namespace.oid = class.relnamespace
                   WHERE namespace.nspname = 'public'
                     AND class.relkind IN ('r', 'p')
                     AND class.relowner = role.oid
               )
        FROM pg_roles AS role
        CROSS JOIN pg_database AS database
        WHERE role.rolname = current_user
          AND database.datname = current_database()
        "#,
    )
    .fetch_one(runtime)
    .await?;
    ensure!(
        !is_superuser
            && !bypasses_rls
            && !creates_databases
            && !creates_roles
            && !inherits_roles
            && !replicates
            && !owns_database
            && !owns_public_table,
        "reference runtime role must be NOSUPERUSER, NOBYPASSRLS, NOCREATEDB, \
         NOCREATEROLE, NOINHERIT, NOREPLICATION and own no data objects"
    );

    let privileged_memberships = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM pg_roles AS privileged
         WHERE (privileged.rolsuper OR privileged.rolbypassrls)
           AND pg_has_role(current_user, privileged.oid, 'MEMBER')",
    )
    .fetch_one(runtime)
    .await?;
    ensure!(
        privileged_memberships == 0,
        "reference runtime role must not be able to assume a privileged role"
    );
    Ok(())
}
