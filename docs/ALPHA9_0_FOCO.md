# RECALL Alpha 9.0 — Modo Foco 2.0

**Branch de implementação:** `feature/v0.5-alpha9-focus-2`  
**Base:** `feature/v0.5-alpha8-migracao` (PR #16), com correções da Alpha 7.1 já integradas.  
**Estado:** primeira entrega funcional de Alpha 9; **não equivale à conclusão de toda a Alpha 9 nem substitui A8.2–A8.4**.

## Objetivo

O RECALL deixa de apenas guardar a tarefa atual e passa a ajudar a retomar o trabalho com contexto e progresso real, mantendo a tela **Agora** simples.

## Entrega implementada

- Painel discreto **Modo Foco 2.0** dentro da tarefa atual na tela Agora, acima de **Onde parei?**.
- **Iniciar sessão** para uma tarefa não concluída e não excluída.
- **Cronômetro persistido no SQLite**, atualizado por pulsos periódicos do frontend (intervalo aproximado de 12 s).
- **Pausar** ou **Finalizar sessão**, registrando duração e um breve resumo do que foi feito (máximo 2.000 caracteres). Esse resumo também fica em armazenamento local durante a digitação, até a gravação.
- **Retomar a tarefa** iniciando uma nova sessão, com as sessões anteriores ainda disponíveis em histórico local.
- **Últimas sessões** na própria tarefa: pausadas, finalizadas ou interrompidas, duração e resumo.
- **Apenas uma sessão ativa por banco**: índice parcial único no SQLite. Outra janela não pode iniciar uma sessão paralela em outra tarefa sem parar a anterior.
- Excluir ou concluir a tarefa encerra a sessão ativa atomicamente na mesma transação; sem sessões zumbis.
- Ao reiniciar o aplicativo, sessões ainda ativas continuam disponíveis para retomar. O tempo offline **não é contado integralmente**; cada heartbeat adiciona no máximo 20 segundos.
- Sem nuvem, chamadas externas, IA paga, novos painéis ou modificações do conteúdo de notas/tarefas.

## Migração v5 → v6

- `PRAGMA user_version=6`.
- Banco já existente exige um backup SQLite `pre-alpha9` validado por `PRAGMA quick_check` **antes** de qualquer criação de tabela/índice Alpha 9.
- A tabela `focus_sessions` e índices são criados em **uma única transação** com a versão v6. Falha reverte as alterações.
- Bancos com versão acima de 6 são recusados para evitar corrupção por downgrade.
- Históricos de notas `content_history`, disposição de ideias `inbox_dispositions`, tarefas, lembretes e `task_resumes` continuam válidos e intactos.
- No upgrade direto de Alpha 7 ou anterior para Alpha 9, a migração v5 anterior continua obrigatória e precede a v6, guardando seus backups. **Nunca testar sobre a única cópia do banco real.**

### Estrutura

`focus_sessions`: `id`, `task_id`, `started_at`, `last_tick_at`, `active_ms`, `ended_at`, `state` e `summary`. Estados: `active`, `paused`, `finished`, `interrupted`.

`src-tauri/src/focus.rs` é proprietário do módulo, migração e invariantes. IPC Tauri exposto em `lib.rs` com contratos tipados em `src/lib/types.ts` e `src/lib/api.ts`. UI independente em `src/components/FocusSession.svelte` e helpers puros em `src/lib/focusTime.ts`.

## Segurança de dados e limitações

1. O cronômetro **não é um rastreador de produtividade exato**. Ele registra tempo confirmado pelos heartbeats; suspensão do Windows, fechamento inesperado ou WebView pausada podem deixar pequenas lacunas, mas não inflacionar horas.
2. O texto do resumo digitado e ainda não gravado sobrevive normalmente em localStorage por sessão. Não equivale a backup SQLite e pode falhar se o armazenamento do navegador estiver indisponível.
3. Sessões de foco não substituem nem sobrescrevem os campos **Onde parei?**; são registros separados e complementares.
4. A UI de lixeira e recuperação de versões planejada para a Alpha 8 permanece pendente. Itens excluídos continuam soft-deleted sem lixeira visual.
5. Não existe sincronização com Android ou login.
6. Histórico limita a exibição às 8 sessões mais recentes por tarefa, mas **não remove** sessões anteriores do banco.
7. O recurso não aciona notificações, gravação de tela, captura de janelas, telemetria ou envio a servidores.

## Testes automatizados

- Migração e reabertura com um banco v5, preservando tarefas e gerando backup pré-v6.
- DDL incompleto: migração falha e mantém versão v5 e dados anteriores.
- Iniciar/pausar/finalizar, gravar resumo, reabrir e carregar histórico.
- Duas tarefas não podem manter sessões simultâneas; tarefa concluída ou excluída não inicia sessão.
- Excluir tarefa encerra a sessão ativa; heartbeats posteriores são rejeitados.
- Heartbeat após suspensão de 1h adiciona no máximo 20 segundos; estado inválido e resumo longo são rejeitados.
- Testes Node do contador/limites, além da CI anterior da Alpha 8.1.

## Teste manual Windows — apenas o instalador Alpha 9.0

1. **Fechar o RECALL** e copiar integralmente `%APPDATA%\app.memoriaexterna.desktop`, incluindo backups e eventuais `-wal`/`-shm`. Preferir testar em conta separada do Windows com uma cópia da base.
2. Baixar o artefato `RECALL-v0.5-alpha9-0-focus2-Windows-x64` da execução **Success** do instalador no GitHub; extrair o ZIP e instalar o NSIS `*-setup.exe`.
3. Verificar que suas notas, tarefas, ideias e **Onde parei?** permanecem intactos após migração v6; o backup `pre-alpha9` existe.
4. Em **Agora**, iniciar o Modo Foco 2.0. Trabalhar 30–60 segundos, registrar o resumo, pausar e confirmar histórico.
5. Retomar a tarefa, iniciar nova sessão, finalizar; reiniciar o aplicativo e conferir os dois registros.
6. Iniciar foco e tentar abrir outra tarefa, observando a restrição de sessão única; voltar à tarefa anterior para pausar.
7. Iniciar foco, fechar o aplicativo inesperadamente e abrir de novo; verificar que o cronômetro não conta todo o período offline.
8. Concluir e excluir tarefas de teste com foco ativo: devem encerrar sessão sem registros órfãos; lembretes e texto permanecem coerentes.
9. Repetir os bugs reportados antes: abrir/sair de ideia com campo vazio, excluir nota e criar tarefa sem nome. Nenhum deles deve regredir.
10. Confirmar que apresentação, busca, lembretes, Quick Capture e notas flutuantes continuam funcionando.

## Próximas entregas Alpha 9

| Etapa | Escopo |
|---|---|
| **9.0 — Foco 2.0** | Sessões, resumos, pausas e retomada (esta entrega) |
| **9.1 — Contexto conectado** | Ligações locais entre tarefas, notas e ideias |
| **9.2 — Revisão diária** | Sugestões conservadoras, sem notificação excessiva |
| **9.3 — Captura rápida 2.0** | Atalhos, fluxos e estabilização |

Essas etapas serão PRs incrementais próprios. A8.2–A8.4 continuam no roadmap e não são declaradas concluídas.

**Política:** cada entrega passa por CI, instalador Windows e teste manual. Nenhum merge automático na `main`.
