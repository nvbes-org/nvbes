UPDATE tenants
SET mfa_policy = 'required_all'
WHERE mfa_policy = 'required_admins';

ALTER TABLE tenants
    DROP CONSTRAINT IF EXISTS tenants_mfa_policy_check;

ALTER TABLE tenants
    ADD CONSTRAINT tenants_mfa_policy_check
    CHECK (mfa_policy IN ('optional', 'required_all'));
