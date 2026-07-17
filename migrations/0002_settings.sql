-- Runtime-configurable site domain + default new-user quota (MB stored as bytes).
-- site_url: '' means relative links (default). Set to your public origin, e.g.
--           'https://pic.example.com', to get correct absolute share/embed links.
-- default_quota_bytes: applied to newly registered users. 0 = unlimited.
INSERT INTO settings (key, value) VALUES
    ('site_url', ''),
    ('default_quota_bytes', '104857600')
ON CONFLICT (key) DO NOTHING;
