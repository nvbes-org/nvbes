ALTER TABLE identity_auth_factors
    ADD COLUMN enrollment_session_id UUID REFERENCES identity_sessions(id) ON DELETE SET NULL,
    ADD COLUMN enrollment_expires_at TIMESTAMPTZ;

-- Existing active factors remain usable. Pending internal diagnostic factors have
-- no browser binding and cannot be confirmed through the new HTTP ceremony.
