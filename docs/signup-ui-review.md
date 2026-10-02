# Cadastro — Astra Wealth

A rota `/signup` reutiliza o AuthShell e o CSS Module do login: mesma marca, paleta escura, coluna de 400 px, tipografia, campos, botões e comportamento responsivo. O shell recebe título e subtítulo opcionais, preservando os valores padrão do login. O formulário fica separado em SignUpForm.

O fluxo começa pelo e-mail, permite voltar para alterá-lo e apresenta a senha com opção de revelar. Rótulos, autocomplete, foco e mensagens estão em português. O retorno do Supabase sem sessão apresenta instruções de confirmação; com sessão, segue para o destino interno validado. A confirmação segue a documentação: https://supabase.com/docs/reference/javascript/auth-signup.

Verificação: TypeScript, build de produção Webpack e git diff --check passaram. O detector da skill não apontou problemas nos componentes examinados. Capturas desktop e mobile foram inspecionadas; o roteiro executou verificações em 1440, 768, 390 e 320 px, navegação por teclado, e-mail inválido, foco/revelação da senha, alteração do e-mail e texto a 200% antes de parar na confirmação.

Limites: o teste completo com confirmação simulada não concluiu. Não há variáveis locais do Supabase; tentativas de iniciar uma instância de teste com configuração fictícia encontraram indisponibilidade do servidor. Cadastro real, entrega de e-mail e OAuth não foram verificados. O ícone Google compartilhado pelo login depende de uma imagem externa que não apareceu nas capturas. Nenhuma configuração de provedor foi alterada.
