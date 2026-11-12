# RECALL v0.5 — Alpha 3 (experiência de uso)

## Objetivo
Ajustar o fluxo de trabalho do RECALL sem criar novas telas ou migrar o banco de dados.

## Melhorias
- Criar ideias e tarefas diretamente no campo de captura da tela **Agora**, sem classificação obrigatória posterior.
- Selecionar a tarefa em foco; escolha lembrada localmente no Windows e removida quando a tarefa é concluída.
- Editar **tarefas** e **ideias** com salvamento automático, como já ocorria nas notas.
- Impedir a troca de item quando houver falha de gravação de dados pendentes.
- Editar uma ideia sem a necessidade de transformá-la em nota.
- Pesquisa **sem distinção de maiúsculas/minúsculas e acentos**, filtros simples de tipo e opção de limpar busca.
- Visual mais discreto, com melhor hierarquia, estados de seleção e foco via teclado.

## Segurança dos dados
O banco SQLite e seu identificador de aplicativo continuam inalterados. Não há mudança de tabelas nesta etapa. A nova operação update_inbox modifica apenas o texto de um registro ainda não processado; não cria duplicatas. A preferência de foco fica no localStorage da aplicação e não inclui conteúdo sensível.

## Verificações
- npm run check — diagnóstico Svelte/TypeScript.
- npm run test:unit — busca, agrupamento, ordenação, filtros e apresentações.
- npm run build e cargo test --lib — compilação do frontend e testes Rust.
- Build NSIS no GitHub Actions para Windows x64.

## Testes manuais necessários
- Criar uma tarefa diretamente na tela Agora.
- Escolher uma outra tarefa como foco e reiniciar a aplicação.
- Editar tarefa e ideia, mudar de item, reiniciar e confirmar persistência.
- Buscar por "reuniao" e encontrar "Reunião".
- Confirmar que a janela de captura global continua funcionando.
- Confirmar que notas, cartões flutuantes e notificações não regrediram.
- Verificar preservação de dados de versões anteriores antes de considerar merge.

Esta Alpha 3 não inclui lembretes recorrentes, sync Android ou migração de banco.
