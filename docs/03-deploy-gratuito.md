# Deploy inicial: Vercel + Render + Supabase

## Serviços

- Vercel: frontend Next.js em `apps/web`.
- Render: API Rust em `apps/api-rust` e Key Value compatível com Redis.
- Supabase: autenticação e PostgreSQL. Reutilizar o projeto existente, sem apagar dados.

`render.yaml` define explicitamente `plan: free` para a API e o cache.
A API gratuita suspende após 15 minutos ociosa e pode demorar cerca de um minuto
para responder novamente. O cache gratuito não é persistente. Dados de usuários
ficam no PostgreSQL, nunca no filesystem do Render.

## 1. Supabase

Reative o projeto, se estiver pausado. Em Settings > General, ajuste o nome de
exibição para `astra wealth`; o identificador e a URL do projeto continuam iguais.

Para o backend, copie em Connect a conexão **Session pooler**, porta **5432**,
compatível com IPv4. Preencha a senha apenas no painel Render e use SSL
(`sslmode=require`). Não use Transaction pooler na porta 6543: a API o rejeita.

Antes de aplicar migrations em um banco existente, confira as tabelas e o histórico
`_prisma_migrations`. Não use `db push`, reset ou seed de demonstração em produção.
Após confirmar que o histórico corresponde às migrations do repositório, execute
`pnpm db:migrate:deploy` com `DATABASE_URL` configurada num ambiente privado.
As migrations não são executadas automaticamente pelo container da API.

As tabelas usadas apenas pela API devem ficar protegidas contra acesso direto pela
Data API (RLS sem políticas públicas ou schema não exposto). A API conecta via SQL.

## 2. Vercel

Importe o repositório GitHub e use a branch `main`:

| Campo | Valor |
| --- | --- |
| Framework | Next.js |
| Root Directory | `apps/web` |
| Node.js | 24.x |
| Build Command | `pnpm build` |
| Install Command | `pnpm install --frozen-lockfile` |

Habilite **Include source files outside of the Root Directory in the Build Step**
para resolver `packages/shared`. O `packageManager` da raiz fixa pnpm 10.
Não use o build da raiz: ele também compila o backend legado.

Configure as variáveis de `apps/web/.env.example`:

- `NEXT_PUBLIC_SUPABASE_URL`: URL do projeto Supabase.
- `NEXT_PUBLIC_SUPABASE_ANON_KEY`: chave pública publishable ou anon.
- `NEXT_PUBLIC_API_URL`: origem HTTPS da API Render, sem `/v1` no final.
- `MARKET_API_URL` e `NEWS_API_URL`: a mesma origem HTTPS da API Render.

Nunca coloque `SUPABASE_SERVICE_ROLE_KEY` em variáveis `NEXT_PUBLIC_*`.
Ao mudar uma variável pública, faça um novo deploy, pois ela entra no bundle.

## 3. Render

Em **New > Blueprint**, conecte o repositório e selecione `main`.
Revise os dois serviços gratuitos de `render.yaml` antes de criar.
Preencha os campos solicitados:

| Variável | Valor |
| --- | --- |
| `DATABASE_URL` | Session pooler Supabase, porta 5432, com SSL |
| `SUPABASE_URL` | URL do mesmo projeto usado pelo frontend |
| `SUPABASE_SERVICE_ROLE_KEY` | Chave service_role, somente no backend |
| `WEB_ORIGIN` | Origem exata do site Vercel, com HTTPS, sem barra final |

O Blueprint conecta `REDIS_URL` automaticamente pelo endereço privado do cache.
O Render fornece `PORT`, e a API escuta em `0.0.0.0`.
Se o projeto ainda assinar JWTs com HS256, adicione `SUPABASE_JWT_SECRET` no painel;
projetos com chaves assimétricas usam JWKS automaticamente.
CoinGecko funciona sem chave, sujeito aos limites públicos. Notícias RSS não
precisam de `NEWSDATA_API_KEY`; não habilite esse provedor no cache volátil sem
reavaliar o controle de orçamento, pois os contadores podem ser perdidos.

## 4. URLs e validação

Após obter os endereços reais, complete as variáveis nos dois painéis e redeploy.
No Supabase Auth, configure Site URL com a origem Vercel. O login por senha já
existe; Google OAuth depende de configuração própria e callback e não deve ser
considerado pronto por este deploy.

- `/` abre `/signin`; após login, a aplicação abre `/dashboard`.
- Overview e contas/aportes continuam em demonstração. O dashboard não ganhou
  proteção de rota nova nesta configuração de deploy; dados reais da API exigem JWT.
- `/health` deve retornar `status: ok`, `database: ok` e `redis: ok`. Atualmente
  o HTTP 200 também pode indicar `degraded`, então confira o corpo da resposta.
- Teste Markets e notícias autenticadas após a API acordar.
- Não declare o deploy concluído antes de testar as URLs públicas e o login.

## Referências

- https://vercel.com/docs/monorepos
- https://render.com/docs/blueprint-spec
- https://render.com/docs/free
- https://supabase.com/docs/guides/database/connecting-to-postgres
