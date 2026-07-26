ALTER TABLE users
    ADD COLUMN profile_avatar BYTEA,
    ADD COLUMN profile_avatar_content_type VARCHAR(64);
