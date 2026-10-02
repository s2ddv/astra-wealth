# API persistente de contas, posições e aportes

A API Rust usa as tabelas oficiais `FinancialAccount`, `Holding` e `Contribution`.
A migration `20260930180000_add_financial_accounts_and_contributions` deve estar
aplicada no PostgreSQL direto, porta 5432. O frontend está sendo trabalhado em
outra tarefa; a disponibilidade destas rotas não significa que seus mocks já
foram substituídos.

## Contrato

Todas as rotas exigem a autenticação existente. O proprietário é o usuário local
resolvido pelo backend; `userId`, `origin` e `externalId` não são aceitos em corpos
de criação. Valores decimais são strings, inclusive nas respostas. Datas usam
RFC3339 e são normalizadas para UTC ao persistir.

| Método | Rota | Comportamento |
| --- | --- | --- |
| GET | `/v1/accounts` | Lista contas do usuário |
| POST | `/v1/accounts` | Cria conta; retorna 201 |
| GET | `/v1/accounts/{id}` | Conta com `holdings` e `contributions` |
| DELETE | `/v1/accounts/{id}` | Exclui conta e registros dependentes; 204 |
| POST | `/v1/accounts/{id}/holdings` | Registra posição manual; 201 |
| PUT | `/v1/accounts/{id}/holdings/{record}` | Substitui campos de posição manual |
| DELETE | `/v1/accounts/{id}/holdings/{record}` | Exclui posição manual; 204 |
| POST | `/v1/accounts/{id}/contributions` | Registra aporte ou retirada; 201 |
| DELETE | `/v1/accounts/{id}/contributions/{record}` | Exclui registro manual; 204 |

Criação de conta:

```json
{"name":"Reserva","kind":"MANUAL","baseCurrency":"BRL","institutionName":"Banco"}
```

Posição:

```json
{"assetClass":"FIXED_INCOME","name":"CDB","quantity":"1","currentValue":"1250.1234567891","unitPrice":null,"symbol":null,"dueDate":null}
```

Aporte (a conta vem da URL):

```json
{"kind":"DEPOSIT","amount":"100","currency":"USD","fxRateToBrl":"5.25","occurredAt":"2026-01-01T12:00:00Z","note":"Aporte registrado pelo usuário"}
```

Quantidades e valores aceitam 18 dígitos inteiros e 10 decimais; câmbio histórico
aceita 10 inteiros e 8 decimais. Aportes devem ser positivos, com tipo `DEPOSIT`
ou `WITHDRAWAL`, e não podem ter data futura. USD exige câmbio histórico positivo.
Posições aceitam zero. Aportes não alteram o valor das posições.

Erros têm `{ "code": "...", "error": "..." }`: 400 `VALIDATION_ERROR`, 404
`NOT_FOUND`, 409 `CONFLICT` e 503 `STORAGE_UNAVAILABLE`. Contas e registros de
outros usuários respondem 404. Nenhum detalhe de SQL ou credencial é retornado.

## Relação entre contas e carteiras

- Uma conta `WALLET` exige `walletId` de uma carteira do mesmo usuário.
- Uma carteira só pode estar vinculada a uma conta, pela unicidade do schema.
- `MANUAL`, `BANK` e `BROKERAGE` não aceitam `walletId`; conexões de exchanges e
  Open Finance ainda não são criadas por este endpoint.
- Posições on-chain ficam exclusivamente em `wallet_assets`; contas `WALLET`
  rejeitam a criação de holdings manuais para evitar dupla contagem.
- Aportes em contas cripto são registros explícitos do usuário. Sincronizar
  saldos ou detectar transferências nunca cria, altera ou remove contribuições.
- Excluir uma conta remove seus holdings/aportes, mas preserva a carteira.
- A API impede excluir uma carteira vinculada usando um lock transacional; remova a
  conta antes. A API de carteira retorna 409 `WALLET_LINKED_TO_ACCOUNT`.

## Verificação

Testes de domínio e HTTP cobrem limites decimais, datas, câmbio, autenticação e
campos forjados. O teste `accounts_repository` usa PostgreSQL descartável com o
histórico Prisma e verifica isolamento de dois usuários, persistência após
recriar o repositório, precisão, exclusão em cascata e vínculo de carteira.

```sh
cd apps/api-rust
cargo check --workspace --locked
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
TEST_DATABASE_URL=postgresql://astra_test:astra_test@127.0.0.1:5432/astra_test cargo test -p astra-infrastructure --test accounts_repository --locked -- --ignored
```

A avaliação consolidada e os snapshots são uma etapa posterior. Esta API retorna
os registros persistidos; não inventa conversão cambial atual nem rentabilidade.
