#!/bin/bash
# Postgres listens on 127.0.0.1 only. Uploads and the database both live under /data.
set -euo pipefail

PG_BIN=/usr/lib/postgresql/15/bin
PGDATA=/data/postgres
STORAGE=/data/storage
PG_USER=rangpic
PG_DB=rangpic

mkdir -p "$PGDATA" "$STORAGE" /var/run/postgresql
chown postgres:postgres /var/run/postgresql
if [ "$(stat -c %u "$PGDATA")" != "$(id -u postgres)" ]; then
  chown -R postgres:postgres "$PGDATA"
fi
chmod 700 "$PGDATA"

if [ ! -s "$PGDATA/PG_VERSION" ]; then
  runuser -u postgres -- "$PG_BIN/initdb" -D "$PGDATA" \
    --username="$PG_USER" --auth-local=trust --auth-host=trust --locale=C.UTF-8
fi

cat > "$PGDATA/pg_hba.conf" <<'EOF'
local all all trust
host  all all 127.0.0.1/32 trust
host  all all ::1/128 trust
EOF
chown postgres:postgres "$PGDATA/pg_hba.conf"
chmod 600 "$PGDATA/pg_hba.conf"

runuser -u postgres -- "$PG_BIN/postgres" -D "$PGDATA" \
  -c listen_addresses=127.0.0.1 \
  -c unix_socket_directories=/var/run/postgresql \
  -c logging_collector=off \
  &
pgpid=$!

ready=0
for _ in $(seq 1 60); do
  if runuser -u postgres -- "$PG_BIN/pg_isready" -h 127.0.0.1 -p 5432 -U "$PG_USER" >/dev/null 2>&1; then
    ready=1
    break
  fi
  if ! kill -0 "$pgpid" 2>/dev/null; then
    break
  fi
  sleep 1
done
if [ "$ready" -ne 1 ]; then
  echo "PostgreSQL 未能在容器内启动" >&2
  kill "$pgpid" 2>/dev/null || true
  wait "$pgpid" 2>/dev/null || true
  exit 1
fi

exists="$(runuser -u postgres -- "$PG_BIN/psql" -h 127.0.0.1 -U "$PG_USER" -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname='${PG_DB}'")"
if [ "$exists" != "1" ]; then
  runuser -u postgres -- "$PG_BIN/createdb" -h 127.0.0.1 -U "$PG_USER" "$PG_DB"
fi

export DATABASE_URL="postgres://${PG_USER}@127.0.0.1:5432/${PG_DB}?sslmode=disable"
export STORAGE_DIR="$STORAGE"
export DATA_DIR=/app
export BIND_ADDR="${BIND_ADDR:-0.0.0.0:17777}"

cleaned=0
cleanup() {
  if [ "$cleaned" = 1 ]; then
    return
  fi
  cleaned=1
  if [ -n "${appid:-}" ] && kill -0 "$appid" 2>/dev/null; then
    kill -TERM "$appid" 2>/dev/null || true
    wait "$appid" 2>/dev/null || true
  fi
  if [ -n "${pgpid:-}" ] && kill -0 "$pgpid" 2>/dev/null; then
    runuser -u postgres -- "$PG_BIN/pg_ctl" -D "$PGDATA" -m fast -w stop >/dev/null 2>&1 || true
    wait "$pgpid" 2>/dev/null || true
  fi
}
trap 'cleanup; exit 0' TERM INT

/app/rangpic &
appid=$!

set +e
wait -n "$appid" "$pgpid"
status=$?
set -e
cleanup
exit "$status"
