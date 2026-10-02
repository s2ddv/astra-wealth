# 01 — Renomeação para Astra Wealth

## Nomes internos

A aplicação usa Astra Wealth, os pacotes usam `@astra-wealth/*` e os crates Rust
usam `astra-*`. O binário HTTP é `astra-api`. Os diretórios `apps/api` (Fastify)
e `apps/api-rust` (Axum) continuam separados.

## Compatibilidade preservada

Banco, usuário e senha PostgreSQL nos exemplos e no Compose permanecem iguais.
O healthcheck usa esses mesmos identificadores. Os IDs de usuários demo e seu
e-mail existente também são preservados; mudar a marca não deve criar outra
identidade de autenticação. As fixtures continuam cobrindo essa compatibilidade.

A watchlist é gravada em `astra.markets.watchlist.v1`. Na primeira visita, se a
nova chave não existir, a aplicação lê e normaliza a chave anterior. A cópia antiga
é mantida para permitir voltar à versão anterior. Uma chave nova já existente
sempre tem prioridade. A migração local só funciona na mesma origem; trocar o
domínio exige exportação/importação dos dados do navegador.

As migrations Prisma existentes não foram alteradas. Lockfiles foram atualizados
pelos gerenciadores. Nenhum segredo, projeto remoto, domínio ou pasta raiz foi
renomeado automaticamente.

## Ações externas a coordenar

1. Renomear manualmente a pasta raiz e atualizar atalhos e caminhos de execução.
2. Ajustar nomes visíveis de projetos e serviços em Supabase, Vercel e Railway.
   IDs, URLs e credenciais existentes continuam válidos até uma migração explícita.
3. Configurar domínio/DNS/certificados e redirects do domínio anterior.
4. Revisar Site URL e redirect URLs de autenticação, OAuth e links de e-mail.
5. Conferir `NEXT_PUBLIC_*`, `MARKET_API_URL`, `NEWS_API_URL`, CORS, URLs de banco
   e variáveis de cada ambiente. A alteração da marca não muda os nomes dessas
   variáveis. Não substitua valores de produção automaticamente.
6. Atualizar comandos de deploy que chamam o binário ou pacotes antigos.
7. Ao mudar a pasta raiz ou o nome do projeto Compose, preservar o projeto Compose
   existente (`-p`) ou mapear explicitamente os volumes já usados. Caso contrário,
   o Compose pode criar volumes vazios em vez de reutilizar os dados.
8. Se renomear o repositório remoto, atualizar o remote Git somente depois disso.

## Validação

```sh
pnpm install
pnpm turbo run lint typecheck build
cd apps/api-rust
cargo check --workspace --locked
cargo test --workspace --locked
```

Na raiz, `docker compose config` verifica a configuração local. O override Rust
requer `apps/api-rust/.env`; use também `-f apps/api-rust/compose.yml` para validá-lo.
Os testes de integração marcados `ignored` exigem PostgreSQL ou Redis isolados.
