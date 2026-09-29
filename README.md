# Astra Wealth

> A unified platform for tracking your complete wealth, starting with crypto and expanding to traditional investments.

Astra Wealth is a modern dashboard that brings digital assets and, over time, traditional investments into a single, intuitive view. By tracking on-chain wallets today, and connecting exchanges and brokers next, users get a complete picture of their net worth without jumping between platforms.

## Overview

Managing assets across multiple wallets, exchanges, and brokers is fragmented and time-consuming.

Astra consolidates them into one platform, enabling investors to monitor, analyze, and understand their financial position from a single source of truth.

With Astra, users can:

* Track total net worth and portfolio evolution
* Monitor crypto holdings across multiple wallets and chains
* Visualize portfolio allocation
* Follow market prices, watchlists, and news
* Eliminate manual balance tracking

> **Status:** under active development. The MVP is crypto-only; stocks, ETFs, and broker integrations are post-MVP.

## Features

### Unified Portfolio Tracking

View all tracked crypto holdings, their USD value, and total portfolio value in one place.

### On-Chain Wallet Tracking

Add public wallet addresses and see native balances, tokens, and USD value. Tracking is strictly read-only: no wallet connection, transaction signing, or custody in the MVP.

### Market Data

Live prices, market tables, trending assets, and per-coin charts powered by CoinGecko, cached in Redis to avoid rate limits.

### Portfolio Analytics

Allocation, distribution, and historical net worth through portfolio snapshots.

### Watchlists & News

Save favorite assets and follow an aggregated crypto and economy news feed.

### Real-Time Updates

Price and portfolio updates (polling in the MVP, native WebSockets afterwards).

### Planned

* Exchange connections (read-only): Coinbase, Binance, Kraken, Bybit
* Stocks, ETFs, and fixed income via brokerage integrations
* Multi-chain support beyond EVM

## Tech Stack

### Frontend (`apps/web`)

* Next.js 16 (App Router)
* React 19
* TypeScript 5
* Tailwind CSS 4
* TanStack Query 5
* Lightweight Charts (TradingView)

### Backend (`apps/api`)

* Rust + Axum (Tokio)
* sqlx (compile-time checked PostgreSQL queries)
* deadpool-redis (caching)
* Supabase JWT validation
* ts-rs / specta (TypeScript types generated from Rust structs)
* Clean Architecture: Cargo workspace with `domain`, `application`, `infrastructure`, and `api` crates

> The backend is being migrated from Fastify/TypeScript to Rust/Axum. The legacy backend lives in `apps/api-legacy-fastify` and stays available until cutover. Route contracts are preserved, so the frontend needs no changes during the migration.

### Data & Infrastructure

* PostgreSQL hosted on Supabase
* Prisma schema as the single source of truth for the database (`packages/database`)
* Supabase Auth (email/password and Google OAuth)
* Redis for caching
* Docker and Docker Compose (local Postgres and Redis)
* Vercel (frontend), Railway or Fly.io (backend), GitHub Actions (CI/CD)

### External Providers

* CoinGecko: market data
* Alchemy: on-chain balances and tokens
* CryptoPanic / NewsAPI: news

### Monorepo Structure

```text
apps/
├── web/                  # Next.js frontend
├── api/                  # Rust/Axum backend (Cargo workspace)
└── api-legacy-fastify/   # Legacy Fastify backend (temporary)

packages/
├── database/             # Prisma schema, migrations, generated client
└── shared/               # Shared TypeScript types/DTOs
```

## Installation

### Prerequisites

* Node.js 22+
* pnpm
* Rust toolchain (stable) and Cargo
* Docker and Docker Compose
* A Supabase project (PostgreSQL and Auth)
* API keys for CoinGecko and Alchemy

### Clone the Repository

```bash
git clone https://github.com/your-username/astra-wealth.git

cd astra-wealth
```

### Install Dependencies

```bash
pnpm install
```

### Configure Environment

Copy the example environment files in each app and fill in your credentials (Supabase, database, Redis, CoinGecko, Alchemy).

> Use the **direct** Supabase connection (port `5432`) for `DATABASE_URL`. The pooled connection on port `6543` can hang silently on some networks.

### Start Local Services

```bash
docker compose up -d
```

### Run Development Server

```bash
pnpm dev
```

Web application available at:

```text
http://localhost:3000
```

### Run the Rust API

```bash
cd apps/api
cargo run
```

## Scripts

### Development

```bash
pnpm dev
```

### Build

```bash
pnpm build
```

### Production

```bash
pnpm start
```

### Type Checking

```bash
pnpm typecheck
```

### Backend (Rust)

```bash
cargo build
cargo test
cargo clippy
```

## Vision

Astra aims to become the central operating system for personal wealth management.

By bridging digital assets and traditional finance, the platform gives investors a complete and accurate picture of their financial life, with performance and real-time data quality as core differentiators.

## Roadmap

### Foundation (done)

* Turborepo + pnpm monorepo
* Database schema and migrations
* Authentication with Supabase Auth
* Dashboard UI (overview, markets, wallets, news, settings) on mock data

### MVP (in progress)

* Rust/Axum backend migration
* CoinGecko integration with Redis caching
* Wallet tracking via Alchemy
* Portfolio engine (total value, allocation, snapshots)
* Coin pages with charts
* News aggregation
* Watchlists

### Next

* Real-time updates via native WebSockets
* Exchange integrations (read-only)
* On-chain write layer using existing contracts (client-side signing only; backend remains read-only)

### Later

* Stocks, ETFs, and broker integrations
* Performance analytics and historical reporting
* Tax reporting
* AI-powered portfolio analysis
* Mobile applications

## Security

Security and privacy are fundamental principles of Astra.

* The backend is read-only with respect to funds: no custody, no private keys
* Exchange integrations will only request read permissions, never withdrawal or trading
* Exchange credentials are stored encrypted
* Authentication is delegated to Supabase Auth
* The frontend never accesses the database directly; all data flows through the API

## Contributing

Contributions are welcome.

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Open a Pull Request

## License

This project is currently under active development.

A license will be selected before public release.
