# Contas e aportes — Etapa 1

Implementação da base de contas e Open Finance do Astra Wealth, com interface de demonstração.

## Comportamento

- `/dashboard/accounts`: seis contas iniciais, agrupadas por tipo, e quinze aportes/retiradas fictícios.
- `/dashboard/accounts/[accountId]`: posições e histórico de aportes, com origem manual/importada.
- Criação de contas manuais e registro de aportes/retiradas em memória; a navegação mantém os dados, mas recarregar a página os descarta.
- O formulário usa o schema Zod compartilhado. USD exige cotação histórica em BRL. Novos registros têm origem manual.
- Os KPIs e o patrimônio da overview usam as mesmas contas e aportes. Registrar um aporte muda a base de cálculo, sem alterar a avaliação dos ativos.
- Carteiras on-chain não inferem aportes de transferências. Posições cripto continuam separadas de `Holding`.
- Open Finance e conexão on-chain no novo modal aparecem como “Em breve”. Não há integração bancária nem novos endpoints.
- A overview existente não possui donut; nenhum gráfico foi acrescentado.

## Cálculo e moeda

Os resumos e os valores consolidados das contas são em BRL. `Holding.currentValue` permanece na moeda da conta; todas as contas de demonstração usam BRL. Os DTOs mantêm decimais como strings. O cálculo usa inteiros de escala fixa com dez casas decimais; conversões cambiais e divisões são truncadas nessa escala. A apresentação arredonda para centavos. `resultPct` é uma fração: `0.1` equivale a 10%; bases líquidas menores ou iguais a zero retornam `null`.

## Banco

A migration `20260930180000_add_financial_accounts_and_contributions` foi gerada por comparação Prisma do schema anterior com o atual. Adiciona sete enums, quatro tabelas, índices e chaves estrangeiras nas tabelas novas. Nenhuma coluna existente foi alterada. O schema oficial permanece em `packages/database/prisma/schema.prisma`.

## Verificações

- Instalação com pnpm 10.0.0, a versão declarada pelo projeto.
- `pnpm build`, `pnpm lint` e `pnpm typecheck` na raiz.
- `pnpm --filter @astra-wealth/shared test`: depósitos, retiradas, base zero/negativa, USD, precisão decimal, validação Zod e resultado negativo.
- `prisma validate` e `prisma generate`.
- `prisma migrate dev --skip-seed` aplicou todo o histórico em PostgreSQL 16 descartável na porta direta 5432, sem volumes e sem usar o banco configurado do projeto.
- Verificação do SQL aditivo e da preservação dos models existentes, exceto pelas relações inversas solicitadas.

- Teste Playwright em Chromium sobre o build de produção: listagem das seis contas, criação manual, opções “Em breve”, rejeição de valor zero, aporte USD com cotação, retirada, atualização da overview, preservação do estado na navegação, descarte após recarga, fechamento por Escape e ativos/histórico importado. Nenhum erro de console ou de execução foi registrado.

## Arquivos criados

- `packages/database/prisma/migrations/20260930180000_add_financial_accounts_and_contributions/migration.sql`
- `packages/shared/src/accounts.ts`
- `packages/shared/tests/accounts.test.ts`
- `apps/web/src/mocks/accounts.ts`
- `apps/web/src/components/dashboard/AccountsProvider.tsx`
- `apps/web/src/components/dashboard/AccountPresentation.tsx`
- `apps/web/src/components/dashboard/AccountsView.tsx`
- `apps/web/src/app/dashboard/accounts/page.tsx`
- `apps/web/src/app/dashboard/accounts/[accountId]/page.tsx`
- `docs/02-contas-e-aportes.md`

## Arquivos alterados

- `packages/database/prisma/schema.prisma`
- `packages/shared/src/index.ts`
- `packages/shared/package.json`
- `pnpm-lock.yaml`
- `apps/web/src/components/dashboard/DashboardLayoutClient.tsx`
- `apps/web/src/components/dashboard/SideBar.tsx`
- `apps/web/src/components/dashboard/WalletsSummary.tsx`
- `apps/web/src/components/dashboard/PortfolioHeader.tsx`
- `apps/web/public/fonts/material-symbols-outlined.ttf`
- `apps/web/public/fonts/README.md`

A fonte local foi ampliada com os ícones oficiais dos tipos de conta. Nenhuma biblioteca de UI foi adicionada. Zod já existia no monorepo e foi declarado como dependência direta do pacote compartilhado. A exportação `@astra-wealth/shared/accounts` permite consumir os tipos e funções no frontend.
