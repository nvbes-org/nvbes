-- Add high-assurance OAuth security profiles and sender constraints.
CREATE TYPE oauth_security_profile AS ENUM ('standard', 'high_assurance');
CREATE TYPE oauth_sender_constraint AS ENUM ('dpop', 'mtls');

ALTER TABLE oauth_clients
    ADD COLUMN security_profile oauth_security_profile NOT NULL DEFAULT 'standard',
    ADD COLUMN request_object_signing_jwks JSONB,
    ADD COLUMN sender_constraint oauth_sender_constraint,
    ADD COLUMN tls_client_certificate_sha256 VARCHAR(43);

ALTER TABLE oauth_clients
    ADD CONSTRAINT oauth_clients_request_object_jwks_is_object
    CHECK (
        request_object_signing_jwks IS NULL
        OR jsonb_typeof(request_object_signing_jwks) = 'object'
    ),
    ADD CONSTRAINT oauth_clients_mtls_thumbprint_shape
    CHECK (
        tls_client_certificate_sha256 IS NULL
        OR tls_client_certificate_sha256 ~ '^[A-Za-z0-9_-]{43}$'
    ),
    ADD CONSTRAINT oauth_clients_high_assurance_configuration
    CHECK (
        security_profile <> 'high_assurance'
        OR (
            client_type <> 'public'
            AND client_assertion_required
            AND client_assertion_public_key_jwk IS NOT NULL
            AND request_object_signing_jwks IS NOT NULL
            AND sender_constraint IS NOT NULL
            AND (
                sender_constraint <> 'mtls'
                OR tls_client_certificate_sha256 IS NOT NULL
            )
        )
    );

CREATE TABLE oauth_request_object_jtis (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    client_id VARCHAR(100) NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    jti VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (client_id, jti)
);

CREATE INDEX idx_oauth_request_object_jtis_expires_at
    ON oauth_request_object_jtis (expires_at);

ALTER TABLE oauth_request_object_jtis ENABLE ROW LEVEL SECURITY;
CREATE POLICY oauth_request_object_jti_isolation ON oauth_request_object_jtis
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
