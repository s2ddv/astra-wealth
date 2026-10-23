#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$#" -ne 0 ]; then echo 'Usage: test-security-isolation.sh' >&2; exit 2; fi

# This gate never accepts a caller's database URL. Only newly created,
# localhost-only containers receive fixture data. db push initializes their
# empty schema; it must not be run under a no-schema-changes instruction.
if command -v podman >/dev/null; then engine=podman; else engine=docker; fi
command -v "$engine" >/dev/null || { echo 'Podman or Docker is required.' >&2; exit 1; }
pg_id= redis_id=
cleanup() {
  local status=$?
  trap - EXIT
  if [ -n "$redis_id" ]; then "$engine" rm -f "$redis_id" >/dev/null || status=1; fi
  if [ -n "$pg_id" ]; then "$engine" rm -f "$pg_id" >/dev/null || status=1; fi
  exit "$status"
}
trap cleanup EXIT
pg_id=$("$engine" run --rm -d --pull=never --tmpfs /var/lib/postgresql/data \
  -e POSTGRES_USER=astra_test -e POSTGRES_PASSWORD=isolation-local-only \
  -e POSTGRES_DB=astra_isolation_test -p 127.0.0.1::5432 docker.io/library/postgres:16-alpine)
redis_id=$("$engine" run --rm -d --pull=never --tmpfs /data \
  -p 127.0.0.1::6379 docker.io/library/redis:7.4-alpine redis-server --save '' --appendonly no)
pg_port=$("$engine" port "$pg_id" 5432/tcp | awk -F: '{print $NF}')
redis_port=$("$engine" port "$redis_id" 6379/tcp | awk -F: '{print $NF}')
for attempt in {1..30}; do
  # The image starts a temporary Unix-socket server during init. Wait for TCP.
  if "$engine" exec "$pg_id" pg_isready -h 127.0.0.1 -U astra_test -d astra_isolation_test >/dev/null; then break; fi
  sleep 1
done
"$engine" exec "$pg_id" pg_isready -h 127.0.0.1 -U astra_test -d astra_isolation_test
"$engine" exec "$redis_id" redis-cli ping | grep -qx PONG
export TEST_DATABASE_URL="postgresql://astra_test:isolation-local-only@127.0.0.1:${pg_port}/astra_isolation_test"
export TEST_REDIS_URL="redis://127.0.0.1:${redis_port}/0"
export DATABASE_URL="$TEST_DATABASE_URL" DIRECT_URL="$TEST_DATABASE_URL"
export SQLX_OFFLINE=true

# Materialize the current Prisma schema in the empty disposable database.
# Both database URLs above override any saved connection settings.
node packages/database/node_modules/prisma/build/index.js db push --skip-generate \
  --schema packages/database/prisma/schema.prisma
npm test
(
  cd apps/api-rust
  cargo test --workspace --locked
  for target in user_repository wallet_repository tracking_repository accounts_repository watchlist_repository portfolio_repository usd_snapshots_repository; do
    cargo test -p astra-infrastructure --locked --test "$target" -- --ignored
  done
  cargo test -p astra-api --locked --test multiuser_isolation -- --ignored
)
python3 scripts/test-rust-http.py
echo 'Security isolation gate passed (real disposable PostgreSQL/Redis; synthetic identities/providers).'
