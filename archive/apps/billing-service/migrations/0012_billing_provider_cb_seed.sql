INSERT INTO billing_providers (provider)
VALUES ('cb')
ON CONFLICT (provider) DO NOTHING;
