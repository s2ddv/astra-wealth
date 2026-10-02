# Astra Wealth — API Rust

Backend Rust/Axum em `apps/api-rust`, executado ao lado do Fastify em `apps/api`.
Portas padrão: Rust **3334**, Fastify **3333**. A migração dos demais domínios é
incremental; não redirecione todas as chamadas do frontend para Rust de uma vez.

## Arquitetura

- `astra-domain`: entidades, validação e contratos de repositório.
- `astra-application`: casos de uso, dependentes do domínio.
- `astra-infrastructure`: PostgreSQL/SQLx, Redis, autenticação e provedores HTTP.
- `astra-api`: rotas Axum e composição das dependências.

Use Rust 1.94+. SQLx 0.9 utiliza `runtime-tokio` e `tls-rustls-ring`, além de
`postgres` e `macros`; não use o antigo recurso `runtime-tokio-rustls`.

## Execução

Neste diretório:

```sh
cp .env.example .env
cargo run -p astra-api --bin astra-api
```

Ou, na raiz, com os serviços locais PostgreSQL/Redis:

```sh
docker compose -f docker-compose.yml -f apps/api-rust/compose.yml up --build api-rust
```

Use conexão PostgreSQL direta, porta **5432**. A porta **6543** do pooler
transacional é rejeitada. Configure autenticação em `apps/api-rust/.env`;
segredos nunca devem entrar na imagem. Os identificadores do banco existente
foram preservados durante a renomeação.

## Saúde e autenticação

`GET /v1/health`, `/v1/health/` e `/health` retornam `status`,
`service: "astra-wealth-api"`, `timestamp` UTC com milissegundos e
`checks: { database, redis }`. Saúde degradada continua retornando HTTP 200;
a prontidão do container verifica o campo `status`. As sondagens expiram em 5s.

O domínio de usuários espelha o schema Prisma: IDs TEXT, `authId` obrigatório,
`name` opcional e timestamps com milissegundos. O upsert de autenticação atualiza
e-mail e preserva ID local e nome. Sem e-mail, usa `user-{authId}@supabase.local`.
Não há rota pública de perfil `/v1/me` ou `/users`.

Use os mesmos `SUPABASE_URL` e `SUPABASE_SERVICE_ROLE_KEY` do legado. Projetos
HS256 também usam `SUPABASE_JWT_SECRET`; assinaturas assimétricas usam JWKS em
`/auth/v1/.well-known/jwks.json`, substituível por `SUPABASE_JWKS_URL`.
São aceitos HS256, RS256 e ES256, com validação de assinatura, expiração, issuer,
audience `authenticated` e subject. O cache JWKS dura 5 minutos; uma chave
desconhecida permite nova consulta no máximo a cada 30s. Requisições expiram em 5s.

Após validar o JWT, a API consulta `/auth/v1/user` antes de sincronizar o e-mail.
A service-role key é credencial de upstream, nunca segredo de assinatura de JWT
de usuário. `AuthenticatedUser` preserva os erros 401 do legado. Tokens inválidos
não recorrem ao usuário demo. Fora de produção, tokens ausentes podem usar
`DEV_USER_ID`; sem configuração Supabase, `X-User-Id` também funciona.
**Produção não inicia sem configuração Supabase.**

## Carteiras

Rotas autenticadas:

- `GET /v1/me/wallets`: carteiras e ativos do usuário local autenticado.
- `POST /v1/me/wallets`: criação com validação EVM, Solana ou Bitcoin.
- `PATCH /v1/me/wallets/:id`: alteração apenas do apelido.
- `DELETE /v1/me/wallets/:id`: remoção com filtro de proprietário.

Apelidos são aparados e limitados a 1–64 unidades UTF-16. Duplicatas retornam 409;
carteiras de outro usuário retornam 404 nas alterações. Quantidades são strings
decimais exatas. Esta etapa não consulta saldos on-chain nem expõe sincronização.
Watchlist, ativos, snapshots e conexões de exchanges seguem como próximos domínios.
Assinaturas de transações permanecem no cliente.

## Mercados e ícones

`crypto_market_services` compõe mercados e ícones com um cliente HTTP limitado e
cache Redis. CoinGecko fornece `/coins/markets` em USD, `/coins/{id}` e
`/search/trending`. `COINGECKO_API_KEY` é opcional e usa o header da API Demo;
a integração Pro não está configurada. A chave nunca chega ao navegador.

Rotas públicas de leitura:

- `GET /v1/market/coins?page=1&perPage=50`: `{ data, page, perPage, hasMore }`;
  página 1–10000, tamanho 1–100; `ids` aceita até 100 IDs validados e deduplicados.
- `GET /v1/market/coins/{id}`: preço, variação, capitalização, volume, ícone,
  posição e timestamp. Métricas ausentes permanecem `null`.
- `GET /v1/market/trending`: até 15 buscas populares, sem inferir preços USD de BTC.
- `GET /v1/market/spot?limit=10`: array compatível com o legado.

Consultas inválidas retornam 400, moeda ausente 404, erro de provedor 502 e limite
upstream 503 com `Retry-After: 60`. Cache de mercados: 60s; tendências: 300s.
Uma trava por processo agrupa preenchimentos concorrentes; um 429 gera pausa de
60s. A API não serve mercados expirados silenciosamente; entradas corrompidas são
recarregadas e hits válidos não estendem TTL.

Ícones usam os metadados já carregados, sem consultas adicionais CoinGecko.
Redis guarda resoluções por 86400s em `icon:crypto:id:{id}`,
`icon:crypto:token:{chain}:{address}`, `icon:crypto:symbol:{SYMBOL}` ou
`icon:stock:{TICKER}`. Solana preserva maiúsculas; EVM normaliza a chave.
Símbolos ambíguos são rejeitados; prefira ID ou rede/contrato.

O fallback Trust Wallet valida PNG, HTTP 200, assinatura PNG e limite de 256 KiB;
usam-se caminhos EIP-55 para EVM. Hosts fixos, redirects desativados e componentes
validados limitam as consultas. HTTP expira em 5s, Redis em 1s. Falhas retornam
`{ url: null, symbol }` e não são guardadas por 24h. PNGs são servidos como data
URLs, sem hotlink nem proxy público. A UI aceita `data:` e exibe iniciais se faltar
imagem. O provedor FMP existe como extensão isolada; não está conectado às rotas.

No web, configure `MARKET_API_URL` apenas no servidor (padrão local
`http://127.0.0.1:3334`). Markets consulta cripto a cada 60s e tendências a cada 5min,
com estados de erro e aviso de resposta anterior. Filtros valem para a página atual.
Watchlists locais têm limite de 100 ativos e migram IDs demo para IDs CoinGecko.
Ações, ETFs, índices, câmbio e commodities permanecem demonstração.

Referências: [CoinGecko markets](https://docs.coingecko.com/reference/coins-markets),
[detalhes](https://docs.coingecko.com/reference/coins-id),
[Trust Wallet](https://github.com/trustwallet/assets) e
[FMP](https://site.financialmodelingprep.com/developer/docs/company-image-api).

## Notícias

`GET /v1/news?category=all&lang=all&limit=20` exige autenticação existente.
`category`: `all|crypto|macro`; `lang`: `all|pt|en`; `limit`: 1–50; `cursor` é o
`nextCursor` da resposta anterior com os mesmos filtros. Retorna
`{ data: NewsArticleDto[], nextCursor: string | null, stale: boolean }`.
Consultas inválidas retornam 400; sem provedor nem cache anterior, 503.
Notícias não são persistidas no PostgreSQL.

`NEWSDATA_API_KEY` é opcional em `apps/api-rust/.env`, junto a `REDIS_URL` e auth.
Sem chave, usa somente RSS e registra um aviso. Nunca use `NEXT_PUBLIC_*` para ela.
O web usa sessão Supabase via `/api/news` e `NEWS_API_URL=http://127.0.0.1:3334`.

Listas de até 200 artigos usam `news:v1:{category}:{lang}` por 300s e
`news:v1:stale:{category}:{lang}` por 3600s, como fallback se todos os provedores
falharem. Consultas concorrentes compartilham o preenchimento; falhas RSS são
isoladas por fonte. A paginação usa offsets do snapshot em cache, que pode mudar
após TTL; clientes devem deduplicar IDs ao acrescentar páginas.

NewsData `/crypto` e `/market` buscam uma página de 10 artigos por endpoint, nos
dois idiomas, com cache de 1200s independente dos filtros. Lua no Redis reserva
créditos antes da chamada: até 180/dia UTC e 30 por janela móvel de 900s. Cada
endpoint tem pausa de 1200s, inclusive após erro, coordenada entre instâncias.
Duas chamadas bem-sucedidas a cada 20min consomem até 144 créditos/dia.
Falhas Redis desativam chamadas pagas. Use Redis persistente, dedicado e com
`maxmemory-policy noeviction`; apagar chaves, perder persistência ou compartilhar
a chave externa com outros clientes invalida a contabilização global.

Fontes e exclusões: [news-sources.md](docs/news-sources.md). Apenas resumos dos feeds
são sanitizados; páginas completas não são coletadas. O plano gratuito NewsData
pode atrasar artigos em aproximadamente 12h.

## Build, tipos e testes

Execute dentro de `apps/api-rust` para carregar `.cargo/config.toml`:

```sh
cargo check --workspace --locked
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
cargo run -p astra-api --bin export-types
```

O gerador escreve `UserDto.ts`, `WalletDto.ts`, `IconResponse.ts`, `MarketDto.ts` e
`NewsDto.ts` em `packages/shared/src/generated/rust`. Datas usam UTC e camelCase.
As chaves privadas das fixtures são públicas e exclusivas de testes.

SQLx usa macros e metadados `.sqlx` versionados; builds normais não exigem banco
ativo. Para regenerar, inicialize um PostgreSQL descartável com o schema Prisma
atual (`prisma db push --skip-generate`). As migrations históricas não representam
sozinhas todo o schema atual e não devem ser editadas.

```sh
# DATABASE_URL deve apontar apenas ao banco descartável.
cargo clean -p astra-infrastructure
SQLX_OFFLINE=false SQLX_OFFLINE_DIR="$PWD/.sqlx" cargo build --workspace
TEST_DATABASE_URL="$DATABASE_URL" cargo test -p astra-infrastructure --test user_repository -- --ignored
TEST_DATABASE_URL="$DATABASE_URL" cargo test -p astra-infrastructure --test wallet_repository -- --ignored
# Redis isolado para roundtrip, TTL e orçamento:
TEST_REDIS_URL=redis://127.0.0.1:56391 cargo test -p astra-infrastructure --test asset_icon_cache -- --ignored
TEST_REDIS_URL=redis://127.0.0.1:56401 cargo test -p astra-infrastructure --test news_cache -- --ignored
```

Os testes normais usam fixtures HTTP locais para auth, provedores, contratos,
limites, validação e fallback. Os testes de banco cobrem IDs TEXT, concorrência,
unicidade, isolamento de proprietário e precisão dos valores. Remova os serviços
descartáveis após a validação.
