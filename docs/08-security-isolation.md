> Atualização 23/10/2026: o gate atual é Rust-only e inclui snapshots USD,
> snapshots BRL e smoke HTTP/preflight. Não executa o antigo teste Fastify.
> As evidências Rust/Fastify abaixo são históricas. Consulte
> [11-retirada-fastify.md](11-retirada-fastify.md) para o estado desta retirada.

# Verificação P0 de isolamento multiusuário

As correções de autenticação e cache estão separadas da verificação integrada.
Os testes rápidos não substituem os testes com PostgreSQL e Redis reais.

## Correções implementadas

- Fastify e Rust exigem configuração Supabase completa. A exceção local exige
  `NODE_ENV=development` e `ALLOW_DEV_AUTH=true`; configuração parcial é erro.
- Authorization presente nunca permite fallback para um usuário de desenvolvimento.
- O helper legado Fastify aceita somente a identidade estabelecida pelo plugin.
- A mudança de identidade troca o QueryClient, recria os observadores e cancela
  chamadas privadas da sessão anterior. Etapas posteriores de uma operação usam
  o mesmo escopo cancelável. Eventos repetidos do mesmo usuário preservam o cache.

## Comando integrado

Após instalar dependências e gerar o cliente Prisma, executar:

```sh
npm run test:security
```

Pré-requisitos: Node 24+, pnpm 10, Rust compatível com o workspace, Podman ou
Docker, e imagens locais `postgres:16-alpine` e `redis:7.4-alpine` do Docker Hub.
O comando falha se algum requisito ou teste falhar; não converte indisponibilidade
do banco em aprovação. Não modifica workflows nem configurações publicadas.

O script cria containers novos, portas dinâmicas restritas a `127.0.0.1` e dados
efêmeros. Sobrescreve as URLs de banco para apontar exclusivamente a esses
containers. **Inicializa o schema vazio com `prisma db push`** a partir do Prisma
atual. Essa é uma alteração de schema, mesmo sem histórico de migrações; exige
autorização específica quando a tarefa proíbe migrações/alterações de schema.
Os containers são removidos ao finalizar, inclusive em caso de falha.

## Matriz

| Área | Verificação |
| --- | --- |
| Autenticação | Configuração ausente/parcial; flags de desenvolvimento; credenciais inválidas e headers conflitantes. |
| Identidade integrada | Dois JWTs sintéticos verificados pelo Rust; `authId` distinto de `User.id`; o Fastify resolve os mesmos IDs locais no mesmo banco. |
| Contas e filhos Rust | A/B preenchidos; GET/DELETE de conta alheia; POST/PUT/DELETE de filhos alheios; conta própria com filho de outro usuário ou de outra conta própria; vínculo a carteira alheia. |
| Carteiras Rust | Listagem e ativos separados; PATCH/DELETE/GET assets/POST sync cruzados retornam 404; estado persistido permanece igual. |
| Tracking | Cache Redis real aquecido para A/B; requisições negadas não chegam ao cache nem aos provedores simulados; respostas próprias preservadas. |
| Watchlists Rust/Fastify | Listas homônimas com itens distintos; alterações/exclusões cruzadas negadas; conteúdo preservado. |
| Fastify legado | Rotas reais, serviços e Prisma; carteiras/ativos criados pelo Rust visíveis somente ao dono; snapshots USD separados; campo `userId` e header forjados não trocam proprietário. |
| Frontend | Respostas atrasadas, callbacks antigos, logout, A→B→A, etapas subsequentes e cancelamento de queries usando TanStack real. |
| Repositórios | Execução explícita dos testes de banco de users, wallets, tracking, accounts e watchlists, normalmente ignorados na suíte rápida. |

Os testes HTTP usam os routers/handlers reais em processo. Supabase e provedores
externos são simulados; a persistência usa PostgreSQL e, no tracking Rust, Redis.
O teste Fastify roda após o Rust para verificar a associação de identidade entre
backends. Não executar esse arquivo isoladamente contra um banco existente.

## Resultado local em 2026-10-09

Após autorização específica para inicializar exclusivamente o schema
descartável, `npm run test:security` passou. Os cinco testes de repositório
listados acima foram executados, com zero ignorados nessas execuções. A matriz
HTTP Rust e a integração Fastify também passaram, incluindo 54 tentativas com
recursos alheios ou IDs pai/filho incompatíveis e comparações de estado
antes/depois. Os containers
foram removidos ao final.

A execução detectou uma exportação incorreta no pacote de banco: o namespace
Prisma vinha do caminho padrão, embora o schema configure geração em outro
diretório. A exportação passou a usar o mesmo cliente gerado dos demais exports;
a integração Fastify verificou a importação e o acesso real ao banco.

## Limites e pendências de liberação

- A validação ocorreu no checkout local, incluindo o trabalho de portfólio
  ainda não publicado. Os commits de segurança mantêm esse trabalho separado.
- Permissões/RLS e Data API do ambiente externo permanecem pendentes por decisão
  do usuário; o projeto Supabase disponível estava inativo na consulta.
- Troca de sessão com duas contas reais e múltiplas abas não foi validada no
  navegador; os testes automatizados exercitam a corrida no cache e nas operações.
- O portfólio BRL/snapshots `portfolio_valuations` é trabalho local separado,
  ainda fora dos commits de segurança. Sua integração HTTP e banco precisa ser
  acrescentada à matriz antes de sua liberação.
- Os testes que exigem provedores reais continuam separados. A aprovação deste
  comando não comprova configuração de produção nem substitui a revisão de RLS.
