ALTER TABLE user_sessions
    ADD COLUMN IF NOT EXISTS geo_country_code TEXT;
