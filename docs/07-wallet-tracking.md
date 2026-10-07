# Tracking de carteiras em Rust

`GET /v1/me/wallets/{id}/assets` consulta saldos e cotações com cache.
`POST /v1/me/wallets/{id}/sync` força atualização, respeitando o cooldown.
Ambas exigem a sessão autenticada e verificam o proprietário antes de acessar
provedores. Ethereum, Polygon, Arbitrum e Base usam Alchemy; outras redes
retornam `CHAIN_NOT_SUPPORTED`.

Valores decimais são strings. `complete` indica cobertura dos saldos;
`valuationComplete` indica que todos os saldos têm cotação. Um total parcial
nunca deve ser apresentado como avaliação completa. Identidades são `native`
ou endereço de contrato em minúsculas, nunca apenas o símbolo. Respostas
paginadas e tokens sem metadados preservam registros anteriores. Sincronizar
carteiras não cria aportes.

Aplicar a migration `20261002000000_wallet_tracking` pelo fluxo de migrations
do projeto antes de habilitar as rotas. Ela preserva registros legados e
normaliza endereços EVM; colisões existentes precisam ser resolvidas antes.
Credenciais ficam somente em `apps/api-rust/.env`.

## Validação

Em 2026-10-03, os testes reais verificaram metadados USDC no Alchemy,
persistência e cache de uma carteira pública vazia, isolamento entre usuários
e ausência de aportes criados. CoinGecko retornou cotações ETH e USDC.
O teste da carteira vazia não comprova uma carteira real com saldo positivo.
Os testes isolados de PostgreSQL/Redis verificaram precisão, identidade,
preservação em respostas parciais, TTL, cooldown e propriedade do lock.

Em 2026-10-07, a suíte Rust padrão passou novamente. Os testes externos são
opt-in e exigem `TEST_DATABASE_URL` e `TEST_REDIS_URL` descartáveis:

```sh
cargo test --workspace --locked
cargo test -p astra-infrastructure --test tracking_live --test tracking_cache --test tracking_repository --locked -- --ignored --nocapture
```

O teste real aceita apenas o banco local `astra_test` na porta 5432.
