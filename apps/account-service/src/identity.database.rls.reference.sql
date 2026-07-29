-- TEST-ONLY reference contract.
--
-- This file is deliberately outside migrations. Account handlers do not yet
-- set transaction-scoped RLS context consistently, so applying this contract
-- in production would break valid request and background-worker paths.

ALTER TABLE devices ENABLE ROW LEVEL SECURITY;
ALTER TABLE devices FORCE ROW LEVEL SECURITY;
CREATE POLICY device_tenant_isolation ON devices
    FOR ALL
    USING (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    )
    WITH CHECK (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    );

ALTER TABLE saml_assertion_ids ENABLE ROW LEVEL SECURITY;
ALTER TABLE saml_assertion_ids FORCE ROW LEVEL SECURITY;
CREATE POLICY saml_assertion_id_tenant_isolation ON saml_assertion_ids
    FOR ALL
    USING (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    )
    WITH CHECK (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    );

ALTER TABLE saml_pending_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE saml_pending_requests FORCE ROW LEVEL SECURITY;
CREATE POLICY saml_pending_request_tenant_isolation ON saml_pending_requests
    FOR ALL
    USING (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    )
    WITH CHECK (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    );

ALTER TABLE saml_sp_config ENABLE ROW LEVEL SECURITY;
ALTER TABLE saml_sp_config FORCE ROW LEVEL SECURITY;
CREATE POLICY saml_sp_config_tenant_isolation ON saml_sp_config
    FOR ALL
    USING (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    )
    WITH CHECK (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    );

ALTER TABLE security_event_deliveries ENABLE ROW LEVEL SECURITY;
ALTER TABLE security_event_deliveries FORCE ROW LEVEL SECURITY;
CREATE POLICY security_event_delivery_tenant_isolation ON security_event_deliveries
    FOR ALL
    USING (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    )
    WITH CHECK (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
    );

-- NULL-tenant sessions require a separate audited recovery capability.
ALTER TABLE user_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_sessions FORCE ROW LEVEL SECURITY;
CREATE POLICY user_session_tenant_principal_isolation ON user_sessions
    FOR ALL
    USING (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
        AND principal_id = NULLIF(
            current_setting('nvbes.principal_id', true),
            ''
        )::uuid
    )
    WITH CHECK (
        tenant_id = NULLIF(current_setting('nvbes.tenant_id', true), '')::uuid
        AND principal_id = NULLIF(
            current_setting('nvbes.principal_id', true),
            ''
        )::uuid
    );

DROP POLICY IF EXISTS system_policy_read ON system_policies;
CREATE POLICY system_policy_read ON system_policies
    FOR SELECT
    USING (TRUE);

DO $$
DECLARE
    isolated_table RECORD;
BEGIN
    FOR isolated_table IN
        SELECT namespace.nspname AS schema_name, class.relname AS table_name
        FROM pg_class AS class
        INNER JOIN pg_namespace AS namespace ON namespace.oid = class.relnamespace
        WHERE namespace.nspname = 'public'
          AND class.relkind IN ('r', 'p')
          AND class.relrowsecurity
    LOOP
        EXECUTE format(
            'ALTER TABLE %I.%I FORCE ROW LEVEL SECURITY',
            isolated_table.schema_name,
            isolated_table.table_name
        );
    END LOOP;
END
$$;
