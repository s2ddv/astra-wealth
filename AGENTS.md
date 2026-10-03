# AGENTS.md

## Antes de finalizar qualquer tarefa
Rode, nesta ordem, e corrija o que falhar:
1. `npm ci`
2. `npm run lint`
3. `npm run typecheck`
4. `npm test`
5. `npm run build`

## Regras
- Nunca faça deploy manualmente nem altere arquivos em `.github/workflows/` sem eu pedir.
- Faça commits pequenos, com mensagens no formato Conventional Commits.
- Se algum passo falhar e você não conseguir resolver, explique o motivo em vez de ignorar.
