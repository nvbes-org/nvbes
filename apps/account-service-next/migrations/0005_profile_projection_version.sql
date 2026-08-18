ALTER TABLE account_profiles
    ADD COLUMN profile_version BIGINT NOT NULL DEFAULT 0,
    ADD CONSTRAINT account_profiles_profile_version_non_negative
        CHECK (profile_version >= 0);
