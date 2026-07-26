ALTER TABLE users ADD COLUMN profile_avatar_key TEXT;

ALTER TABLE users DROP COLUMN IF EXISTS profile_avatar;
ALTER TABLE users DROP COLUMN IF EXISTS profile_avatar_content_type;
