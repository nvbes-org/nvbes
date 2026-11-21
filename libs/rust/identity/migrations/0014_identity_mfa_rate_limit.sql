ALTER TABLE identity_rate_buckets
    DROP CONSTRAINT identity_rate_buckets_category_check,
    ADD CONSTRAINT identity_rate_buckets_category_check
        CHECK (category IN ('login_account', 'login_source', 'protocol_source', 'mfa_account'));
