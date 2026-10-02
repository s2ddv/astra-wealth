# Watchlists no Rust

A migração usa as mesmas tabelas Prisma e preserva o contrato do Fastify:

- `GET /v1/me/watchlists`: array de listas com `id`, `name`, `userId`, `items`,
  `createdAt` e `updatedAt`.
- `POST /v1/me/watchlists`: `{ "name": "Favoritos" }`, resposta 201.
- `DELETE /v1/me/watchlists/{id}`: resposta 204.
- `POST /v1/me/watchlists/{id}/items`: `{ "coinId": "bitcoin" }`, resposta 201
  com `id`, `coinId` e `addedAt`.
- `DELETE /v1/me/watchlists/{id}/items/{coinId}`: resposta 204.

Os nomes são aparados e limitados a 64 unidades UTF-16, e `coinId` a 128,
compatível com o Zod existente. A validação retorna `error.formErrors` e
`error.fieldErrors`. Duplicidades retornam 409 com as mensagens existentes;
objetos de outros usuários ou inexistentes retornam 404. Todas as operações
usam o usuário local autenticado, e a inclusão de itens trava a lista durante a
transação para impedir corrida com exclusão. Excluir uma lista remove seus itens.

Não há alteração de schema, migração de favoritos do localStorage nem troca
global da URL do frontend. As duas APIs acessam os mesmos registros. A troca do
cliente deve ser feita por domínio após a integração da interface.

Validação: testes de contrato HTTP com respostas legadas, erros Zod, autenticação
e limites Unicode; integração PostgreSQL com dois usuários, duplicidades,
releitura persistida, remoção e cascata.
