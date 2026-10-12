# Operações financeiras manuais — P1

## Escopo e fonte de verdade

O domínio pertence à API Rust. `FinancialOperation` é um livro de registros imutáveis ligado a `FinancialAccount` e, quando aplicável, a `Holding`. O Prisma continua sendo a fonte de verdade do banco. A migration `20261010100000_manual_operations` apenas cria tabela, índices, vínculos e proteção RLS; não modifica os registros anteriores.

Na página de uma conta é possível cadastrar um ativo, registrar operações, consultar o histórico e corrigir valores, quantidades, taxas, data/hora e observação. O cadastro de ativo reutiliza `Holding` e começa com quantidade e avaliação patrimonial zero, sem preço. Uma operação não inventa cotação para preencher essa avaliação.

**Livro de operações e avaliação patrimonial são distintos.** As quantidades e custos calculados aparecem na seção de operações. Não há movimentação automática de caixa nem sobrescrita de `Holding.quantity`, `unitPrice` ou `currentValue`. Compras, vendas e saldos iniciais posteriores à avaliação tornam essa avaliação pendente (`valuationPending`, `MANUAL_VALUATION_REQUIRED`); a consolidação Rust passa a ser parcial e não usa o valor desatualizado como valor atual. Uma atualização explícita da avaliação pela API de holdings existente encerra a pendência. A interface completa de conciliação/avaliação e um livro de caixa são dependências futuras; não estão implicitamente entregues neste P1.

Aportes/retiradas continuam no modelo e formulário `Contribution`; nenhuma compra, venda, taxa, provento, transferência ou saldo inicial gera uma contribuição. O frontend usa a ponte privada já autenticada e seu isolamento de cache por sessão.

## Tipos

| `kind` | Significado | Campos específicos |
| --- | --- | --- |
| `BUY` | Compra de ativo | Ativo, mercado e quantidade; bruto e taxas podem ser desconhecidos |
| `SELL` | Venda de ativo | Ativo, mercado e quantidade; bruto e taxas podem ser desconhecidos |
| `INCOME` | Provento recebido | Valor bruto positivo; ativo opcional; taxas conhecidas ou desconhecidas |
| `FEE` | Despesa avulsa | Valor positivo; sem ativo e sem taxa embutida |
| `TRANSFER` | Transferência monetária interna | Valor positivo e conta de destino do mesmo usuário e moeda |
| `OPENING` | Saldo inicial do histórico de um ativo | Quantidade positiva, mercado e custo total conhecido ou nulo |

`OPENING` deve ser o primeiro evento vigente do ativo em ordem cronológica. Serve para posições anteriores ao período registrado. O valor de mercado de uma posição existente **não** é presumido como seu custo. Sem saldo inicial, o histórico parte de quantidade zero; uma venda acima da quantidade registrada é rejeitada. Não há posições vendidas a descoberto neste fluxo.

Transferências são um único registro, visível como saída na origem e entrada no destino. Não alteram quantidade/custo de ativos, aportes, posições patrimoniais ou patrimônio consolidado. Transferência de custódia de ativos, conversão entre moedas e taxas de câmbio estão fora deste contrato. Uma taxa bancária de transferência pode ser registrada separadamente como `FEE`, sem modificar o principal da transferência.

## Regra de custo e precisão

A regra se aplica somente a holdings **manuais**, com classe `STOCK`, `REAL_ESTATE_FUND` ou `ETF`, mercado explicitamente `BR` e moeda `BRL`. O usuário identifica o mercado; não se infere país pelo ticker. `INTERNATIONAL` e `UNKNOWN` não recebem a regra. Renda fixa, cripto, fundos, previdência e demais classes têm quantidade registrada, mas custo médio e resultado realizado indisponíveis.

O custo é acompanhado **por holding dentro de cada conta**. Não há unificação por ticker, compensação de prejuízos entre contas nem tratamento de eventos societários. Esses cálculos servem ao acompanhamento patrimonial, **não são apuração fiscal**.

Sejam `Q` a quantidade anterior e `C` o custo acumulado:

- Compra de `q`, valor bruto `V`, taxa `T`: `Q' = Q + q`; `C' = C + V + T`.
- Custo médio anterior: `CM = C / Q`.
- Venda de `q`: custo atribuído `CV = C × q / Q`; `Q' = Q − q`; `C' = C − CV`.
- Recebimento líquido: `L = V − T`; resultado realizado: `L − CV`.
- Venda da posição inteira: usa **todo o custo restante**, evitando resíduo de arredondamento.
- Proventos: líquido `V − T`, sem alterar custo médio ou quantidade. Não são apresentados como resultado de venda.
- Taxas avulsas: despesas independentes, sem alocação automática ao custo de um ativo. A taxa já informada em uma compra/venda/provento não deve ser registrada novamente como despesa avulsa.

Exemplo: compras de 10 unidades por 100 + 2 de taxa e 10 por 200 + 4 resultam em 20 unidades e custo 306 (médio 15,30). Venda de 5 por 100 com taxa 1 atribui custo 76,50, recebe líquido 99 e realiza 22,50. Restam 15 unidades com custo 229,50.

Entradas e saídas monetárias usam strings decimais, com até 18 dígitos inteiros e 10 casas fracionárias de entrada; persistência `DECIMAL(28,10)` e cálculos `BigDecimal`. Não há `float`/`Number` nos cálculos ou na apresentação dos valores deste fluxo. Divisões não terminantes usam 20 casas decimais, arredondamento para o par mais próximo (`HalfEven`); o custo médio mostrado não é realimentado no cálculo. Dados originais permanecem com a precisão recebida, normalizados numericamente pelo banco. Soma/subtração/multiplicação são decimais; o arredondamento explicitado ocorre na divisão.

Campo nulo significa **desconhecido**, nunca zero. Taxa `"0"` confirma ausência de taxa; taxa nula impede cálculo do líquido. Uma compra com bruto/taxa desconhecido ou saldo inicial sem custo torna o custo acumulado desconhecido até correção ou encerramento integral da posição. Ao zerar a posição, um novo ciclo elegível pode voltar a acumular custo conhecido. Nenhum preço ou câmbio é consultado/inventado. Valores em USD podem ser registrados em conta USD, mas não recebem resultado realizado desta regra.

## Contrato HTTP

- `GET /v1/accounts/{accountId}/operations` → `200`, `{ operations, results, positions }`.
- `POST /v1/accounts/{accountId}/operations` → `201`, operação persistida (também em repetição idempotente idêntica).
- Corpo máximo: 16 KiB. Até 2.000 registros por usuário, incluindo originais e correções. Sem paginação nesta etapa pequena; aumentar o limite exige paginação e recálculo incremental revisados.
- Datas RFC3339 não futuras, armazenadas em UTC com milissegundos. Ordenação: data da operação, data de criação e ID para desempate.
- Todos os valores decimais chegam como strings. Exponentes, valores negativos, quantidade zero, excesso de casas, campos desconhecidos e propriedade forjada são rejeitados.
- `400`: entrada inválida, regra inconsistente ou histórico com venda excedente; `401`: autenticação ausente/inválida; `404`: conta/ativo/correção inexistente ou de outro proprietário; `409`: conflito de chave, correção já substituída ou exclusão de registro referenciado; `503`: indisponibilidade de armazenamento, sem detalhes internos no corpo.

Exemplo de compra, substituindo os IDs pelos registros da própria conta:

```json
{
  "requestId": "1181b73d-7bb3-423a-ad1d-c7cf2f23b1db",
  "kind": "BUY",
  "holdingId": "holding-id",
  "market": "BR",
  "quantity": "10",
  "amount": "100",
  "fees": "2",
  "currency": "BRL",
  "occurredAt": "2026-10-01T13:00:00Z",
  "note": "Compra manual",
  "destinationAccountId": null,
  "replacesId": null
}
```

`operations` contém cada registro original e suas correções. `results` relaciona `id`, `active`, `netAmount`, `assignedCost`, `realizedResult` e `unavailableReason`. Resultados não aplicáveis/substituídos são nulos. Motivos: `UNSUPPORTED_COST_RULE` e `MISSING_COST_AMOUNT_OR_FEES`. `positions` contém `holdingId`, quantidade do histórico, custo acumulado e custo médio; não representa cotação de mercado.

## Correções, concorrência e isolamento

Para corrigir uma operação, envie um novo POST com novo `requestId` e `replacesId` igual ao ID vigente. Tipo, ativo, conta de origem/destino, mercado, moeda e classe são preservados. Uma correção pode, por sua vez, ser substituída por outra. Não há DELETE, alteração destrutiva nem cancelamento de operações neste P1. Correção de classificação/contas e cancelamento exigem um contrato posterior, em vez de apagar histórico.

O replay ignora registros substituídos para calcular posições/resultados e mantém todos para consulta. Correções históricas recalculam vendas posteriores. Se uma alteração causar quantidade negativa em qualquer ponto, a transação inteira é revertida. Classe/mercado/moeda precisam permanecer consistentes no histórico de um mesmo holding; reutilizar um holding para outro ativo não é suportado.

`requestId` é único por conta. Repetir a mesma requisição retorna o registro persistido, sem duplicação; reutilizar a chave com dados diferentes retorna conflito. A interface conserva a chave ao repetir um envio malsucedido sem mudar o conteúdo. Um novo lançamento intencional usa outra chave; após perder o estado do navegador, consulte o histórico antes de reenviar.

O ID local do usuário autenticado é a única fonte do proprietário. A API autoriza origem, destino, holding manual e registro substituído. Escritas de um mesmo proprietário são serializadas com lock transacional; as duas contas de transferência são bloqueadas em ordem estável. Vendas concorrentes não podem consumir a mesma quantidade duas vezes. Vínculos `RESTRICT` impedem exclusão de contas/holdings que apagaria o histórico ou uma ponta de transferência.

A tabela nova tem RLS habilitada sem políticas para clientes diretos. Mesmo um papel com grants de tabela não pode ler/alterar operações pelo acesso de navegador. A conexão privada do Rust deve usar o proprietário da tabela ou papel com `BYPASSRLS`; não use a conexão do navegador. A migration não altera políticas das tabelas preexistentes.

## Validação e operação

Sequência da raiz: `npm ci`, restauração dos links do workspace com `pnpm install --frozen-lockfile` (gerenciador declarado pelo repositório), `npm run lint`, `npm run typecheck`, `npm test`, `npm run build`.

Rust: `cargo check/build/test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings` e `cargo fmt --all --check` dentro de `apps/api-rust`.

`bash scripts/test-manual-operations.sh` cria PostgreSQL e Redis descartáveis, aplica todas as migrations, compara o banco com o schema Prisma e executa os testes opt-in de operações, contas e consolidação. Não aceita conexão do projeto e remove seus containers ao terminar. Requer Podman ou Docker e imagens locais `postgres:16-alpine` / `redis:7.4-alpine`.

Os testes cobrem precisão subcentavo, custo ponderado, taxas, saída parcial/total, divisões recorrentes, desconhecidos, classes não elegíveis, correções, validação HTTP, dois proprietários, referências cruzadas, RLS, repetição idempotente, concorrência, rollback e invariância patrimonial/aportes das transferências.

A migration deve ser aplicada pelo fluxo habitual no ambiente de destino antes de executar o novo backend. Não há migration no banco do projeto, deploy manual, alteração de workflows ou ativação da overview de produção nesta entrega. CSV, Open Finance, cobrança, tributação, eventos societários, transferências de ativos e reformulação do dashboard permanecem fora de escopo.

### Evidências desta entrega (11/10/2026)

- `npm ci`, `npm run lint`, `npm run typecheck`, `npm test` e `npm run build` aprovados nessa ordem; 24 testes JS/TS aprovados. O lint atual do repositório executa TypeScript (`tsc --noEmit`). Os links pnpm removidos pelo npm foram restaurados com o lockfile congelado antes do lint.
- Rust: check, build, suite padrão (87 aprovados, 15 opt-in ignorados), Clippy com `-D warnings` e rustfmt aprovados. Três testes opt-in foram executados separadamente e aprovados: operações, contas e consolidação/snapshots.
- Migrations completas aplicadas em PostgreSQL descartável; comparação com Prisma sem divergências. Teste de RLS confirmou ausência de leitura/exclusão para papel sem políticas, mesmo com grants.
- Smoke visual com componente real e dados fictícios no navegador: renderização de custos/resultados, abertura do formulário, seleção de transferência, preenchimento, feedback de erro e correção preenchida. Não equivale a sessão autenticada real, teste móvel ou E2E entre navegador e banco.
- Limitações ambientais resolvidas: pnpm e sockets de testes bloqueados no sandbox foram executados com permissão local. Rustfmt/Clippy foram baixados dos repositórios Fedora e extraídos em `/tmp`, sem instalação no sistema. O teste de consolidação foi adaptado para aceitar o nome do banco descartável em porta dinâmica, mantendo restrição explícita a loopback e nomes de teste.
- Avisos existentes: detecção de módulo TypeScript pelo Node e depreciação da configuração Prisma em `package.json`; não impediram as verificações.
- `docs/08-security-isolation.md` contém alteração paralela, preservada e excluída desta entrega.

### Arquivos desta entrega

Banco:
- `packages/database/prisma/schema.prisma`
- `packages/database/prisma/migrations/20261010100000_manual_operations/migration.sql`

Rust (caminhos relativos a `apps/api-rust/`):
- `crates/domain/src/accounts.rs`
- `crates/application/src/lib.rs`
- `crates/application/src/operations.rs`
- `crates/application/src/operations/tests.rs`
- `crates/application/src/portfolio.rs`
- `crates/infrastructure/src/lib.rs`
- `crates/infrastructure/src/accounts.rs`
- `crates/infrastructure/src/operations.rs`
- `crates/infrastructure/src/portfolio.rs`
- `crates/infrastructure/tests/operations_repository.rs`
- `crates/infrastructure/tests/portfolio_repository.rs`
- `crates/api/src/lib.rs`
- `crates/api/src/main.rs`
- `crates/api/src/accounts.rs`
- `crates/api/src/operations.rs`
- `crates/api/tests/operations_contract.rs`

Frontend e contrato compartilhado:
- `apps/web/src/components/dashboard/OperationsView.tsx`
- `apps/web/src/components/dashboard/AccountsView.tsx`
- `apps/web/src/components/dashboard/AccountsProvider.tsx`
- `apps/web/src/types/operations-api.ts`
- `apps/web/src/lib/private-routes.ts`
- `apps/web/tests/private-api.test.mjs`
- `packages/shared/src/accounts.ts`

Documentação e executor isolado:
- `docs/09-operacoes-manuais.md`
- `scripts/test-manual-operations.sh`

Revisão de segurança final: um subagente dedicado executou a skill `review-security` em modo somente leitura; nenhum achado verificável nas alterações P1. A revisão estática não substitui os testes dinâmicos listados acima.
