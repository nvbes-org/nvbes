UPDATE users
SET firstname = COALESCE(NULLIF(btrim(firstname), ''), COALESCE(NULLIF(btrim(username), ''), split_part(email, '@', 1), 'User')),
    lastname = COALESCE(NULLIF(btrim(lastname), ''), 'Account')
WHERE firstname IS NULL
   OR lastname IS NULL
   OR btrim(firstname) = ''
   OR btrim(lastname) = '';

ALTER TABLE users
    ALTER COLUMN firstname SET NOT NULL,
    ALTER COLUMN lastname SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_firstname_not_blank'
    ) THEN
        ALTER TABLE users
            ADD CONSTRAINT users_firstname_not_blank CHECK (btrim(firstname) <> '');
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_lastname_not_blank'
    ) THEN
        ALTER TABLE users
            ADD CONSTRAINT users_lastname_not_blank CHECK (btrim(lastname) <> '');
    END IF;
END $$;
