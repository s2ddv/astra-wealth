# Zora Wealth Rust API

Replacement backend, running beside Fastify until the final cutover. Fastify and
`apps/web` are unchanged. Default Rust port: **3334**, legacy: **3333**.

## Architecture

- `zora-domain`: entities and repository ports; no infrastructure dependencies.
- `zora-application`: user use cases; depends only on domain.
- `zora-infrastructure`: Postgres/SQLx, Redis pool and Supabase JWT adapters.
- `zora-api`: Axum HTTP boundary, dependency injection and composition.

Use Rust 1.94 or newer. Dependency versions were checked against the crates.io
registry. SQLx 0.9 removed `runtime-tokio-rustls`; the equivalent features are
`runtime-tokio` and `tls-rustls-ring`, alongside `postgres` and `macros`.

## Run

From this directory:

```sh
cp .env.example .env
cargo run -p zora-api --bin zora-api
```

Or, from the repository root, using the existing Postgres/Redis services:

```sh
docker compose -f docker-compose.yml -f apps/api-rust/compose.yml up --build api-rust
```

Use direct Postgres port **5432**. Supavisor port **6543** is rejected at startup.
The compose override does not change the legacy service configuration. Configure
authentication values in `apps/api-rust/.env`; never bake secrets into the image.

## Route contract audit

Sources: `apps/api/src/server.ts`, `plugins/auth.ts`, `lib/dev-auth.ts`,
`repositories/user.repository.ts`, `modules/me/me.routes.ts` and
`modules/health/health.routes.ts`.

Fastify has **no user/profile HTTP endpoint**. No `/v1/me` or `/users` route is
introduced. The user domain supports authentication internally, for the next
wallet stage. Existing wallet/watchlist/snapshot/market routes are not yet
implemented in Rust; do not cut over the frontend at this stage.

`GET /v1/health` and `/v1/health/` preserve the legacy JSON:
`status`, `service: "zora-wealth-api"`, millisecond UTC `timestamp`, and
`checks: { database, redis }`. Degraded health still returns HTTP 200, as in
Fastify. `GET /health` is the requested additional alias. Container readiness
checks the body's status, not just HTTP 200. Dependency probes time out after
five seconds.

## Next domains

Implement wallet → watchlist → wallet-asset → portfolio-snapshot →
exchange-connection. Compose their repositories/services in the API entrypoint
and consume the shared authenticated-user extractor in protected handlers.
Never add transaction signing to the backend; signing remains client-side.

## User and authentication

The entity mirrors `packages/database/prisma/schema.prisma`, including required
`authId`, nullable `name`, TEXT ids and millisecond timestamps. The repository
implements lookup, creation, partial update (including explicit name clearing)
and an atomic authentication upsert. Authentication updates email only, keeping
the existing local id and profile name. An absent email becomes
`user-{authId}@supabase.local`, matching Fastify. Relation queries will be added
with their respective domains; there is no profile route requiring them now.

Set the same `SUPABASE_URL` and `SUPABASE_SERVICE_ROLE_KEY` as Fastify. For legacy
HS256 projects, also set `SUPABASE_JWT_SECRET` from that Supabase project. For
asymmetric signing, JWKS defaults to `/auth/v1/.well-known/jwks.json`; an explicit
`SUPABASE_JWKS_URL` overrides it. Supported algorithms: HS256, RS256 and ES256.
Signature, expiry, issuer, audience (`authenticated`) and subject are checked.
JWKS is cached for five minutes; unknown key ids can trigger a refresh at most
once every 30 seconds. Upstream requests have a five-second timeout.

After local verification, `/auth/v1/user` is still consulted, as in Fastify's
`auth.getUser(token)`, before synchronizing the current email. The service role
key is only an upstream credential, never treated as a user JWT signing secret.
The extractor inserts `AuthenticatedUser` into request extensions and returns
the legacy 401 JSON errors. With Supabase configured, invalid tokens never fall
back to a development user. Missing tokens can use `DEV_USER_ID` outside production.
When Supabase is absent in development, `X-User-Id` and `DEV_USER_ID` preserve the
legacy fallback. **Production fails startup without Supabase configuration**;
this deliberately prevents Fastify's missing-configuration auth bypass.

`garde` is available for the next domain's request validation. No invented user
payload or HTTP validation contract is added in this stage.

## Build and verify

Run inside `apps/api-rust` so Cargo reads `.cargo/config.toml`:

```sh
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo fmt --all -- --check
cargo run -p zora-api --bin export-types
```

Type generation writes `packages/shared/src/generated/rust/UserDto.ts`. It does
not replace existing shared exports or require frontend changes. The DTO uses
camelCase keys and UTC timestamps matching Prisma serialization. Test fixture
private keys are intentionally public, test-only credentials.

SQLx queries use `query_as!` and committed `.sqlx` metadata, generated against an
isolated Postgres database initialized from the **current Prisma schema**. No
live database or credentials are required for normal builds or Docker builds.
Do not use the historical Prisma migrations to regenerate this cache: they do
not represent the current schema completely. To refresh, initialize a disposable
Postgres with the existing Prisma schema (`prisma db push --skip-generate`), then:

```sh
# DATABASE_URL must point to that disposable database, never production.
cargo clean -p zora-infrastructure
SQLX_OFFLINE=false SQLX_OFFLINE_DIR="$PWD/.sqlx" cargo build --workspace
TEST_DATABASE_URL="$DATABASE_URL" cargo test -p zora-infrastructure --test user_repository -- --ignored
```

The integration test exercises creation, TEXT ids, nullable updates, uniqueness,
email fallback and concurrent first-login upserts. It only deletes its own test
users. Normal tests exercise HS256/ES256, JWKS, invalid claims and legacy auth
errors against a local mock Supabase server.

## Asset icons and CoinGecko ingestion

`infrastructure::market::crypto_market_services(redis_pool, api_key)` composes a
`CryptoMarketService` and `AssetIconService` with one bounded HTTP client and one
Redis cache. Phase 5 exposes crypto market routes and connects the Markets page through the
Next.js same-origin bridge. Ingestion supports paginated `/coins/markets` in USD,
`/coins/{id}` details and `/search/trending`. Stock tracking and server-side
watchlist persistence remain outside this phase.

Market payloads use a 60-second TTL; trending uses 300 seconds. The same decoded payload supplies icon metadata:
markets returns an image URL; details prefer `large`, then `small`, then `thumb`.
The icon provider performs **zero HTTP requests**. API keys, when configured, are
sent using the CoinGecko Demo API header, never in URLs. Paid/Pro API support is not
configured by this adapter.

Resolved icons use Redis `SETEX` with **86400 seconds**. Keys are
`icon:crypto:id:{coingecko_id}`, `icon:crypto:token:{chain}:{address}`,
`icon:crypto:symbol:{SYMBOL}` or `icon:stock:{TICKER}`. Chain and contract identity
prevent ticker collisions; Solana address case is preserved and EVM keys normalize
case. Symbol lookup only uses the currently ingested metadata, refuses ambiguous
matches, and never reads old symbol cache entries. It is not a global symbol
catalog lookup. Supply a CoinGecko id or chain/contract for reliable identity.

The Trust Wallet fallback downloads a PNG (HTTP 200, image/png, PNG signature,
256 KiB maximum). EVM paths use EIP-55 checksums. Fixed upstream hosts, disabled
redirects and validated path components prevent arbitrary URL fetching. HTTP calls
have a 5-second timeout; Redis operations have a 1-second timeout. Missing images,
rate limits and unavailable providers fall back to `{ url: null, symbol }`;
cache failures do not fail a successful icon resolution. Failed lookups aren't
cached for 24h so transient outages can recover.

Downloaded Trust Wallet images are returned and cached as PNG **data URLs**, never
as GitHub URLs. This avoids hotlinking and doesn't need a new public proxy route or
filesystem storage. The future frontend should accept `data:` for image sources,
render initials when `url` is null or image loading fails, and derive a stable color
from the returned normalized symbol. Markets implements this fallback and image-error handling.

`FmpStockIconProvider` implements the separate `StockIconProvider` opt-in port. It
validates/downloads the public PNG before resolving its HTTPS URL. It is not wired
into `crypto_market_services`, API routers or the dashboard. `with_stocks` is only
for future consumers and isolated tests, not stock tracking or market integration.

`cargo run -p zora-api --bin export-types` also generates
`packages/shared/src/generated/rust/IconResponse.ts` using ts-rs. The DTO does not
change existing shared exports. `MarketDto.ts` now supplies the web market hooks. Provider tests use local HTTP fixtures; service
tests verify cache TTLs, fallback ordering, validation and stock isolation.

To verify Redis roundtrip and TTL against a disposable instance:

```sh
TEST_REDIS_URL=redis://127.0.0.1:56391 cargo test -p zora-infrastructure --test asset_icon_cache -- --ignored
```

Provider references: [CoinGecko markets](https://docs.coingecko.com/reference/coins-markets),
[CoinGecko details](https://docs.coingecko.com/reference/coins-id),
[Trust Wallet asset layout](https://github.com/trustwallet/assets),
[FMP company logos](https://site.financialmodelingprep.com/developer/docs/company-image-api).


## Phase 5: live crypto markets

Public read-only routes, all backed by `CryptoMarketService`:

- `GET /v1/market/coins?page=1&perPage=50`: paginated `{ data, page, perPage, hasMore }`.
  `perPage` accepts 1–100 and `page` 1–10000. Optional `ids=bitcoin,ethereum`
  filters by at most 100 validated CoinGecko IDs. IDs are sorted/deduplicated for cache reuse.
- `GET /v1/market/coins/{id}`: details with price, 24h change, market cap, volume,
  icon, rank and upstream update timestamp. Missing metrics stay null, never zero.
- `GET /v1/market/trending`: up to 15 trending searches. Search popularity is not
  price performance, and no fake USD prices are inferred from BTC-denominated values.
- `GET /v1/market/spot?limit=10`: legacy array response shape with nullable missing metrics.

Invalid queries return 400, missing coins 404, provider/network errors 502,
and upstream rate limits 503 with `Retry-After: 60`. A process-wide cache-fill
lock coalesces concurrent requests; a 60-second cooldown follows an upstream 429.
Expired market data is not silently served by the API. Malformed cache entries
are refetched. Valid market cache hits do not extend their TTL. Icons resolve in
bounded batches so a cache outage cannot serially delay every asset on a page.

Run the Rust API as documented above with Redis reachable. Set `COINGECKO_API_KEY`
in `apps/api-rust/.env` for a Demo key where required by the provider. Without it,
the adapter attempts the public endpoint. Keys are never exposed to the browser.
For deployment, set **server-side** `MARKET_API_URL` in the Next.js environment to
this Rust service's origin. Local default: `http://127.0.0.1:3334`. Existing
`NEXT_PUBLIC_API_URL` and legacy wallet/news flows are not redirected.

Markets polls crypto every 60 seconds and trending every five minutes, presents
loading/error/empty states and an explicit stale-response warning after refresh
failure, and opens an accessible native dialog for details. Filters and sorting
apply to the current page; navigation reaches additional CoinGecko results.
The browser watchlist migrates the old BTC/ETH/SOL/LINK/USDT demo keys to canonical
CoinGecko IDs, retains assets outside the current page and fetches their quotes
in a batch. It remains local (100-asset cap), not account-synchronized. Traditional
assets retain their existing demo values and explicit Demo labels; FMP is still
unwired and no stock route exists.

Route tests cover validation before provider calls, DTOs, error statuses and
stock isolation. Provider tests cover pagination/IDs, trending payloads, shared
cache fills, corrupted cache recovery and rate-limit cooldowns.

## News feed

`GET /v1/news?category=all&lang=all&limit=20` uses the existing Supabase JWT
extractor (`Authorization: Bearer <access_token>`). Parameters: `category` is
`all|crypto|macro`, `lang` is `all|pt|en`, `limit` is 1–50, and `cursor` is the
opaque `nextCursor` from the preceding response with the same filters.
Returns `{ data: NewsArticleDto[], nextCursor: string | null, stale: boolean }`.
Invalid queries return 400; unavailable providers with no stale cache return 503.
The feature stores no news in Postgres; existing authentication still uses the
existing user service.

Put optional `NEWSDATA_API_KEY` in **apps/api-rust/.env**, alongside `REDIS_URL`
and the existing auth/database settings. An empty/missing key logs a warning and
uses RSS only. Run from this directory with `cargo run -p zora-api --bin zora-api`
(port 3334). Never put the NewsData key in a `NEXT_PUBLIC_*` variable.
The web app uses its existing Supabase cookie session via `/api/news` and
`NEWS_API_URL=http://127.0.0.1:3334` in `apps/web/.env.local`.

Merged lists (up to 200 articles) live at `news:v1:{category}:{lang}` for 300s;
`news:v1:stale:{category}:{lang}` lasts 3600s and is returned if all providers fail.
A shared in-process single-flight gate coalesces cache misses, with a second
cache lookup inside the gate. RSS failures are isolated per source. Pagination
uses offsets over the current cached snapshot; snapshots can change after TTL,
so clients should deduplicate IDs when appending pages.

NewsData `/crypto` and `/market` each fetch one page of 10 articles, both languages,
with a 1200s cache independent of user filters. Atomic Redis Lua reserves a
credit **before** the request: maximum 180 per UTC day and 30 in a rolling 900s
window. A per-endpoint 1200s cooldown also includes failed requests and coordinates
multiple API instances. Two successful endpoint requests every 20 minutes cost
at most 144 credits/day. Redis failure disables paid calls (fail closed).
Use a persistent, dedicated Redis with `maxmemory-policy noeviction` for the
budget keys: manually deleting them, losing persistence or sharing the same API
key with other clients invalidates the global credit accounting. The API cannot
account for credits consumed outside this service.

RSS URLs, live checks and exclusion reasons are in [news-sources.md](docs/news-sources.md).
Only feed summaries are sanitized/truncated; full article pages are never scraped.
NewsData free-plan articles may arrive about 12 hours late.

Checks (unit fixtures need no upstream services):

```sh
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cargo run -p zora-api --bin export-types
# Optional integration test, isolated local Redis/Valkey only:
TEST_REDIS_URL=redis://127.0.0.1:56401 cargo test -p zora-infrastructure --test news_cache -- --ignored
```
