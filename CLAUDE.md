# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

RangPic is a self-hosted **random-image service + image host** written in Rust. The flagship feature is the random-image API (`/random-image`, `/api/random-image`) with tag filtering; around it is a full image-hosting platform: user accounts, upload with thumbnails/dedup/quota, albums, share links, API tokens, and an admin panel. The UI is hand-written (Chinese-facing) — warm-stone neutrals + emerald accent, dark-mode-first galleries.

The Go version that originally lived in `cmd/rangpic/` has been superseded; the active project is the Rust crate at the repo root. `data/image_urls.txt` (the legacy seed format) is still honored.

## Commands

```bash
cargo run                       # dev run (needs DATABASE_URL + ADMIN_USERNAME/PASSWORD; .env supported)
cargo run --release             # optimized build
cargo build --release           # build only
cargo check                     # fast type-check while iterating
cargo build                     # produce target/debug/rangpic
docker compose up --build -d    # full stack (app + postgres), the recommended way to run

# validate frontend (parse-check all page scripts):
node --check static/js/*.js
```

There are no Rust tests in-tree yet. SQL queries use runtime `sqlx::query*` (not the compile-time `query!` macros), so the project compiles **without** a live database; `sqlx::migrate!` reads `migrations/` at compile time and the `rust-embed` `#[folder = "static/"]` reads `static/` at compile time — both directories must exist when you build.

**Required env vars** (all `log::panic` if missing at startup): `DATABASE_URL`, `ADMIN_USERNAME`, `ADMIN_PASSWORD`. `ADMIN_*` are only used to bootstrap an admin on first run. See `.env.example` for the rest.

## Architecture

### One crate, module per concern
Single binary crate `rangpic` (`src/main.rs` + modules). There is no workspace. `main.rs` is thin: load config → connect pool → run migrations → bootstrap admin → seed random pool → build `AppState` → serve.

- `config.rs` — all config from env (`.env` optional via `dotenvy`).
- `state.rs` — `AppState` (Clone, shared via `Arc<Inner>`): holds `PgPool`, boxed `dyn Storage`, and a `reqwest::Client`.
- `error.rs` — `AppError` (thiserror) with `IntoResponse` that emits `{error, code}` JSON; expected errors (404/400/401/403/409/413) are not logged at error level.
- `db.rs` — `FromRow` models (`Image`, `User`, `Album`, `ApiToken`) and the `*_COLUMNS` string constants.
- `auth.rs` — argon2id hashing, opaque session tokens (DB `sessions` table), API tokens (`rpic_…`, stored as SHA-256), and the `AuthUser` / `MaybeAuthUser` / `AdminUser` request extractors.
- `storage.rs` — `Storage` trait (`put`/`get`/`open`/`delete`/`size`) with a `LocalStorage` filesystem impl; keys are sanitized to prevent path traversal. Designed for adding S3/etc.
- `imaging.rs` — magic-byte format detection, `sha256_hex`, dimensions, and WebP thumbnail generation via the `image` crate.
- `routes/` — `mod.rs` assembles the router and holds shared helpers (`render_image`, `serve_thumb`, `image_json`, `load_image`, `can_view`, `ensure_viewable`, `bump_views`); submodules `public`, `auth_routes`, `upload`, `images`, `albums`, `admin`, `pages`.
- `assets.rs` — `rust-embed` over `static/`; serves embedded files and named HTML pages.

### Database
PostgreSQL. One migration file (`migrations/0001_init.sql`) creates `users`, `sessions`, `api_tokens`, `albums`, `images`, `settings`. `role`/`source_kind`/`privacy` are **Postgres enums** — read as `String` via `::text` casts in the `*_COLUMNS` constants, and written with explicit `$n::image_privacy` / `$n::user_role` casts on bound parameters (string *literals* cast implicitly; bound `String` does **not** — easy footgun, see `upload.rs` ingest insert).

Gotcha: `SUM(bigint)` returns `numeric` in Postgres, so aggregate sums are cast `::bigint` before decoding into `i64` (see stats queries). And `now()` is not `IMMUTABLE`, so it cannot appear in a partial-index predicate (the random-pool index only filters `deleted_at IS NULL AND privacy='public'`; expiry is filtered at query time).

### Request flow / auth
Extractors in `auth.rs` resolve identity in order: `Authorization: Bearer <token>` (API token) → `session` cookie. Sessions are stored server-side (revocable, survive restart) — not JWTs. `AuthUser` rejects 401, `AdminUser` escalates 401→403, `MaybeAuthUser` is anonymous-safe. The same extractors work for both browser (cookie) and API (token) callers on every endpoint.

### The flagship random-image feature
`choose_random` (in `routes/public.rs`) selects from `images WHERE deleted_at IS NULL AND privacy='public' AND (expires_at IS NULL OR expires_at>now())`, optional case-insensitive substring tag match over `unnest(tags)`, `ORDER BY RANDOM() LIMIT 1`. `/random-image` streams bytes (local storage is streamed via `tokio_util::io::ReaderStream`; remote `url` entries are proxied through `reqwest`), with `Cache-Control: no-store` so each request re-randomizes. `/api/random-image` returns JSON built by `image_json` (direct/thumbnail/share/markdown/html/bbcode links).

### Image storage model
Each `images` row is one of `source_kind`: `upload` (bytes in `Storage` under `orig/<sh>/<idhex>.<ext>`, thumbnail under `thumb/<sh>/<idhex>.webp`), `remote` (legacy seed entries — proxied from `url`), or `local`. `render_image` branches on `is_local()` (upload/local) vs proxy. Upload pipeline (`routes/upload.rs::ingest`) validates magic bytes + size, hashes for per-owner dedup, checks quota, writes original + WebP thumb, inserts the row, and increments `users.used_bytes`.

Privacy: `public` (in random pool + public gallery), `unlisted` (anyone with `/i/{id}` or `/s/{id}`), `private` (owner/admin only). `ensure_viewable` also treats expired/deleted as 404 for everyone.

### Frontend (no build step)
`static/` is hand-written and embedded into the binary. `app.js` exposes `window.R` (API helper, toasts, clipboard, formatting, custom SVG `icon()` set, theme, nav/footer rendering, `modal`/`confirmDialog`). Each page is a thin HTML shell (`pages/*.html`) + a `page-*.js` that calls `R.onReady(cb)` after `/api/site` loads. Nav and footer are injected at runtime from `#nav-mount`/`#footer-mount`, so don't hard-code them in page HTML. Design tokens live in `static/css/app.css` (`:root` for light, `[data-theme="dark"]` for dark) — keep new components on these CSS variables.

## Gotchas

- **Dockerfile fixes a latent bug from the Go version.** The Go code read `data/image_urls.txt` relative to CWD but the old Dockerfile flattened it to `/app/image_urls.txt`. The Rust Dockerfile sets `DATA_DIR=/app` and copies `data/` so the seed path resolves to `/app/data/image_urls.txt`.
- **Frontend is embedded.** Edits under `static/` require a recompile (`cargo run`) to take effect — there is no live reload.
- **MSRV is 1.88.** Pinned deps in `Cargo.lock` need rustc ≥ 1.88 (`image`/`time`) and ≥ 1.86 (`icu_*`). The Dockerfile uses `rust:1-bookworm` (a current stable, e.g. 1.97.x) so the `--locked` build resolves; pin to e.g. `rust:1.97-bookworm` if you want reproducibility. Building with an older Rust (e.g. 1.85) fails at dependency compile time, not on our code.
- **`POST /api/my/images/{id}/delete`** is a POST (not DELETE), because the frontend uses plain form-style fetches; the DELETE method isn't wired for it.
- **Random pool can include remote seed URLs.** `/random-image` will try to proxy them; if the seed URLs are stale/dead it returns the upstream error status (not a RangPic bug). Replace `data/image_urls.txt` with live URLs.
- Old Go sources under `cmd/rangpic/`, plus `go.mod`/`go.sum`/`web/`, are retained for history but are **not** part of the build.
