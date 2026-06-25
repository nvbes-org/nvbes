CREATE TABLE IF NOT EXISTS internal_admin_operator_grants (
  principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  granted_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  revoked_at TIMESTAMPTZ,
  reason TEXT,
  PRIMARY KEY (principal_id, role),
  CONSTRAINT internal_admin_operator_grants_role_check CHECK (
    role IN (
      'compliance_admin',
      'developer_admin',
      'finance_admin',
      'operations_admin',
      'platform_admin',
      'product_admin',
      'security_admin',
      'support_agent',
      'viewer'
    )
  ),
  CONSTRAINT internal_admin_operator_grants_status_check CHECK (
    status IN ('active', 'revoked')
  ),
  CONSTRAINT internal_admin_operator_grants_revoked_at_check CHECK (
    (status = 'revoked' AND revoked_at IS NOT NULL)
    OR (status = 'active' AND revoked_at IS NULL)
  )
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_operator_grants_active_role
  ON internal_admin_operator_grants (role, principal_id)
  WHERE status = 'active';
