-- RangPic core schema

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TYPE user_role AS ENUM ('admin', 'user');
CREATE TYPE image_source AS ENUM ('upload', 'remote', 'local');
CREATE TYPE image_privacy AS ENUM ('public', 'unlisted', 'private');

CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role          user_role NOT NULL DEFAULT 'user',
    quota_bytes   BIGINT NOT NULL DEFAULT 5368709120,
    used_bytes    BIGINT NOT NULL DEFAULT 0,
    banned        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS sessions (
    token      TEXT PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    user_agent TEXT,
    ip         INET
);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);

CREATE TABLE IF NOT EXISTS api_tokens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL DEFAULT 'default',
    token_hash   TEXT NOT NULL UNIQUE,
    prefix       TEXT NOT NULL,
    last_used_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_api_tokens_user ON api_tokens(user_id);

CREATE TABLE IF NOT EXISTS albums (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    privacy     image_privacy NOT NULL DEFAULT 'unlisted',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(owner_id, slug)
);

CREATE TABLE IF NOT EXISTS images (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id      UUID REFERENCES users(id) ON DELETE SET NULL,
    album_id      UUID REFERENCES albums(id) ON DELETE SET NULL,
    source_kind   image_source NOT NULL DEFAULT 'upload',
    -- For remote/local entries: the URL to proxy/serve. For uploads: nullable.
    url           TEXT NOT NULL DEFAULT '',
    -- For uploads: storage keys for original + thumbnail.
    storage_key   TEXT,
    thumb_key     TEXT,
    filename      TEXT NOT NULL DEFAULT '',
    original_name TEXT NOT NULL DEFAULT '',
    content_type  TEXT NOT NULL DEFAULT 'application/octet-stream',
    ext           TEXT NOT NULL DEFAULT '',
    size_bytes    BIGINT NOT NULL DEFAULT 0,
    width         INTEGER,
    height        INTEGER,
    sha256        TEXT,
    privacy       image_privacy NOT NULL DEFAULT 'public',
    tags          TEXT[] NOT NULL DEFAULT '{}',
    views         BIGINT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at    TIMESTAMPTZ,
    deleted_at    TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_images_owner ON images(owner_id);
CREATE INDEX IF NOT EXISTS idx_images_album ON images(album_id);
CREATE INDEX IF NOT EXISTS idx_images_sha ON images(sha256);
CREATE INDEX IF NOT EXISTS idx_images_created ON images(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_tags ON images USING GIN (tags);
CREATE INDEX IF NOT EXISTS idx_images_privacy ON images(privacy);

-- Random-image pool index: images eligible for the flagship random API.
-- (Expiry is filtered at query time — `now()` is not IMMUTABLE so it can't
-- be part of an index predicate.)
CREATE INDEX IF NOT EXISTS idx_images_random_pool
    ON images(created_at)
    WHERE deleted_at IS NULL AND privacy = 'public';

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO settings(key, value) VALUES
    ('site_name', 'RangPic'),
    ('site_description', '一个自托管的随机图片与图床服务'),
    ('registration_open', 'true'),
    ('index_blurb', '')
ON CONFLICT (key) DO NOTHING;
