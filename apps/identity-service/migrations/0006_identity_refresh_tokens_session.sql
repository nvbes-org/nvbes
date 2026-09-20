-- Add session_id to refresh tokens for proper token issuance
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'identity_refresh_tokens' AND column_name = 'session_id'
    ) THEN
        ALTER TABLE identity_refresh_tokens ADD COLUMN session_id UUID;
        
        -- Create index for session_id
        CREATE INDEX IF NOT EXISTS idx_refresh_tokens_session ON identity_refresh_tokens(session_id);
        
        -- Backfill session_id from sessions table where possible
        UPDATE identity_refresh_tokens irt
        SET session_id = s.id
        FROM identity_sessions s
        WHERE s.principal_id = irt.principal_id
        AND s.revoked_at IS NULL
        AND s.expires_at > clock_timestamp()
        AND irt.session_id IS NULL;
        
        -- Add foreign key constraint (validated after backfill)
        ALTER TABLE identity_refresh_tokens ADD CONSTRAINT fk_refresh_tokens_session FOREIGN KEY (session_id) REFERENCES identity_sessions(id) ON DELETE CASCADE;
    END IF;
END $$;