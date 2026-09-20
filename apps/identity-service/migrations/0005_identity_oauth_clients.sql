-- OAuth clients table for OAuth 2.1 authorization server
CREATE TABLE IF NOT EXISTS identity_oauth_clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id VARCHAR(128) UNIQUE NOT NULL,
    client_secret_hash BYTEA NOT NULL,
    name VARCHAR(255) NOT NULL,
    redirect_uris TEXT[] NOT NULL,
    scopes TEXT[] NOT NULL,
    is_confidential BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX IF NOT EXISTS idx_oauth_clients_client_id ON identity_oauth_clients(client_id);

-- Authorization codes table for OAuth 2.1 authorization code flow
CREATE TABLE IF NOT EXISTS identity_authorization_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code_hash BYTEA NOT NULL,
    client_id VARCHAR(128) NOT NULL,
    principal_id UUID NOT NULL,
    session_id UUID NOT NULL,
    redirect_uri TEXT NOT NULL,
    scope TEXT NOT NULL,
    code_challenge VARCHAR(128),
    code_challenge_method VARCHAR(10),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    consumed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT fk_auth_codes_principal FOREIGN KEY (principal_id) REFERENCES identity_principals(id) ON DELETE CASCADE,
    CONSTRAINT fk_auth_codes_session FOREIGN KEY (session_id) REFERENCES identity_sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_auth_codes_hash ON identity_authorization_codes(code_hash);
CREATE INDEX IF NOT EXISTS idx_auth_codes_client ON identity_authorization_codes(client_id);
CREATE INDEX IF NOT EXISTS idx_auth_codes_expires_at ON identity_authorization_codes(expires_at);