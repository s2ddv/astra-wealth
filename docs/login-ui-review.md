# Login — Astra Wealth

## Resumo

Tela web em Next.js/React, com CSS Modules. Avaliação: **Boa**. O objetivo é dar acesso ao patrimônio com clareza; a composição centralizada da referência Coinbase usa a marca, o fundo e o roxo já existentes no Astra. Aplicam-se os fundamentos da skill apple-design, sem impor convenções nativas de macOS à web.

A revisão inicial identificou campos sem rótulos persistentes, falta de retorno para corrigir o e-mail, erros técnicos em inglês e um tema claro isolado. O callback estava em `routes.ts`, que não registra uma rota, e `/login` não existia.

## Sistema visual e decisões

| Papel | Escuro implementado | Contraste no fundo #131316 | Claro proposto, não implementado |
| --- | --- | --- | --- |
| Fundo | #131316 | — | #ffffff |
| Texto | #e6e1e8 | 14,39:1 | #211f26 (16,30:1) |
| Apoio | #aaa4b2 | 7,65:1 | #625c69 (6,45:1) |
| Ação e foco | #d1bcff | 10,90:1 | #6738c2 (7,19:1) |
| Borda de campo | #77717f | 3,93:1 | #77717f (4,73:1) |
| Erro | #ffb4ab | 10,92:1 | #a12838 (7,30:1) |

O texto do botão ativo, #3b1f73 sobre #d1bcff, tem contraste 7,58:1. Cálculos pela luminância relativa sRGB. O tema escuro acompanha o dashboard e a referência solicitada; suporte claro fica pendente de uma decisão conjunta do produto.

Família já adotada pelo projeto: Coinbase Sans com fallback de sistema. Título 30–32 px, texto e controles 16 px, rótulos 14 px, em rem. Campos e ações principais têm altura mínima de 56 px. Formulário de até 400 px; foco de 2 px. Não há blur ou animação de entrada; transições de cor de 150 ms respeitam movimento reduzido.

```text
Regular: marca no canto → coluna central de 400 px
                         título e contexto
                         e-mail → continuar → senha
                         ou / Google
                         criar conta
Compacto: marca → mesma coluna fluida com margens de 16–24 px
```

Julgamento de craft: manter o login deliberadamente discreto, ligado ao produto pelo símbolo Astra, a paleta existente e a frase sobre patrimônio. Removido o rodapé que repetia a marca. Não foram acrescentados links legais sem páginas de destino nem métodos de autenticação sem integração.

## Fundamentos aplicados

- `text-fields.md › Best practices`: “include a separate label describing the field”. Rótulos persistentes, tipos nativos e autocomplete.
- `accessibility.md › Vision`: “Support larger text sizes.” Layout fluido e verificação de texto ampliado em 200%.
- `buttons.md › Best practices`: “Always include a press state for a custom button.” Ações com hover, active e foco visível.
- `managing-accounts.md › Best practices`: “Refer only to authentication methods that are available in the current context.” Mantidos e-mail/senha e Google, sem botões de Apple ou passkeys.

Referências lidas em `/home/samuel-barbosa/.agents/skills/apple-design-skill/references/`: accessibility, layout, typography, color, designing-for-macos, buttons, entering-data, text-fields, managing-accounts, dark-mode e cross-platform.

## Integração e validação

- Componentes separados: AuthShell, SignInForm, CSS Module e funções de navegação/erros.
- `/signin` e `/login` apresentam a tela; callback registrado em `/auth/callback`.
- Redirecionamentos aceitam apenas destinos internos do dashboard. Falhas no callback preservam o destino e retornam uma mensagem recuperável.
- TypeScript e build de produção Webpack aprovados. A execução restrita falhou ao interpretar `tsc --showConfig`; o build fora do sandbox concluiu.
- Chromium: larguras 1440, 768, 390 e 320 px; sem transbordamento horizontal. Texto a 200% em 320 px; opção de revelar senha passa para baixo do campo nessa largura.
- Verificados teclado, e-mail inválido, foco na senha, mostrar/ocultar, alteração de e-mail, limpeza da senha ao voltar, alias `/login` e callback sem código. Sem erros JavaScript no roteiro.
- Casos de redirecionamento externo, esquema JavaScript, barras invertidas e saída do dashboard rejeitados; query e fragmento internos preservados.

## Limites

Login real, emissão de cookies e conclusão do OAuth com Google não foram validados com uma conta. O callback segue a [documentação de Google OAuth do Supabase](https://supabase.com/docs/guides/auth/social-login/auth-google); configuração do provedor e URLs autorizadas continuam necessárias. A tela de cadastro mantém seu desenho anterior. Não foi realizado teste manual com leitor de tela.
