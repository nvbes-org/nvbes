-- Billing V1 — Schema additions for gRPC operations snapshot
-- Adds monthly_price_cents to billing_subscriptions (denormalized for fast MRR aggregation)
-- and idempotency_key to billing_outbox for external event deduplication.

ALTER TABLE billing_subscriptions
  ADD COLUMN IF NOT EXISTS monthly_price_cents BIGINT NOT NULL DEFAULT 0;

COMMENT ON COLUMN billing_subscriptions.monthly_price_cents
  IS 'Denormalized monthly price for fast MRR aggregation without joining billing_plans. Updated on every subscription event.';
