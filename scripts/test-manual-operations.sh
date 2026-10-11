#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# No configured project database is accepted: migrations and fixtures only in a new container.
if command -v podman >/dev/null; then engine=podman; else engine=docker; fi
pg_id= redis_id=
cleanup() { local status=$?; trap - EXIT; if [ -n "$redis_id" ]; then "$engine" rm -f "$redis_id" >/dev/null || status=1; fi; if [ -n "$pg_id" ]; then "$engine" rm -f "$pg_id" >/dev/null || status=1; fi; exit "$status"; }
trap cleanup EXIT
pg_id=$("$engine" run --rm -d --pull=never --tmpfs /var/lib/postgresql/data -e POSTGRES_USER=astra_test -e POSTGRES_PASSWORD=operations-local-only -e POSTGRES_DB=astra_operations_test -p 127.0.0.1::5432 docker.io/library/postgres:16-alpine)
redis_id=$("$engine" run --rm -d --pull=never --tmpfs /data -p 127.0.0.1::6379 docker.io/library/redis:7.4-alpine redis-server --save '' --appendonly no)
redis_port=$("$engine" port "$redis_id" 6379/tcp | awk -F: '{print $NF}')
export TEST_REDIS_URL="redis://127.0.0.1:${redis_port}/0"
pg_port=$("$engine" port "$pg_id" 5432/tcp | awk -F: '{print $NF}')
for attempt in {1..30}; do
  if "$engine" exec "$pg_id" pg_isready -h 127.0.0.1 -U astra_test -d astra_operations_test >/dev/null; then break; fi
  sleep 1
done
"$engine" exec "$pg_id" pg_isready -h 127.0.0.1 -U astra_test -d astra_operations_test
export TEST_DATABASE_URL="postgresql://astra_test:operations-local-only@127.0.0.1:${pg_port}/astra_operations_test"
export DATABASE_URL="$TEST_DATABASE_URL" DIRECT_URL="$TEST_DATABASE_URL" SQLX_OFFLINE=true
node packages/database/node_modules/prisma/build/index.js migrate deploy --schema packages/database/prisma/schema.prisma
# Check the additive migration agrees with Prisma's source of truth.
node packages/database/node_modules/prisma/build/index.js migrate diff --from-url "$TEST_DATABASE_URL" --to-schema-datamodel packages/database/prisma/schema.prisma --exit-code
cd apps/api-rust
cargo test -p astra-infrastructure --locked --test operations_repository -- --ignored
cargo test -p astra-infrastructure --locked --test accounts_repository -- --ignored
cargo test -p astra-infrastructure --locked --test portfolio_repository -- --ignored
