ALTER TYPE billing_provider ADD VALUE IF NOT EXISTS 'cb';

INSERT INTO billing_providers (provider)
VALUES ('cb')
ON CONFLICT (provider) DO NOTHING;
