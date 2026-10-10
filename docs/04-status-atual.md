# Astra Wealth — Status Atual e Referência Operacional

> Documento de referência para o trabalho no repositório. Atualizado em 10 de outubro de 2026 (PTAX e consolidação; demais domínios mantêm limites de validação indicados).
>
> Este arquivo complementa o roadmap histórico e deve ser consultado antes de iniciar novas tarefas. Ele descreve o estado verificado do checkout atual, não apenas a intenção futura do produto.

## 1. Checkout e estado de referência

- Repositório local: `/home/samuel-barbosa/Documentos/astra-wealth`.
- Branch verificada: `main`.
- Base inicial desta tarefa: `865f544`, `main` limpa. Durante as interrupções, entraram os commits de autenticação/isolamento até `b51e7b8`; foram preservados. O status de publicação desta entrega deve ser consultado no Git, não inferido deste checkpoint.
- O caminho antigo `zora-wealth` é um link simbólico para este checkout Astra.
- O produto e os pacotes usam a marca Astra Wealth; referências históricas a Zora podem permanecer em commits e migrações antigas.

Qualquer tarefa deve preservar alterações locais não relacionadas e confirmar o status do Git antes de editar arquivos.

## 2. Arquitetura que existe hoje

```text
apps/
  web/          Next.js, React, Tailwind v4 e TanStack Query
  api/          Fastify/TypeScript legado, ainda ativo
  api-rust/     Rust/Axum em migração incremental
packages/
  database/     Prisma e schema oficial do banco
  shared/       DTOs, validações e tipos compartilhados
docs/           Documentação operacional em português
```

O frontend não acessa o banco diretamente. A comunicação deve passar por uma API.

O backend Rust roda por padrão na porta `3334`; o Fastify legado roda na porta `3333`. Os dois devem continuar lado a lado até que cada domínio tenha sido migrado e validado.

## 3. Estado por domínio

### Fundação, autenticação e saúde — concluído

- Monorepo pnpm/Turborepo configurado.
- Frontend Next.js com App Router e Tailwind CSS v4.
- Autenticação Supabase no frontend.
- Validação de JWT Supabase no Rust, incluindo HS256, RS256 e ES256.
- Health checks no Rust em `/health`, `/v1/health` e `/v1/health/`.
- TypeScript compartilhado entre `web`, `api`, `database` e `shared`.

### Banco de dados — implementado e em evolução

- Fonte oficial: `packages/database/prisma/schema.prisma`.
- Conexão direta do PostgreSQL deve usar a porta `5432`.
- A porta `6543` do pooler transacional não deve ser usada.
- O schema atual contém, além dos modelos cripto originais:
  - `FinancialAccount`;
  - `Holding`;
  - `Contribution`;
  - conexões Open Finance;
  - contas bancárias, corretoras e contas manuais;
  - enums de classe de ativo, origem, moeda e status.
- As migrations históricas não representam sozinhas todo o schema atual; o Prisma continua sendo a fonte de verdade.

### Carteiras — CRUD e tracking Rust commitados

Disponível no Rust:

- `GET /v1/me/wallets`;
- `POST /v1/me/wallets`;
- `PATCH /v1/me/wallets/:id`;
- `DELETE /v1/me/wallets/:id`;
- isolamento por usuário;
- validação de endereços Ethereum, Polygon, Arbitrum, Base, Solana e Bitcoin;
- precisão decimal dos saldos;
- testes de contrato e de repositório.

Limites atuais:

- Rotas Rust `GET /v1/me/wallets/:id/assets` e `POST /v1/me/wallets/:id/sync`, Alchemy, CoinGecko, cache Redis e `wallet_assets` estão commitados (`edfc42c`, `59d1905`). O frontend foi conectado em `865f544`. Esta tarefa valida repositórios/cache isolados, sem afirmar nova sincronização Alchemy real.
- O Fastify legado continua presente para compatibilidade durante a migração.
- A migração dos demais domínios ainda não foi concluída.

### Mercados e ícones — implementado no Rust

Rotas Rust disponíveis:

- `GET /v1/market/coins`;
- `GET /v1/market/coins/:id`;
- `GET /v1/market/trending`;
- `GET /v1/market/spot`.

Características verificadas:

- CoinGecko como provedor de cripto;
- Redis para cache;
- mercados com cache de 60 segundos;
- tendências com cache de 300 segundos;
- ícones com cache de 86400 segundos;
- fallback Trust Wallet para cripto;
- extensão isolada para FMP, ainda não conectada às rotas;
- ações, ETFs, índices, câmbio e commodities continuam demonstração.

O frontend consulta o Rust através de rotas servidoras do Next e usa polling de 60 segundos para mercados.

### Notícias — implementado no Rust

- `GET /v1/news` exige autenticação.
- Filtros atuais: categoria (`all`, `crypto`, `macro`) e idioma (`all`, `pt`, `en`).
- Fontes atuais: RSS e NewsData opcional.
- Notícias não são persistidas no PostgreSQL.
- Cache principal de 300 segundos, com fallback stale de até 3600 segundos.
- O limite de uso do NewsData é controlado por Redis e Lua.

O plano antigo que citava CryptoPanic e NewsAPI não representa a implementação atual.

### Contas e aportes — API persistente e interface conectada

Existe uma primeira etapa funcional de interface:

- `/dashboard/accounts`;
- `/dashboard/accounts/[accountId]`;
- contas agrupadas por tipo;
- contas/holdings/aportes consultados pelas APIs privadas;
- mutações de contas e aportes persistentes;
- valores de contas WALLET/USD ainda indisponíveis na consolidação antiga de `AccountsProvider`.

Limites importantes:

- `AccountsProvider` usa `usePrivateFetch` e React Query; alterações são persistidas pelo Rust.
- A overview antiga ainda contém câmbio demonstrativo; a nova visão está condicionada ao gate descrito abaixo.
- Rust agora oferece `/v1/accounts` e rotas de holdings/contribuições com PostgreSQL, isolamento por usuário e decimais em strings; contrato em `docs/05-api-contas-persistentes.md`.
- A nova consolidação do backend é separada do cálculo antigo do provider de contas; não declara carteiras/USD como zero para completar um total.

### Watchlists — API Rust e cliente conectado

- As rotas persistentes de watchlists foram entregues no commit `ba12581`; contrato em `docs/06-watchlists-rust.md`.
- Cliente privado conectado em `865f544`; a validação operacional de produção não é inferida de testes locais.

### Portfolio — primeira consolidação Rust; exchanges permanecem no legado

O Fastify legado possui partes de:

- snapshots de portfólio;
- conexões de exchanges;
- serviços de carteira.

A consolidação BRL e snapshots novos são aditivos no Rust; os snapshots USD do Fastify não foram migrados. Exchanges continuam pendentes. Ordem histórica de referência:

1. user;
2. wallet;
3. watchlist;
4. wallet-asset;
5. portfolio-snapshot;
6. exchange-connection.

## 4. O que ainda é mock, demonstração ou não entregue

- O tracking e o cliente privado estão commitados; sua existência não comprova operação real de todos os ativos/redes.
- A nova overview real fica desligada até validar o contrato com contas reais e carteira sincronizada com saldo não zero.
- Gráficos avançados com Lightweight Charts ainda não foram confirmados como integrados.
- Não há WebSocket nativo implementado; o frontend usa polling.
- A página de ativo `/coin/[slug]` e histórico completo ainda são etapas futuras.
- Exchanges continuam fora do fluxo principal e devem permanecer somente leitura.
- Ações e ETFs não são dados reais no frontend atual.
- Não há write-layer, assinatura de transações ou custódia.

## 5. Regras operacionais permanentes

1. Usar `packages/database/prisma/schema.prisma` como fonte única do schema.
2. Usar PostgreSQL direto na porta `5432`; nunca configurar o pooler `6543`.
3. Manter frontend, Fastify legado e Rust com contratos compatíveis durante a migração.
4. Não direcionar todo o frontend para Rust sem migrar e validar o domínio correspondente.
5. Controllers/rotas não devem chamar provedores externos diretamente; usar services e infraestrutura.
6. Manter separação entre domínio, aplicação, infraestrutura e camada HTTP no Rust.
7. Não expor chaves de provedores ao navegador.
8. Blockchain permanece somente leitura no backend.
9. Não afirmar que uma integração real existe quando ela está apenas mockada ou disponível no legado.
10. Preservar estados de carregamento, erro, retry, vazio, fallback e paginação quando aplicáveis.
11. Manter documentação em português em `docs/`.
12. Antes de cada tarefa, confirmar se o domínio pertence ao Fastify, ao Rust ou a ambos.

## 6. Validação esperada

Para alterações TypeScript:

```sh
pnpm_config_verify_deps_before_run=false pnpm typecheck
pnpm_config_verify_deps_before_run=false pnpm build
```

Se o ambiente impedir a escrita de arquivos incrementais, usar a verificação do frontend com `tsc --noEmit --incremental false`.

Para alterações Rust, executar dentro de `apps/api-rust`:

```sh
cargo check --workspace --locked
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
```

Se `target/` estiver read-only, usar um `CARGO_TARGET_DIR` temporário e registrar essa limitação no resultado, sem confundi-la com falha de compilação.

## 7. Próximas prioridades recomendadas

1. **Entregue em código:** API persistente de contas, holdings e contribuições; cliente conectado em `865f544`.
2. **Definido e implementado:** vínculo exclusivo com carteira do mesmo usuário, holdings separados de wallet-assets e aportes exclusivamente explícitos.
3. **Backend entregue:** watchlists no Rust com as mesmas tabelas, rotas e DTOs do Fastify; ver `docs/06-watchlists-rust.md`.
4. **Próximo passo:** validar operacionalmente a nova consolidação BRL com contas reais, carteira não vazia, PTAX/CoinGecko e duas sessões antes de habilitar a overview.
5. Consolidação e snapshots Rust implementados nesta etapa; aplicar a migration aditiva no ambiente alvo antes da ativação. Ver `docs/08-ptax-e-consolidacao.md`.
6. Só depois avançar para exchanges, gráficos avançados e tempo real.

O critério de prioridade é entregar primeiro dados reais, seguros e persistentes para o acompanhamento do portfólio; novas integrações não devem aumentar o escopo antes de os contratos e estados existentes estarem validados.

## 8. Arquivos de referência

- `README.md` — visão geral do repositório.
- `apps/api-rust/README.md` — contratos e operação do backend Rust.
- `packages/database/prisma/schema.prisma` — schema oficial.
- `docs/01-renomeacao-astra.md` — histórico da renomeação.
- `docs/02-contas-e-aportes.md` — implementação da primeira etapa de contas/aportes.
- `docs/03-deploy-gratuito.md` — hospedagem e limitações do deploy.
- `docs/05-api-contas-persistentes.md` — contratos Rust e relação entre contas e carteiras.


## 9. PTAX e visão patrimonial — etapa de 10/10/2026

Contrato completo, políticas e gate: [08-ptax-e-consolidacao.md](08-ptax-e-consolidacao.md).

- PTAX USD/BRL de venda/fechamento, data atual ou histórica, cache Redis, data/fonte explícitas, recuo de até 14 dias, ausência e fallback desatualizado sinalizados.
- Rust consolida holdings BRL/USD e carteiras únicas com `BigDecimal`; cripto de carteira usa CoinGecko BRL. Transferências/sync não geram aportes. Taxas históricas informadas são preservadas.
- Snapshots JSONB novos, por usuário, sem reescrita dos snapshots USD legados e sem reconstrução fictícia do passado.
- Overview real preparada com erro/retry, vazio, subtotal parcial e fontes. `PORTFOLIO_REAL_OVERVIEW_ENABLED=false` por padrão; não ativada em produção.
- Fastify preservado; nenhum deploy manual ou alteração em `.github/workflows/`.

### Evidência de validação desta entrega

- **Aprovados:** `npm ci`; restauração do workspace com `pnpm install --frozen-lockfile`; `npm run lint`, `npm run typecheck`, `npm test` (24 testes) e `npm run build`, nessa ordem. Os scripts atuais de lint executam TypeScript (`tsc --noEmit`).
- **Rust aprovado:** `cargo check/build/test --workspace --locked`, Clippy com `-D warnings`, `cargo fmt --all --check`. Suíte padrão: 80 aprovados, 14 opt-in ignorados.
- **Opt-in executados separadamente:** 6 testes de PostgreSQL/Redis (contas, carteiras, tracking, caches e consolidação/snapshots) e 2 testes PTAX reais, todos aprovados. As migrations completas passaram no banco descartável.
- **PTAX real:** consulta histórica para 2024-01-07 retornou fechamento de 2024-01-05; consulta atual em 2026-10-10 retornou 2026-10-09, taxa de venda `4.98920` BRL/USD. É evidência datada, não uma cotação permanente.
- **Não executados nesta tarefa:** 6 opt-in restantes da suíte (gate multiusuário completo, repositórios de usuários/watchlists, cache de notícias e os dois testes live Alchemy/CoinGecko). A nova consolidação tem testes próprios de isolamento no PostgreSQL e contrato autenticado. Navegador autenticado e patrimônio não zero com provedores reais permanecem sem validação operacional.
- **Falhas corrigidas:** espaços OData `+` → `%20`; aviso Clippy sobre clone em slice; formatação Rust; declaração duplicada durante adaptação ao contexto de sessão.
- **Ambiente:** sockets e pnpm dentro do sandbox impediram/travaram verificações iniciais; reexecuções locais permitidas passaram. Rustfmt/Clippy foram extraídos em `/tmp`, sem instalação no sistema. Nenhum teste declarado aprovado com base em execução interrompida. Não restou bloqueio ambiental para as verificações executadas.
- **Dependências:** `npm ci` apontou duas vulnerabilidades no conjunto instalado pelo lockfile npm (uma moderada e uma crítica). Nenhum `audit fix` ou atualização ampla de dependências foi aplicado nesta tarefa; esse aviso não corresponde à auditoria do grafo pnpm completo.
- **Publicação:** commits pequenos e push na `main` autorizados para esta entrega. Nenhum deploy manual, migration no banco do projeto ou ativação da flag de produção. Confirmar o SHA remoto no Git para verificar a publicação.
