-- Versioned client verification keys for private_key_jwt and signed request objects.
CREATE TYPE oauth_client_key_purpose AS ENUM (
    'client_authentication',
    'request_object'
);

CREATE TYPE oauth_client_key_status AS ENUM (
    'active',
    'retiring',
    'revoked'
);

CREATE TABLE oauth_client_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    client_id VARCHAR(100) NOT NULL,
    purpose oauth_client_key_purpose NOT NULL,
    kid VARCHAR(255) NOT NULL,
    jwk JSONB NOT NULL,
    status oauth_client_key_status NOT NULL DEFAULT 'active',
    activated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retire_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT oauth_client_keys_jwk_is_object CHECK (jsonb_typeof(jwk) = 'object'),
    CONSTRAINT oauth_client_keys_kid_matches_jwk CHECK (jwk ->> 'kid' = kid),
    CONSTRAINT oauth_client_keys_client_tenant_fk
        FOREIGN KEY (tenant_id, client_id)
        REFERENCES oauth_clients (tenant_id, client_id)
        ON DELETE CASCADE,
    CONSTRAINT oauth_client_keys_lifecycle_consistent CHECK (
        (status = 'active' AND retire_at IS NULL AND revoked_at IS NULL)
        OR (status = 'retiring' AND retire_at IS NOT NULL AND revoked_at IS NULL)
        OR (status = 'revoked' AND revoked_at IS NOT NULL)
    ),
    UNIQUE (client_id, purpose, kid)
);

CREATE INDEX idx_oauth_client_keys_verification
    ON oauth_client_keys (client_id, purpose, status, retire_at);

-- Preserve existing private_key_jwt registrations while making kid mandatory.
INSERT INTO oauth_client_keys (tenant_id, client_id, purpose, kid, jwk)
SELECT
    tenant_id,
    client_id,
    'client_authentication',
    COALESCE(client_assertion_public_key_jwk ->> 'kid', 'legacy-' || id::text),
    CASE
        WHEN client_assertion_public_key_jwk ? 'kid' THEN client_assertion_public_key_jwk
        ELSE jsonb_set(
            client_assertion_public_key_jwk,
            '{kid}',
            to_jsonb('legacy-' || id::text)
        )
    END
FROM oauth_clients
WHERE client_assertion_public_key_jwk IS NOT NULL;

-- Expand the registered request-object JWKS into individually revocable keys.
INSERT INTO oauth_client_keys (tenant_id, client_id, purpose, kid, jwk)
SELECT
    client.tenant_id,
    client.client_id,
    'request_object',
    key ->> 'kid',
    key
FROM oauth_clients AS client
CROSS JOIN LATERAL jsonb_array_elements(client.request_object_signing_jwks -> 'keys') AS key
WHERE client.request_object_signing_jwks IS NOT NULL;

ALTER TABLE oauth_client_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE oauth_client_keys FORCE ROW LEVEL SECURITY;

CREATE POLICY oauth_client_keys_isolation ON oauth_client_keys
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
