ALTER TABLE tenant_policies
    ADD COLUMN IF NOT EXISTS admin_session_ttl_hours INTEGER;

ALTER TABLE tenant_policies
    DROP CONSTRAINT IF EXISTS tenant_policies_admin_session_ttl_hours_check;

ALTER TABLE tenant_policies
    ADD CONSTRAINT tenant_policies_admin_session_ttl_hours_check
    CHECK (
        admin_session_ttl_hours IS NULL
        OR admin_session_ttl_hours BETWEEN 1 AND 168
    );
