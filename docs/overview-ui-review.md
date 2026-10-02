# Revisão de UI/UX — Astra Wealth Overview

## Resumo

Avaliação após os ajustes: **Boa**. A Overview tem como propósito explicar quanto patrimônio existe, como ele se compara aos aportes e onde está distribuído. Sua assinatura é a composição patrimonial por tipo de conta, derivada dos mesmos saldos do resumo, com a identidade roxa da Astra preservada.

Avaliação e implementação em Next.js/React, CSS Modules e Tailwind. Foram aplicados fundamentos de acessibilidade, hierarquia, cor e escrita da skill apple-design; convenções nativas de macOS/iOS não foram impostas à interface web. A análise inicial foi feita pelo código; as capturas registram a implementação final.

## Problemas corrigidos

- **Crítico — layout compacto:** a margem fixa de 260 px somada à navegação fixa deixava apenas 60 px brutos para o conteúdo em uma tela de 320 px. A navegação agora ocupa o topo e permite rolagem horizontal abaixo de 1024 px; o conteúdo usa a largura disponível. `layout.md › Adaptability`: “Design a layout that adapts gracefully and consistently.”
- **Alto — hierarquia invertida:** KPIs precediam título e saldo, enquanto informações correlatas apareciam em cartões separados. Agora a ordem é título, patrimônio, aportes/resultado, distribuição e contas. `layout.md › Visual hierarchy`: “Order content by relative importance.”
- **Alto — interpretação dos números:** a variação de 30 dias era um mock independente do patrimônio; USD no saldo coexistia com BRL nos demais valores. Os controles de período foram removidos até existir histórico correspondente. A Overview começa em BRL e converte todos os blocos quando USD é selecionado. Julgamento de produto: valores e rótulos devem descrever a mesma base.
- **Médio — ações sem efeito útil:** “Atualizar saldos” apenas informava que a página era demonstrativa. Foi removido. Busca, notificações e avatar sem handlers deixaram de aparecer no topo da Overview. `buttons.md › Content`: “Ensure that each button clearly communicates its purpose.”
- **Médio — atenção às contas:** contas com erro, consentimento expirado ou reautorização aparecem primeiro, com descrição textual e ligação para o detalhe. Foi removida da Overview a indicação fixa “Sincronizado há 2h”. `color.md › Inclusive color`: “Avoid relying solely on color”.
- **Médio — leitura e interação:** links sem indicação explícita de foco receberam contorno de 2 px; seletor e ação principal têm pelo menos 44 px de altura. Nomes de contas quebram linha. `accessibility.md › Mobility`: “Offer sufficiently sized controls.”

## Sistema visual

Tokens locais à Overview, usando a paleta já presente no produto. Razões calculadas pela luminância relativa sRGB, sem estimativa visual. As variantes claras abaixo são especificação para uma futura adoção conjunta com o shell: **o tema claro não foi implementado**, para não criar uma página clara isolada dentro do dashboard escuro.

| Papel | Escuro | Contraste em #1c1b21 | Claro proposto | Contraste em branco |
| --- | --- | --- | --- | --- |
| Superfície | #1c1b21 | — | #ffffff | — |
| Conteúdo | #e6e1e8 | 13,27:1 | #211f26 | 16,30:1 |
| Texto secundário | #cac4d0 | 10,03:1 | #625c69 | 6,45:1 |
| Ação | #d1bcff | 10,05:1 | #6738c2 | 7,19:1 |
| Resultado positivo | #4ae176 | 10,03:1 | #176b35 | 6,58:1 |
| Atenção/negativo | #ffb4ab | 10,07:1 | #a12838 | 7,30:1 |

Texto da ação principal #3b1f73 sobre #d1bcff: **7,58:1**. Contorno do seletor #948f99 contra #1c1b21: **5,41:1**. Verde e vermelho têm sinais numéricos ou rótulos como redundância. A barra de distribuição tem valores e percentuais em uma lista equivalente, sem depender de hover ou da cor.

Tipografia: família já existente Coinbase Sans, com fallback de sistema; peso 400–600. Saldo 32–60 px, título 24–36 px, métricas 18 px, títulos de seção 16 px e apoio 13–14 px no tamanho base. Dados usam algarismos tabulares. Unidades rem permitem ampliação.

Layout: duas colunas a partir de 1200 px, coluna única abaixo desse ponto; espaços principais de 24 px e padding de cartões de 20–28 px.

```text
Regular
Navegação | Título                         Adicionar conta
          | Demonstração
          | Patrimônio + aportes/resultado | Suas contas
          | Distribuição por tipo         | atenção primeiro

Compacto
Marca + navegação horizontal
Título / Adicionar conta
Demonstração
Patrimônio / aportes / resultado
Distribuição por tipo
Suas contas / gerenciar
```

Decisão de craft: evitar uma curva de crescimento sem histórico e usar a composição real dos dados de demonstração. Isso relaciona a visualização ao produto que reúne bancos, corretoras e cripto. Foram retirados badges redundantes de retorno e as caixas individuais em cada linha de conta. Não foram adicionadas animações ou transparência à Overview. A transição da navegação responde a `prefers-reduced-motion`; bordas de conteúdo ficam mais fortes com `prefers-contrast: more`.

## O que foi preservado

- Paleta escura, identidade roxa e Material Symbols locais.
- Componentes separados: PortfolioHeader, WalletsSummary e novo PortfolioAllocation.
- Dados e cálculos de aportes existentes; nenhuma mudança no backend.
- Acesso aos detalhes e à gestão de contas.

## Validação

- TypeScript (`pnpm --filter @astra-wealth/web typecheck`): aprovado.
- Build de produção (`next build --webpack`): aprovado, 15 páginas geradas.
- Chromium/Playwright: 1440, 1024, 768, 390 e 320 px, sem transbordamento horizontal do documento. A faixa de navegação tem rolagem própria intencional.
- Texto a 200% em 320 px: sem transbordamento do documento e sem recorte horizontal nos títulos, descrições, métricas e seletor inspecionados.
- Moedas: patrimônio BRL 141.000,00 → USD 27.115,38; aviso do câmbio de 5,20 e conversão das contas verificados.
- Teclado: foco visível na ação principal e Tab até o seletor nativo.
- Navegação: ação de adicionar conta leva à gestão de contas; conta que requer atenção leva ao detalhe correto.
- Nenhum erro JavaScript capturado durante o roteiro.
- `git diff --check`: aprovado.

## Limites e próximos pontos

- **Médio:** a aplicação permanece em tema escuro; os tokens claros são proposta, não suporte de tema já entregue.
- **Médio:** Add Assets e Logout da barra lateral desktop são controles preexistentes sem handlers fornecidos pelo layout. Não foram conectados a novos fluxos nesta alteração.
- Estados vazios foram implementados em contas e distribuição e inspecionados no código; a base demonstrativa atual contém seis contas. Não houve teste manual com leitor de tela, dispositivo físico ou todos os tamanhos máximos de fonte de cada sistema.
- O retorno é simples sobre aportes líquidos, não uma medida ponderada pelo tempo; a interface explicita essa diferença. Os dados e o câmbio continuam fictícios. Nenhuma sincronização com provedor foi validada.
- O ajuste de responsividade está no shell compartilhado e beneficia as demais rotas; o conteúdo dessas páginas não foi redesenhado.

Referências consultadas na skill: accessibility.md, layout.md, typography.md, color.md, designing-for-macos.md, buttons.md, charting-data.md, loading.md, writing.md e cross-platform.md. As citações acima se referem aos arquivos locais em `/home/samuel-barbosa/.agents/skills/apple-design-skill/references/`.
