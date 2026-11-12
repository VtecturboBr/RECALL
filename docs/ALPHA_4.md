# RECALL v0.5 Alpha 4 — Captura inteligente e organização temporal

Esta é a primeira de três entregas:
1. Alpha 4: captura inteligente e organização temporal
2. Próxima entrega: lembretes confiáveis, adiamento e recorrência
3. Entrega posterior: Onde parei? integrado às tarefas

## Captura de tarefas
Na tela Agora ou na janela global, escolha Tarefa. Exemplos:
- Revisar slides amanhã às 15h
- Fazer compras hoje
- Preparar apresentação depois de amanhã
- Pagar conta em 15/12 às 09:30
- Organizar fotografias algum dia
- Revisar tarefas semana que vem

O reconhecimento é offline, sem IA ou assinatura. Interpreta expressões simples no fim da primeira linha e mostra uma prévia antes de salvar. Datas inválidas ou passadas permanecem como texto e não provocam agendamento.

Datas de tarefas são prazos informativos e não geram notificações ainda.

## Organização temporal
O RECALL continua com apenas Agora e Arquivo. Tarefas agora podem ser programadas para Hoje, Depois e Algum dia.
Abra uma tarefa no Arquivo e altere o período. Se marcar Depois, você pode escolher opcionalmente uma data em que ela deve aparecer automaticamente em Hoje.
Tarefas legadas sem período explícito continuam em Hoje. O foco manual permanece disponível; focar uma tarefa futura move-a para Hoje.

## Segurança dos dados
Não alteramos colunas nem registros antigos das tabelas notes, tasks ou inbox_items. O SQLite ganha apenas uma tabela aditiva task_plans associada aos identificadores existentes. O identificador Tauri e o local dos dados não mudaram.

## Validação
GitHub Actions executa TypeScript/Svelte, testes unitários (incluindo datas), compilação Rust e empacotamento NSIS Windows x64.

Testes manuais necessários:
- [ ] Fazer backup antes de instalar sobre a Alpha 3.
- [ ] Conferir notas, tarefas e lembretes antigos.
- [ ] Capturar tarefas com Hoje, Amanhã, Algum dia e datas numéricas.
- [ ] Verificar que datas inválidas não são interpretadas.
- [ ] Mover tarefas entre Hoje, Depois e Algum dia.
- [ ] Verificar que tarefas programadas entram em Hoje na data escolhida.
- [ ] Reiniciar e confirmar que o planejamento ficou salvo.
- [ ] Testar captura global Ctrl+Shift+Espaço nos modos Ideia e Tarefa.

Alpha 4 é experimental e não equivale a um aplicativo instalado e validado em Windows real.

## Diagnóstico de qualidade (12/11/2026)

A revisão estática da Alpha 4 está em `docs/AUDITORIA_ALPHA4_2026-11-12.md`, com plano em `docs/ALPHA4_PLANO_CORRECOES.md` e testes manuais pendentes em `docs/ALPHA4_TESTES_MANUAIS.md`. Nenhum defeito foi corrigido por essa documentação; a PR #5 permanece em rascunho.
