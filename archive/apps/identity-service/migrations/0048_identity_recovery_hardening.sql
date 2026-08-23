ALTER TABLE tenants
    ADD COLUMN recovery_review_min_age_hours INTEGER NOT NULL DEFAULT 24,
    ADD CONSTRAINT tenants_recovery_review_min_age_hours_bounds
        CHECK (recovery_review_min_age_hours BETWEEN 1 AND 168);

UPDATE enterprise_password_recovery_requests
SET status = 'pending',
    approved_by_principal_id = NULL,
    approved_at = NULL,
    secondary_approved_by_principal_id = NULL,
    secondary_approved_at = NULL
WHERE status = 'approved';

UPDATE enterprise_password_recovery_requests
SET status = 'expired',
    updated_at = NOW()
WHERE status = 'token_issued';

ALTER TABLE enterprise_password_recovery_requests
    DROP CONSTRAINT IF EXISTS enterprise_password_recovery_requests_principal_id_key,
    ADD COLUMN risk_score DOUBLE PRECISION NOT NULL DEFAULT 0,
    ADD COLUMN risk_factors JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN review_reason TEXT,
    ADD COLUMN approval_expires_at TIMESTAMPTZ,
    ADD COLUMN rejected_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    ADD COLUMN rejected_at TIMESTAMPTZ,
    ADD CONSTRAINT enterprise_password_recovery_status
        CHECK (
            status IN (
                'pending',
                'approved',
                'issuing',
                'rejected',
                'cancelled',
                'token_issued',
                'consumed',
                'expired'
            )
        ),
    ADD CONSTRAINT enterprise_password_recovery_review_consistency
        CHECK (
            (status NOT IN ('approved', 'issuing', 'token_issued') OR (
                approved_by_principal_id IS NOT NULL
                AND approved_at IS NOT NULL
                AND secondary_approved_by_principal_id IS NOT NULL
                AND secondary_approved_at IS NOT NULL
                AND approval_expires_at IS NOT NULL
            ))
            AND
            (status <> 'rejected' OR (
                rejected_by_principal_id IS NOT NULL
                AND rejected_at IS NOT NULL
            ))
            AND
            (status <> 'token_issued' OR (
                reset_token_hash IS NOT NULL
                AND reset_token_expires_at IS NOT NULL
            ))
        );

CREATE UNIQUE INDEX identity_recovery_one_active_review
    ON enterprise_password_recovery_requests (principal_id)
    WHERE status IN ('pending', 'approved', 'issuing', 'token_issued');

CREATE INDEX identity_recovery_pending_review
    ON enterprise_password_recovery_requests (status, review_available_at, created_at)
    WHERE status = 'pending';
