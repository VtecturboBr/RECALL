# RECALL — Simplificação da interface (alpha 2)

## Objetivo
Duas áreas principais: **Agora** (próxima tarefa e próximos lembretes) e **Arquivo** (uma pesquisa para notas, tarefas e ideias).

## Fluxo
Capturar salva uma ideia sem precisar classificá-la. Ela aparece no Arquivo. Ao abri-la, o usuário pode mantê-la como ideia, transformá-la em nota editável ou em tarefa. O conteúdo não é duplicado na lista quando transformado.

Apresentar, abrir cartão flutuante e agendar lembrete são ações contextuais de notas, não seções da navegação. A tela Agora exibe uma tarefa em destaque, três próximas tarefas e lembretes de notas que ainda não foram notificados.

## Dados legados
**Não existe migração destrutiva nesta etapa.** A interface usa uma visão unificada das tabelas notes, tasks e inbox_items, preservando todos os IDs antigos, datas e persistência local. O código da interface anterior permanece no repositório para facilitar rollback.

## Limitações
Datas de tarefas são prazos informativos, não alarmes. Lembretes de notas continuam ativos no processo nativo Tauri. Testes de captura, persistência e notificações precisam ser feitos no Windows real. TESTAR_AGORA.html é a prévia antiga, da v0.4.

## Teste manual
1. Capturar uma ideia e encontrar no Arquivo.
2. Converter uma ideia em nota, editar, fechar e reabrir.
3. Converter outra ideia em tarefa, concluir em Agora e reabrir.
4. Abrir uma nota em cartão flutuante e testar Apresentar.
5. Conferir lembrete de nota no Agora e após reinício.
6. Confirmar que notas da alpha anterior foram preservadas.
