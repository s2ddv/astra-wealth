# Astra Wealth — Status Atual e Referência Operacional

> Documento de referência para o trabalho no repositório. Atualizado em 3 de outubro de 2026.
>
> Este arquivo complementa o roadmap histórico e deve ser consultado antes de iniciar novas tarefas. Ele descreve o estado verificado do checkout atual, não apenas a intenção futura do produto.

## 1. Checkout e estado de referência

- Repositório local: `/home/samuel-barbosa/Documentos/astra-wealth`.
- Branch verificada: `main`.
- Estado nesta auditoria: `main` está 1 commit à frente de `origin/main` (`487e8f2`); há alterações locais ainda não commitadas em Rust, Fastify, frontend, Prisma e configuração.
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

### Carteiras — CRUD migrado; acompanhamento on-chain em desenvolvimento local

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

- O checkout local agora contém rotas Rust `GET /v1/me/wallets/:id/assets` e `POST /v1/me/wallets/:id/sync`, com implementação Alchemy, preços CoinGecko, cache Redis e persistência de `wallet_assets`. Essas alterações ainda não foram commitadas; não há nesta auditoria comprovação de consulta com chave real do provedor.
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

### Contas e aportes — API Rust persistente, integração da interface pendente

Existe uma primeira etapa funcional de interface:

- `/dashboard/accounts`;
- `/dashboard/accounts/[accountId]`;
- contas agrupadas por tipo;
- holdings e histórico demonstrativos;
- registro de aportes e retiradas em memória;
- KPIs compartilhados entre a visão de contas e a overview.

Limites importantes:

- Os dados ainda são mocks/estado local.
- Recarregar a página descarta alterações feitas na interface.
- Rust agora oferece `/v1/accounts` e rotas de holdings/contribuições com PostgreSQL, isolamento por usuário e decimais em strings; contrato em `docs/05-api-contas-persistentes.md`.
- A próxima implementação desse domínio deve substituir os mocks por API sem misturar a lógica de apresentação com persistência.

### Watchlists — API Rust migrada; cliente ainda pendente

- As rotas persistentes de watchlists foram entregues no commit `ba12581`; contrato em `docs/06-watchlists-rust.md`.
- A troca do cliente do frontend para essas rotas ainda precisa ser validada por domínio.

### Portfolio e exchanges — ainda não migrados para Rust

O Fastify legado possui partes de:

- snapshots de portfólio;
- conexões de exchanges;
- serviços de carteira.

Esses domínios ainda não devem ser considerados migrados para Rust. A ordem de migração prevista continua sendo:

1. user;
2. wallet;
3. watchlist;
4. wallet-asset;
5. portfolio-snapshot;
6. exchange-connection.

## 4. O que ainda é mock, demonstração ou não entregue

- O acompanhamento on-chain no Rust está implementado somente no checkout local e ainda requer validação com provedor real e integração da interface.
- A página de contas/aportes ainda não persiste dados.
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

1. **Backend entregue:** API persistente de contas, holdings e contribuições; conexão da interface permanece em tarefa separada.
2. **Definido e implementado:** vínculo exclusivo com carteira do mesmo usuário, holdings separados de wallet-assets e aportes exclusivamente explícitos.
3. **Backend entregue:** watchlists no Rust com as mesmas tabelas, rotas e DTOs do Fastify; ver `docs/06-watchlists-rust.md`.
4. **Próximo passo:** validar as alterações locais de wallet-assets e sincronização on-chain no Rust, executar um teste com chave Alchemy real em ambiente controlado e então consolidar um commit próprio. Depois, conectar a interface ao contrato Rust por domínio.
5. Implementar snapshots e cálculo persistente do portfolio.
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
