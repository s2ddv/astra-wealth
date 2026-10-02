# Astra Wealth

Plataforma para acompanhar patrimônio, carteiras, mercados e notícias em um só lugar.
O projeto está em desenvolvimento: o overview usa dados simulados; Markets usa
CoinGecko para cripto e mantém os demais mercados identificados como demonstração.
O feed de notícias usa RSS e, opcionalmente, NewsData.

## Estrutura real do monorepo

```text
apps/
├── web/          # Next.js 16, React 19, TypeScript e Tailwind CSS 4
├── api/          # Backend legado Fastify, porta 3333
└── api-rust/     # Backend Rust/Axum, porta 3334
packages/
├── database/     # Prisma e schema PostgreSQL
└── shared/       # Tipos TypeScript e DTOs gerados pelo Rust
```

Os pacotes TypeScript usam o escopo `@astra-wealth/*`. O backend Rust permanece
em `apps/api-rust`; não há pasta `apps/api-legacy-fastify`.

## Preparação

Requisitos: Node.js 24+, pnpm 10 (versão declarada em `packageManager`),
Rust 1.94+ e Docker Compose para serviços locais.

Na pasta em que o repositório foi clonado:

```sh
cp .env.example .env
pnpm install
pnpm db:generate
docker compose up -d
pnpm dev
```

O web fica em `http://localhost:3000`. `pnpm dev` inicia web e Fastify;
o Rust é iniciado separadamente conforme [seu README](apps/api-rust/README.md).
Preencha as variáveis locais de autenticação e banco antes de usar serviços que
precisam delas. Nunca versione credenciais.

## Comandos

```sh
pnpm dev:web
pnpm dev:api
pnpm turbo run lint typecheck build
pnpm --filter @astra-wealth/web start
pnpm db:generate
pnpm db:migrate
pnpm db:seed
```

Turbo gera o cliente Prisma antes das verificações dependentes. O build web
usa Webpack; lint/typecheck aguardam os tipos produzidos pelo Next. O seed é uma ação
explícita de desenvolvimento: não é executado durante instalação ou build.

## Funcionalidades e próximos passos

- Dashboard modular com overview e carteiras simuladas.
- Markets com cache de cripto, detalhes e watchlist local de até 100 ativos.
- Notícias autenticadas com filtros e orçamento de chamadas ao provedor.
- CRUD de carteiras no Rust, com validação por rede e isolamento por usuário.
- Integrações de corretoras/exchanges, sincronização on-chain, histórico,
  análise de desempenho, relatórios fiscais e app móvel continuam no roadmap.

Assinaturas de transações devem permanecer no cliente. Integrações precisam
preservar autenticação, isolamento por usuário e proteção de credenciais.

## Documentação

- [API Rust](apps/api-rust/README.md)
- [01 — Renomeação e configuração externa](docs/01-renomeacao-astra.md)
- [02 — Contas e aportes](docs/02-contas-e-aportes.md)
- [03 — Deploy gratuito](docs/03-deploy-gratuito.md)

## Contribuição e licença

Use branches de trabalho e commits convencionais em inglês. Valide os pacotes
afetados antes de abrir uma proposta de alteração. O projeto ainda não definiu
uma licença para distribuição pública.
