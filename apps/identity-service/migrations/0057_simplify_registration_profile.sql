ALTER TABLE users
    ALTER COLUMN firstname DROP NOT NULL,
    ALTER COLUMN lastname DROP NOT NULL;

DO $$
BEGIN
    IF EXISTS (
        SELECT lower(username)
        FROM users
        WHERE username IS NOT NULL
        GROUP BY lower(username)
        HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION
            'case-insensitive username duplicates must be resolved before migration 0057';
    END IF;
END
$$;

DROP INDEX idx_users_username;

CREATE UNIQUE INDEX idx_users_username
    ON users (lower(username))
    WHERE username IS NOT NULL;
