# Retirada do Fastify: inventário e gates

## Base e decisões (22/10/2026, antes de remover arquivos)

Base `c38ed49254fe5fc398e5f35898fcdf452ff4cd5f`, branch isolada
`codex/fastify-retirement-audit`. `zora-wealth` resolve para `astra-wealth`.
A main tinha alterações em README, status, documentação PTAX/isolamento,
teste portfolio_repository e arquivos não rastreados `.codex/`, docs/sql,
docs/10 e scripts/test-portfolio-release.sh. Não foram copiadas nem alteradas.
Esta entrega precisa ser integrada sem sobrescrever esse trabalho paralelo.

Decisões explícitas do usuário após apresentação do inventário:
- Migrar a consulta pública Alchemy/Moralis.
- Preservar API USD em caminho separado: `/v1/me/portfolio/snapshots-usd`.
- Descontinuar somente o repositório inativo de exchanges; preservar schema,
  registros, relações e mocks demonstrativos. Não criar API de exchanges.

## Matriz dos 14 contratos registrados

Fontes Fastify: `apps/api/src/server.ts`, `modules/*/*.routes.ts`,
`modules/me/me.schema.ts`, serviços, repositórios e `lib/mappers.ts`.
`app.ts` só monta health e não é o entrypoint de produção. HEAD é implícito
nos GET; OPTIONS/preflight é fornecido por CORS. Não há rota HTTP de exchanges.

Todos os `/me` autenticam antes de executar o serviço. Fastify usa getUser
Supabase e upsert por authId; Rust verifica JWT/usuário e resolve a mesma identidade
local. O proprietário vem da sessão, nunca de userId no corpo. Ambos fecham
configuração parcial e só permitem impersonação com NODE_ENV=development e
ALLOW_DEV_AUTH=true; Authorization fornecido não cai no modo dev. Erros 401
mantêm `error`; o modo dev Rust pode incluir `hint`. Não há cache público de
respostas privadas. IDs não vazios não são necessariamente UUIDs (há IDs legados).

| Método/caminho Fastify | Entrada, resposta e erros | Banco/cache no Fastify | Rust existente / destino e classificação inicial |
|---|---|---|---|
| GET `/v1/health/` | Público; 200 `{status,service,timestamp,checks:{database,redis}}`, inclusive degradado | SELECT 1 e PING, sem escrita | Coberto por health_contract; aliases `/health`, `/v1/health`, `/v1/health/`; executar gate |
| GET `/v1/market/spot` | Público; limit numérico (default 10, legado não limita); array camelCase com id/symbol/name/image/currentPrice/priceChangePercentage24h/marketCap; 502 `{error}` | CoinGecko USD ordenado por variação 24h; Redis `market:spot:<limit>` 60s; sem banco | Rust existe mas ordena por market cap, aceita 1..100, rejeita queries extras, campos anuláveis/aditivos e 503+Retry-After em rate limit. Pendente alinhar ordenação e consumidor antigo; não declarar igualdade total |
| GET `/v1/wallet/:address` | Público; EVM 0x+40 hex; 400 Invalid Ethereum address; 200 `{address,alchemy,moralis,fetchedAt}`; cada fonte `{address,nativeBalance,source}` ou null; 502 Failed to fetch wallet data | ETH nativo por Alchemy e Moralis em paralelo; ambas falhas ainda dão 200 com nulls; Redis `wallet:<lowercase>` 30s; sem banco | Ausente. Migrar com mesmo formato, falha parcial e cache; teste de provedores simulados. Não confundir com tracking privado |
| GET `/v1/me/wallets` | 200 WalletDto[] com assets e amount string; 401/500 | Filtro userId, mais recentes primeiro | Coberto por wallet_contract e repositório Rust; executar gate |
| POST `/v1/me/wallets` | address EVM mesmo para outras chains no legado; chain default ETHEREUM; nickname trim 1..64; 201 WalletDto; 400 erro Zod flatten, 409 duplicada | Insert com normalização EVM e sync ETH imediato, não atômico com insert | Rust valida por rede, sem sync no POST. Web já usa criação e leitura/sync separados. Preservar esse fluxo Rust e documentar diferença; não copiar o bug legado de converter ETH decimal por BigInt como wei |
| DELETE `/v1/me/wallets/:id` | 204; 400 id; 404 ausente/alheia; 409 `{code:WALLET_LINKED_TO_ACCOUNT,error}` | Lock da carteira com userId, bloqueia vínculo com conta; delete/cascade assets | Coberto por wallet_contract e SQLx; executar isolamento |
| POST `/v1/me/wallets/:id/sync` | Legado 200 WalletDto; 400 id; 404 ausente/alheia; demais falhas 500 | Busca owner antes do provedor; ETH nativo apenas; upsert assetKey native, marca sync/valuation incompletos | Rust retorna WalletAssetsResponse, EVM nativo+tokens/valoração; 404/422/429/502/503 com code. Cache por usuário/carteira, TTL 30s, intervalo sync 15s e lease 90s. Cliente useWallets já espera formato Rust. Testar owner antes do provedor/cache |
| GET `/v1/me/watchlists` | 200 WatchlistDto[] `{id,name,userId,items,createdAt,updatedAt}`; item `{id,coinId,addedAt}` | Filtro userId, createdAt desc; sem cache | Coberto por watchlist_contract e SQLx; Rust ordena itens explicitamente, legado não garantia ordem |
| POST `/v1/me/watchlists` | name trim 1..64 UTF16; 201 DTO; 400 flatten; 409 Watchlist name already exists | Insert userId, unique por nome/owner | Coberto; executar testes |
| DELETE `/v1/me/watchlists/:id` | 204; 400 id; 404 Watchlist not found | Delete id+owner e cascade itens | Coberto; executar testes |
| POST `/v1/me/watchlists/:id/items` | coinId trim 1..128; 201 item; 400 flatten; 404 owner/lista; 409 Coin already in watchlist | Confere owner, insere item; Rust acrescenta lock transacional | Coberto; executar testes |
| DELETE `/v1/me/watchlists/:id/items/:coinId` | 204; 400 params; 404 Watchlist or item not found | Confere owner e deleta por lista+coin; Rust usa DELETE JOIN owner | Coberto; executar testes |
| GET `/v1/me/portfolio/snapshots` (USD) | limit coercível inteiro 1..100 default 30; 200 `{id,totalUsd:string,userId,createdAt}[]`; 400 flatten | portfolio_snapshots por userId, createdAt desc; sem cache | Pendente migrar para snapshots-usd. Mesmo caminho Rust atual lê portfolio_valuations BRL e ignora limit: NÃO é substituto |
| POST `/v1/me/portfolio/snapshots` (USD) | totalUsd número >=0; 201 DTO USD; 400 flatten; source não exposto | Insert portfolio_snapshots NUMERIC(18,2), source WALLET_REFRESH, owner autenticado | Pendente migrar para snapshots-usd. POST BRL calcula no servidor e retorna `{id,snapshot}`; manter separado e não converter histórico |

Todos os erros não tratados do Fastify usavam envelope padrão 500; implementações
Rust existentes sanitizam falhas. Erros de parsing de corpo têm 400/413/415;
mensagens internas do framework não são contrato a copiar. Endereços e montantes
públicos devem continuar no formato esperado; usar aritmética decimal evita a
perda de precisão Number do legado, sem alimentar o tracking com ETH como wei.

## Comportamentos sem rota e consumidores

| Item real | Classificação e destino |
|---|---|
| ExchangeConnectionRepository: list/byId/byExchange/create/updateCredentials/delete | Candidato aprovado para descontinuação; nenhum import. Não havia criptografia implementada aqui: recebia strings já cifradas. Não criar contrato fictício |
| UserRepository, WalletRepository.updateNickname, WatchlistRepository.rename e helpers não chamados | Não são rotas; user/nickname já cobertos no Rust, rename de watchlist não exposto. Remover só junto ao pacote após gates |
| WalletAssetRepository | Escrita legada ETH; Rust tracking já escreve com owner, lock e precisão. Preservar tabelas e schema |
| `useWalletSummary` → `lib/api.ts` → `/v1/wallet/:address` | Pendente migrar origem para Rust após novo contrato; WalletCard não é importado e usa address=null, mas API pública será preservada por decisão |
| `useSpot` → `lib/api.ts` → `/v1/market/spot` | CoinsCard não é importado; adaptar nulabilidade para contrato Rust sem quebrar uso futuro |
| `apiFetchWithAuth` | Sem consumidor encontrado; helper compartilha origem; não confundir com cliente privado ativo |
| `useWallets`, WalletsView, watchlists-api, AccountsProvider e private-session-context | Cobertos pelo proxy privado Next com allowlist, bearer da sessão, origem nas mutações e no-store; verificar testes JS e Rust |
| `RealPortfolioOverview`, private-routes e tests/portfolio.test.mjs | Contrato BRL ativo sob flag; preservar caminho e comportamento. Novo USD deve ter teste separado |
| `packages/shared/src/database.ts` PortfolioSnapshotDto/CreateSnapshotInput; database/prisma/seed.ts | USD ainda definido e seed grava USD. Preservar DTO/schema/seed e apontar documentação para caminho novo |
| settings/page.tsx, mocks/settings.ts, types/dashboard.ts | Exchange demonstrativa, não chama API. Preservar |
| market/news rotas servidoras Next | Já apontam Rust 3334; preservar |
| `apps/web/next.config.ts` | Só transpilePackages/useTypeScriptCli, nenhuma referência 3333; não alterar |
| root package.json/pnpm-lock.yaml | Dev/build/lint/typecheck/test incluem Fastify. Remover filtro/importer/deps somente após gates. Rust é Cargo, não pacote pnpm |
| turbo.json | Revisar tarefas reais TS: generate/build/lint/typecheck/test/dev; Cargo separado, sem pacote fictício |
| scripts/test-security-isolation.sh | npm test inclui Fastify e teste TS roda após Rust. Substituir por testes Rust equivalentes, incluindo novos USD; usar somente banco/cache descartáveis |
| scripts/test-manual-operations.sh | Já Rust; preservar. Script portfolio-release só existe não rastreado na main e fica fora desta branch |
| .env.example | Root Fastify PORT=3333 e NEXT_PUBLIC_API_URL=3333; atualizar orientação quando migrado; corrigir exemplo antigo de pooler transacional |
| apps/web/.env.example | Variáveis de domínio já 3334; documentar origem pública e USD. Não ler/copiar/editar .env.local |
| apps/api-rust/.env.example | PORT=3334; MORALIS_API_KEY passará a ter consumidor real; documentar |
| README, docs/01,03,04,06,07,08, API Rust README | Referências operacionais atualizar; evidências históricas devem permanecer datadas, não reescrever validações antigas |
| render.yaml, Dockerfile/compose Rust, docker-compose.yml | Rust já hospedado; sem deploy, sem workflows, sem DDL externo |

## Ordem de execução e gates

1. Migrar serviços/infra/rotas públicas e USD sem alterar schema. Testar autorização,
   query/body, formatos, precisão, fonte parcial e erros de cache/provedor.
2. Adaptar consumidor público e referências de ambiente; manter proxy BRL.
3. Validar contratos, repositórios e isolamento em PostgreSQL/Redis descartáveis.
   Testar origem/preflight no binário atual; CORS só muda se houver falha comprovada.
4. Revisar `git ls-files apps/api` (33 arquivos na base), então `git rm -r apps/api`
   apenas se contratos e gates permitirem. Não apagar tabelas, migrations ou dados.
5. Ajustar workspace/lock/scripts/docs. `pnpm dev` supervisiona web e Cargo;
   Cargo check/build/test/clippy/fmt explícitos, sem fingir pacote pnpm Rust.
6. Rodar sequência AGENTS npm ci, lint, typecheck, test, build; restaurar workspace
   com pnpm frozen após npm ci. Registrar separadamente testes locais/simulados e
   Supabase/provedores reais/navegador ainda não comprovados.

Resultados de execução serão adicionados ao final; a existência de testes não é
prova de que passaram nesta entrega.

## Contratos migrados e diferenças deliberadas

- A consulta pública é `GET /v1/wallet/:address`, sem autenticação e sem
  persistência no banco. Respostas de cache mantêm inclusive o casing do endereço
  da primeira consulta, como no legado. Alchemy/Moralis usam timeouts, limite de
  resposta e seis casas ETH; falhas individuais viram null. Cache indisponível
  retorna 502. A conversão usa inteiros/decimais, sem reproduzir perda IEEE-754.
- `/v1/market/spot` mantém ordenação de variação 24h. `/v1/market/coins` continua
  por capitalização. As chaves de cache agora distinguem explicitamente a ordem.
  Continuam limites Rust de 1..100, campos anuláveis e tratamento de rate limit;
  o consumidor CoinsCard agora apresenta ausência como travessão.
- USD: GET/POST `/v1/me/portfolio/snapshots-usd`, Bearer obrigatório em produção.
  GET aceita `limit` inteiro decimal ou notação exponencial de 1 a 100 (default
  30); outras propriedades são ignoradas. POST recebe `{ "totalUsd": 123.45 }`.
  Resposta: `{id,totalUsd:string,userId,createdAt}`. userId/source do corpo não
  controlam persistência. Histórico continua em `portfolio_snapshots`, source
  WALLET_REFRESH em novos registros; nenhuma conversão ou reescrita de dados.
  Totais são arredondados a centavos como NUMERIC(18,2). Overflow e expoentes
  desproporcionais agora falham com 400 antes do banco; mensagens de validação
  mantêm envelope flatten, mas não copiam literalmente todos os textos Zod.
- BRL: GET/POST `/v1/me/portfolio/snapshots` permanece com avaliação calculada
  no servidor e tabela própria. Nenhum redirecionamento silencioso USD → BRL.
  Clientes externos da antiga API USD precisam mudar explicitamente de caminho.
  Nenhum consumidor web ativo de USD foi localizado; o proxy web BRL permanece.
- O POST de carteira Rust continua sem sync implícito. `useWallets`/WalletsView
  já usam as rotas privadas Rust e a resposta WalletAssetsResponse. Alchemy/
  CoinGecko privados não são substituídos pela consulta pública Alchemy/Moralis.
- Exchanges: somente código inativo aprovado para remoção; schema, relações,
  seed e dados permanecem. A tela não passa a oferecer uma integração real.

## Retirada autorizada após os gates

Em 23/10, após contrato, isolamento em PostgreSQL/Redis descartáveis, HTTP e
preflight aprovados e Clippy sem avisos, os 33 arquivos rastreados foram revisados
com git ls-files e retirados por git rm -r apps/api. Nenhum schema, migration,
workflow, arquivo de ambiente local ou serviço externo foi alterado. O CORS
existente passou no teste de origem permitida e origem não autorizada e foi mantido.

## Alterações por arquivo

| Arquivos | Alteração concreta |
|---|---|
| `crates/application/src/public_wallet.rs` | Serviço público, validação EVM, duas fontes em paralelo, cache 30s, conversão wei → ETH com seis casas sem float |
| `crates/infrastructure/src/public_wallet.rs` | Adaptadores HTTP Alchemy/Moralis com timeouts, tamanho limitado, URLs fixas e chaves apenas no servidor |
| `crates/api/src/public_wallet.rs` | Rota pública e respostas 400/502 compatíveis |
| `crates/application/src/usd_snapshots.rs` | Serviço USD separado, owner explícito, limites e arredondamento decimal |
| `crates/infrastructure/src/usd_snapshots.rs` | SQLx na tabela existente, leitura histórica, filtro userId e source preservado |
| `crates/api/src/usd_snapshots.rs` | GET/POST snapshots-usd, auth, limit, corpo JSON, no-store e erros flatten |
| `crates/{api,application,infrastructure}/src/lib.rs`, `crates/api/src/main.rs` | Exportação/composição dos serviços; bloco CORS mantido |
| `crates/domain/src/market.rs`, `crates/api/src/market.rs`, `crates/infrastructure/src/market.rs` | Ordenação spot 24h separada de market cap e chaves de cache distintas |
| `crates/api/tests/{public_wallet_contract,usd_snapshots_contract}.rs` | Novos contratos, validação antes de dependências, auth, cache e precisão |
| `crates/api/tests/multiuser_isolation.rs` | USD com dois JWTs sintéticos, userId forjado ignorado, leitura isolada e escrita inválida sem efeito |
| `crates/infrastructure/tests/usd_snapshots_repository.rs` | Leitura de registros antigos e verificação de source, limite, precisão e proprietário no PostgreSQL |
| `crates/infrastructure/tests/portfolio_repository.rs` | Permite apenas mais um nome de banco descartável local; mantém restrição localhost |
| `crates/api/tests/market_contract.rs`, `crates/infrastructure/src/market/tests.rs` | Ordem de spot e separação dos caches verificadas |
| `apps/web/src/lib/api.ts` | Fallback público 3333 → 3334 |
| `apps/web/src/hooks/useSpot.ts` | Tipo Rust real MarketDto, cancelamento e reset de erro |
| `apps/web/src/components/dashboard/CoinsCard.tsx` | Renderiza campos desconhecidos sem tratá-los como preços/saldos zero |
| `apps/api/**` | 33 arquivos rastreados removidos após revisão e gates; schema/dados não removidos |
| `package.json` | Dev conjunto e comandos Cargo separados; filtros Fastify retirados; testes do supervisor |
| `pnpm-lock.yaml` | Importer api e dependências exclusivas removidos; sem atualização ampla de versões |
| `turbo.json` | Build web sem cache ambiental implícito; tarefas test/dev declaradas; nenhum pacote Rust fictício |
| `scripts/dev.mjs`, `scripts/dev.test.mjs` | Supervisão POSIX dos processos e propagação de falhas/encerramento |
| `scripts/test-security-isolation.sh`, `scripts/test-rust-http.py` | Gate Rust sem Fastify, DB/cache descartáveis e HTTP/CORS real local |
| `.env.example`, `apps/{web,api-rust}/.env.example` | Origem Rust, Moralis público e instruções para configurar ambientes separadamente |
| `README.md`, `apps/api-rust/README.md`, `docs/{03,04,08}*.md` | Operação atual e separação de evidência histórica da retirada |

Caminhos `crates/...` nesta tabela são relativos a `apps/api-rust/`.
O diff completo da branch contra a base é a referência para as linhas exatas.
`apps/web/next.config.ts`, `.env.local`, schema Prisma, migrations e workflows
não têm alterações nesta entrega.

## Validação local e limites

- Sequência final AGENTS: npm ci; restauração pnpm frozen e geração Prisma;
  npm run lint; npm run typecheck; npm test; npm run build, todos aprovados.
  Testes JS: 7 shared + 11 web + 2 supervisor. Lint atual é tsc, não ESLint.
- Cargo check/build/test --workspace --locked aprovados. Suíte padrão:
  93 aprovados, 16 opt-in ignorados; isso não equivale a executar os opt-in.
  Clippy --all-targets com -D warnings e cargo fmt --all --check aprovados.
  Ferramentas rustfmt/Clippy 1.98.1 extraídas em /tmp, sem instalação no sistema.
- Gate de isolamento pré-retirada aprovado: PostgreSQL/Redis reais descartáveis,
  sete testes de repositório (user, wallet, tracking, accounts, watchlist, BRL e
  USD), teste multiusuário com JWTs sintéticos e smoke HTTP. Preflight permite
  a origem configurada, não autoriza a origem estranha e aceita authorization/
  content-type; CORS não precisou mudar.
- pnpm dev testado com web e Cargo reais: proxy web → Rust retornou 400 para
  limit=0 sem acessar provedor. Ao encerrar pnpm dev, portas 3000 e 3334 fecharam.
  Testes do supervisor confirmam encerramento do irmão em falha/exec ausente.
- Turbo dry-run reconhece somente database/shared/web e comandos existentes.
- Limitações: não foram usadas sessões Supabase reais, nem saldos on-chain reais,
  nem chamadas reais Alchemy/Moralis/CoinGecko. Não há prova de E2E autenticado
  em navegador, reconciliação do banco externo ou operação em produção.
- Correções durante validação: quota /tmp resolvida movendo o worktree para disco;
  testes de socket executados fora do sandbox; import MarketDto corrigido;
  banco descartável incluído na allowlist do teste BRL; aviso Clippy corrigido.
  A interrupção anterior de aprovação por limite de uso não executou a ação;
  a retomada concluiu o download de ferramentas oficiais necessário.
- Os arquivos README/status/isolamento e o teste BRL também têm trabalho paralelo
  na main. Esta branch não o sobrescreve; integrar as mudanças com revisão dessas
  regiões. Não houve push, deploy ou DDL externo.

### Resultado após a exclusão

`pnpm test:security` repetido após a retirada: aprovado, com 101 testes Rust
(93 padrão + 8 opt-in), 20 JS e smoke HTTP/CORS. Oito dos 16 testes opt-in
foram executados adicionalmente; os demais, incluindo provedores reais, não
foram promovidos a evidência. Nenhum contrato necessário ficou sem destino.
A classificação final é coberto e validado localmente no Rust para as 14
operações, com as diferenças de validação/formato e o novo caminho USD
explicitados acima; exchanges é descontinuação de código inativo aprovada.

Commits de implementação: `e9abe59` (contratos/consumidores/testes) e
`3dbb188` (retirada e execução). A documentação fica em commit separado.
O worktree está isolado na branch `codex/fastify-retirement-audit`, sem push.
