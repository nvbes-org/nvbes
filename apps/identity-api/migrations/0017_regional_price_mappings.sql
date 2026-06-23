ALTER TABLE stripe_price_mappings
  ADD COLUMN IF NOT EXISTS country_code CHAR(2),
  ADD COLUMN IF NOT EXISTS pricing_region TEXT,
  ADD COLUMN IF NOT EXISTS currency CHAR(3) NOT NULL DEFAULT 'EUR',
  ADD COLUMN IF NOT EXISTS amount_minor BIGINT;

ALTER TABLE billing_provider_price_mappings
  ADD COLUMN IF NOT EXISTS country_code CHAR(2),
  ADD COLUMN IF NOT EXISTS pricing_region TEXT,
  ADD COLUMN IF NOT EXISTS currency CHAR(3) NOT NULL DEFAULT 'EUR',
  ADD COLUMN IF NOT EXISTS amount_minor BIGINT;

UPDATE billing_provider_price_mappings provider_mapping
SET country_code = stripe_mapping.country_code,
    pricing_region = stripe_mapping.pricing_region,
    currency = stripe_mapping.currency,
    amount_minor = stripe_mapping.amount_minor,
    updated_at = NOW()
FROM stripe_price_mappings stripe_mapping
WHERE provider_mapping.provider = 'stripe'
  AND provider_mapping.provider_price_id = stripe_mapping.stripe_price_id
  AND (
    provider_mapping.country_code IS NULL
    OR provider_mapping.pricing_region IS NULL
    OR provider_mapping.amount_minor IS NULL
  );

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'stripe_price_mappings_country_upper'
  ) THEN
    ALTER TABLE stripe_price_mappings
      ADD CONSTRAINT stripe_price_mappings_country_upper
      CHECK (country_code IS NULL OR country_code = upper(country_code));
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'stripe_price_mappings_single_market_scope'
  ) THEN
    ALTER TABLE stripe_price_mappings
      ADD CONSTRAINT stripe_price_mappings_single_market_scope
      CHECK (country_code IS NULL OR pricing_region IS NULL);
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'stripe_price_mappings_amount_non_negative'
  ) THEN
    ALTER TABLE stripe_price_mappings
      ADD CONSTRAINT stripe_price_mappings_amount_non_negative
      CHECK (amount_minor IS NULL OR amount_minor >= 0);
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'billing_provider_price_mappings_country_upper'
  ) THEN
    ALTER TABLE billing_provider_price_mappings
      ADD CONSTRAINT billing_provider_price_mappings_country_upper
      CHECK (country_code IS NULL OR country_code = upper(country_code));
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'billing_provider_price_mappings_single_market_scope'
  ) THEN
    ALTER TABLE billing_provider_price_mappings
      ADD CONSTRAINT billing_provider_price_mappings_single_market_scope
      CHECK (country_code IS NULL OR pricing_region IS NULL);
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'billing_provider_price_mappings_amount_non_negative'
  ) THEN
    ALTER TABLE billing_provider_price_mappings
      ADD CONSTRAINT billing_provider_price_mappings_amount_non_negative
      CHECK (amount_minor IS NULL OR amount_minor >= 0);
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_stripe_price_mappings_plan_market
  ON stripe_price_mappings (plan_id, meter, status, country_code, pricing_region, valid_from DESC);

CREATE INDEX IF NOT EXISTS idx_billing_provider_price_mappings_market
  ON billing_provider_price_mappings (
    provider,
    legacy_plan_id,
    status,
    country_code,
    pricing_region,
    created_at DESC
  );
