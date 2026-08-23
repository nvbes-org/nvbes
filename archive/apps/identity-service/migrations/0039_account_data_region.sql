-- Account data residency belongs to the tenant, not to a workspace.
ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS data_region TEXT NOT NULL DEFAULT 'eu';

ALTER TABLE tenants
    ADD CONSTRAINT tenants_data_region_supported
    CHECK (data_region IN ('eu', 'us', 'ch', 'apac', 'uk', 'latam', 'me_africa'));
