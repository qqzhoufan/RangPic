-- Random-pool sampling without ORDER BY RANDOM(), plus a short exclusion
-- window for entries that fail to load, and a live per-owner dedup constraint.

ALTER TABLE images
    ADD COLUMN IF NOT EXISTS rand_key double precision NOT NULL DEFAULT random();

ALTER TABLE images
    ADD COLUMN IF NOT EXISTS pool_excluded_until timestamptz;

CREATE INDEX IF NOT EXISTS idx_images_rand_pool
    ON images (rand_key)
    WHERE deleted_at IS NULL AND privacy = 'public';

-- Keep the oldest live copy when one owner already has the same bytes.
WITH ranked AS (
    SELECT id, owner_id, size_bytes,
           row_number() OVER (
               PARTITION BY owner_id, sha256
               ORDER BY created_at ASC, id ASC
           ) AS rn
    FROM images
    WHERE deleted_at IS NULL
      AND owner_id IS NOT NULL
      AND sha256 IS NOT NULL
),
doomed AS (
    UPDATE images
    SET deleted_at = now()
    WHERE id IN (SELECT id FROM ranked WHERE rn > 1)
    RETURNING owner_id, size_bytes
)
UPDATE users u
SET used_bytes = GREATEST(u.used_bytes - d.bytes, 0)
FROM (
    SELECT owner_id, SUM(size_bytes) AS bytes
    FROM doomed
    GROUP BY owner_id
) d
WHERE u.id = d.owner_id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_images_owner_sha_live
    ON images (owner_id, sha256)
    WHERE deleted_at IS NULL
      AND owner_id IS NOT NULL
      AND sha256 IS NOT NULL;
