# PTAX e consolidação patrimonial Rust

Implementação iniciada em 08/10/2026; validação final em 10/10/2026. A ativação do overview real permanece desligada por padrão. Fastify continua atendendo seus domínios; seus snapshots USD não foram migrados nem reescritos.

## Contrato de câmbio

Rotas autenticadas no Rust:

- `GET /v1/me/fx/usd-brl`: data atual de Brasília.
- `GET /v1/me/fx/usd-brl/YYYY-MM-DD`: data histórica explícita, entre 2000-01-01 e hoje. Datas inválidas/futuras retornam 400.

A taxa é **BRL por USD, venda, fechamento PTAX**. É uma referência de avaliação, não a taxa de execução de uma operação nem o custo final com spread/tributos. O adaptador usa `CotacaoMoedaPeriodo` com USD, filtro `tipoBoletim eq 'Fechamento'`, ordem decrescente e uma linha. OData do Olinda exige espaços como `%20`, não `+`; isso foi confirmado por consulta real e coberto pelo teste HTTP.

Fontes oficiais: [documentação PTAX](https://olinda.bcb.gov.br/olinda/servico/PTAX/versao/v1/documentacao) e [contrato de boletins do BCB](https://www.bcb.gov.br/conteudo/dadosabertos/BCBDepin/gnastportal-dados-abertostaxas-de-cambio---todos-os-boletins-diarios.pdf).

Exemplo de estrutura (valores ilustrativos):

```json
{
  "pair": "USD/BRL",
  "source": "BCB_PTAX_SELL_CLOSE",
  "requestedDate": "2026-04-05",
  "quote": {
    "rate": "5.1234",
    "effectiveDate": "2026-04-02",
    "publishedAt": "2026-04-02 13:05:12.123"
  },
  "status": "previous_close",
  "fetchedAt": "2026-04-05T18:00:00+00:00"
}
```

`publishedAt` é o timestamp local publicado pelo BCB, no fuso `America/Sao_Paulo`, sem offset fornecido pela origem. `fetchedAt` é RFC3339. A resposta sempre separa a data solicitada da data efetiva. Preços e valores são strings decimais; o parser JSON preserva a precisão numérica original.

| Situação | Comportamento |
| --- | --- |
| Fechamento da própria data disponível | `available` |
| Fim de semana, feriado ou fechamento ainda não publicado | Último fechamento **até** a data solicitada, `previous_close` |
| Nenhum fechamento nos 14 dias anteriores, inclusive data solicitada | `missing`, `quote: null`; não inventa taxa |
| Falha HTTP, timeout, JSON inválido, taxa não positiva ou data fora da janela | Cache elegível com `stale`; sem cache elegível, `unavailable` e `quote: null` |
| Cotação posterior à data histórica | Rejeitada; nunca usa taxa de hoje para preencher o passado |

Não há calendário inferido de feriados nem horário fixo presumido de publicação. A disponibilidade do boletim determina a seleção; o recuo é limitado a 14 dias corridos. Uma paralisação maior resulta em ausência de cotação.

Cache Redis público por par, convenção e **data solicitada** (`fx:ptax:sell:v1:YYYY-MM-DD`). Datas dos últimos sete dias têm atualização a cada 300 segundos; datas anteriores, 86400 segundos. Respostas vazias são reconsultadas após 60 segundos. Retenção máxima para fallback: um dia nas datas recentes e 90 dias nas antigas, medidos desde `fetchedAt`, que não é renovado por leitura/falha. Redis indisponível não impede consulta direta. Respostas de erro do provedor não são gravadas como taxas. Timeout HTTP: 10 segundos; corpo máximo: 64 KiB; redirects desativados.

## Consolidação atual

`GET /v1/me/portfolio` lê uma visão PostgreSQL `REPEATABLE READ`, filtrada pelo ID local do usuário autenticado. Não aceita `userId` escolhido pelo cliente. Todos os cálculos usam `BigDecimal`, sem `f64`; o arredondamento a centavos acontece apenas na apresentação.

- Holdings não cripto são avaliados pelo `currentValue` registrado na moeda da conta: BRL diretamente, USD pela PTAX atual. Isso não constitui preço de mercado verificado para ações/renda fixa.
- Todas as carteiras do usuário entram uma vez, vinculadas ou não a contas. Holdings de contas `WALLET` não são somados novamente.
- Quantidades de carteira vêm de `wallet_assets`. A consolidação não aciona sincronização on-chain nem cria aportes.
- Cripto de carteira usa CoinGecko **diretamente em BRL**, por rede/contrato ou ID nativo, com cache próprio de 60 segundos e decimais exatos. O tracking legado em USD continua intacto.
- Carteiras sem sincronização, sincronização incompleta ou saldos com mais de 24 horas são explicitamente incompletas. Cada posição informa a data do saldo e a data de consulta do preço.
- Cripto manual sem identidade inequívoca de rede/contrato e redes ainda não suportadas ficam sem avaliação; nunca se resolve um token apenas pelo símbolo.
- Não há integração nova de exchanges/Open Finance nem consulta de preços históricos de cripto nesta etapa.

Campos principais:

| Campo | Significado |
| --- | --- |
| `knownValueBrl` | Soma exata dos valores conhecidos; `null` se todas as posições estiverem sem avaliação; `"0"` para portfólio vazio |
| `complete` | Todas as posições avaliadas e sem indicadores de saldo/câmbio incompletos ou desatualizados |
| `netContributionsBrl` / `contributionsComplete` | Aportes explícitos menos retiradas; total nulo se qualquer contribuição não puder ser convertida |
| `resultBrl` | Patrimônio menos aportes registrados, somente quando ambos os lados estão completos |
| `positions` | Identidades, valores, moeda/quantidade original, preço BRL, fonte, data do saldo, PTAX usada e `issue` |
| `contributions` | Valor convertido, taxa, fonte `RECORDED`/`BRL`/`BCB_PTAX_SELL_CLOSE`, data do aporte e metadados PTAX quando aplicáveis |
| `observedAt` / `valuedAt` | Momento da leitura consistente de posições / referência temporal da avaliação |

`issue`: `MISSING_QUOTE`, `STALE_FX`, `NOT_SYNCED`, `PARTIAL_BALANCES`, `STALE_BALANCE` ou `UNSUPPORTED_ASSET`. Cotações anteriores válidas por fim de semana/feriado não são por si só falhas. Um fallback `stale` pode compor o subtotal conhecido, mas impede tratá-lo como total completo ou apresentar resultado acumulado.

Falha de armazenamento ou prazo global de 50 segundos excedido retorna 503. Falha de um preço retorna a posição sem cotação, preservando as demais. Rotas privadas usam `Cache-Control: private, no-store`. Não há cache Redis de patrimônio pessoal.

## Aportes históricos

`Contribution.fxRateToBrl` existente sempre prevalece, identificado como `RECORDED`. Nenhuma rotina desta entrega altera esse campo. A origem histórica de taxas informadas anteriormente não é inventada: a data do aporte é conhecida, mas não se presume que a taxa manual veio do BCB.

BRL usa taxa 1. Para registros legados USD sem taxa, a consolidação consulta a PTAX correspondente à data do aporte em `America/Sao_Paulo`, incluindo regras históricas de horário de verão no PostgreSQL. Não aceita fallback `stale` para completar esse valor histórico. A conversão derivada aparece na resposta e no snapshot salvo; não modifica o aporte original. A API existente de criação de aportes continua exigindo taxa explícita para USD.

Transferências e sincronizações não são aportes. O resultado depende da completude do histórico **registrado**: não comprova que todos os aportes reais foram cadastrados e não é uma taxa de retorno ajustada por tempo.

## Snapshots

- `POST /v1/me/portfolio/snapshots`: captura a avaliação atual no servidor e retorna 201 com `{ id, snapshot }`. Não recebe valores calculados pelo cliente.
- `GET /v1/me/portfolio/snapshots`: últimos 100 snapshots do usuário, mais recentes primeiro.

A tabela nova `portfolio_valuations` possui relação com `User` e JSONB com versão, strings decimais, posições, contribuições, cotações e completude. A migration `20261008090000_portfolio_valuations` é aditiva. A API não oferece atualização de snapshot; mudanças em holdings ou cotações não recalculam os registros salvos. Exclusão de usuário mantém o cascade previsto no schema.

Não se reconstroem saldos históricos a partir de posições atuais. Não há backfill dos snapshots antigos em USD, scheduler, paginação além dos últimos 100 ou histórico anterior às capturas efetivamente salvas. Snapshots parciais preservam a sinalização de parcialidade.

## Overview e ativação

O Next encaminha apenas rotas/métodos permitidos pelos domínios `portfolio` e `fx`, usando o Bearer da sessão e `PORTFOLIO_API_URL` (padrão local 3334). A tela usa o contexto privado da sessão e não compartilha requisições entre usuários.

`PORTFOLIO_REAL_OVERVIEW_ENABLED=false` é o padrão. Com `true`, o overview usa exclusivamente o contrato Rust para total/subtotal, aportes, resultado e fontes; tem carregamento, vazio, erro/retry, atualização manual e captura de snapshot. Não converte valores para USD com a taxa fixa do overview legado. A formatação usa `BigInt`, preservando grandes valores e arredondando somente para exibição.

**Condição para ativar:** aplicar a migration no ambiente alvo e validar com contas reais e carteira sincronizada com saldo não zero, conferindo quantidade, CoinGecko BRL, PTAX/data, aportes e isolamento de duas sessões. Os testes com contas/carteiras persistidas em banco descartável comprovam o contrato e a precisão, mas usam dados controlados e preços simulados; não substituem essa validação operacional. Nenhuma flag de produção foi habilitada nesta entrega.

## Verificação

Consulte `docs/04-status-atual.md` para os resultados finais. Logs locais ignorados pelo Git: `apps/api-rust/target/ptax-validation/`.

A sequência do AGENTS.md é `npm ci`, `npm run lint`, `npm run typecheck`, `npm test`, `npm run build`. Como o workspace é pnpm, após `npm ci` é necessário restaurar os links com `CI=true pnpm install --frozen-lockfile`; isso não altera os lockfiles. Neste ambiente, executar os scripts pnpm fora do sandbox evitou o travamento observado dentro dele.

Os testes de PostgreSQL/Redis usam somente `astra_ptax_test` em `127.0.0.1:55432` e Redis descartável em `127.0.0.1:56379/0`. As migrations oficiais completas foram aplicadas ao banco vazio. Não apontar os testes para o banco do projeto.

```sh
# No workspace Rust, após preparar os serviços descartáveis:
cargo test --workspace --locked
cargo test -p astra-infrastructure --locked --test portfolio_repository -- --ignored
cargo test -p astra-infrastructure --locked --lib fx::tests::live_ -- --ignored --nocapture
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
```

Os testes live PTAX usam a API pública, sem dados de usuários. Os testes live Alchemy e CoinGecko existentes permanecem opt-in; a execução da suíte padrão não prova seu sucesso.
