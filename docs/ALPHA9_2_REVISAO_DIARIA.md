# RECALL Alpha 9.2 — Revisão diária

**Versão:** `0.5.0-alpha.9.2` (prévia Windows x64)  
**Branch:** `feature/v0.5-alpha9-2-revisao-diaria`  
**Base:** `feature/v0.5-alpha9-1-contexto-conectado` (PR #19).  
**Status:** implementação experimental, PR em rascunho, sem merge na `main`.

## Objetivo

Uma revisão simples, sob demanda, que recupera o contexto esquecido sem distrair a pessoa nem alterar automaticamente seus registros. Mantém o design escuro atual e é inteiramente offline.

## Experiência na tela Agora

1. Cartão **Revisão diária** abaixo de Próximas ações / Guarde o pensamento e acima dos lembretes.
2. O app **não** abre a revisão automaticamente: o usuário clica em **Revisar agora**.
3. A lista mostra no máximo **12 destaques por consulta**, não necessariamente todos os dados que exigem atenção.
4. Cada destaque permite **Abrir** a nota ou tarefa verdadeira no Arquivo e **Ocultar hoje** a sugestão, sem mexer em tarefas, notas ou lembretes.
5. **Mostrar os itens ocultados hoje** devolve os destaques ao painel. A ocultação fica somente no `localStorage`, escopada ao dia **local** e ao dispositivo atual.
6. **Atualizar** reconsulta o banco. Enquanto a revisão estiver aberta, mudanças em tarefas, lembretes e sessões de foco atualizam a lista.
7. Nenhuma notificação adicional, nenhum agendamento oculto, nenhuma conclusão ou reagendamento automático.

## Como os destaques são escolhidos

A consulta Rust `list_daily_review` reúne dados **existentes** de `reminder_rules`, `tasks`, `task_plans`, `task_resumes` e `focus_sessions`. O backend não escreve em nenhuma tabela.

A prioridade conservadora é:

| Prioridade | Tipo | Condição |
|---|---|---|
| 1 | **Lembrete pendente** | Regra ativa em `attention`, ou `scheduled/snoozed` cuja data passou |
| 2 | **Prazo passado** | Tarefa ativa com `due_at` anterior ao instante da consulta |
| 3 | **Foco interrompido** | Última sessão da tarefa é `paused` ou `interrupted` |
| 4 | **Sem próximo passo** | Tarefa com pelo menos 3 dias, sem detalhes e sem `next_step`, sem atualização recente |
| 5 | **Sem atividade recente** | Tarefa aberta sem atividade registrada por pelo menos 14 dias |

Regras de segurança semântica:
- Apenas tarefas **não concluídas e não excluídas**; lembretes apenas de itens válidos.
- Tarefa com plano **Algum dia** ou **Depois** com data futura não gera alerta de falta de progresso ou de próximo passo. Prazo explicitamente vencido continua sendo mostrado.
- O estado considerado de foco é a **última** sessão; uma sessão posterior finalizada substitui uma pausa anterior. Empate em `started_at` é resolvido pela ordem de inserção no SQLite.
- Há **no máximo uma sugestão por nota ou tarefa**. Se um item tem lembrete e prazo vencidos, o lembrete prevalece.
- Itens são ordenados por prioridade, tempo e ID para resultado determinístico.
- A lista é uma **ajuda**, não um diagnóstico de produtividade, e pode omitir itens além do limite de 12.

## Arquitetura

- `src-tauri/src/review.rs`: consultas SQL parametrizadas de leitura, filtro, ordenação, limite e testes de integração.
- `src-tauri/src/lib.rs`: novo comando Tauri `list_daily_review`, sem capacidade de modificar dados.
- `src/lib/types.ts`, `src/lib/api.ts`: tipos `DailyReviewItem` e `reviewApi.list`.
- `src/lib/dailyReview.ts`: dia local e ocultações, com testes Node.
- `src/components/DailyReview.svelte`: painel opt-in, atualização, navegação e ocultação do dia.
- `src/components/RecallStudio.svelte`, `src/recall.css`: integração à interface atual.

**Não há migração SQLite nova.** A Alpha 9.2 continua usando o esquema **v7** estabelecido na Alpha 9.1. Isso evita snapshots e migrações adicionais desnecessários. Ao instalar diretamente sobre Alpha 7/8/9.0, as migrações anteriores ainda serão executadas com seus próprios backups.

## Testes automatizados

- Revisão vazia retorna vazia, em consultas repetidas, sem criar eventos de lembrete.
- Lembretes pendentes e prazos vencidos são priorizados, com deduplicação entre causas e exclusão de itens finalizados.
- Sessão pausada só aparece se for a mais recente, não quando uma sessão mais nova já terminou.
- Tarefas `someday` são respeitadas, exceto quando possuem prazo explicitamente vencido.
- Tarefa sem próximo passo após 3 dias e tarefa antiga com detalhes após 14 dias recebem razões distintas.
- Limite de 12 destaques sem excluir ou modificar as 20 tarefas usadas no teste.
- Dia local e ocultação por motivo/tipo/ID, sem mutações, verificadas em testes JavaScript.
- Regressões das Alphas 7.1, 8.1, 9.0 e 9.1 (SQLite, foco, links, lembretes e edição).

## Roteiro de teste integrado — Windows

1. **Fechar completamente** o RECALL, incluindo ícone próximo ao relógio do Windows.
2. Copiar a pasta inteira `%APPDATA%\app.memoriaexterna.desktop`, incluindo backups e possíveis `-wal`/`-shm`. Teste com uma **cópia** do banco, preferencialmente em outro usuário Windows.
3. Depois de **Success** no workflow `Gerar instalador RECALL Windows`, baixar `RECALL-v0.5-alpha9-2-revisao-diaria-Windows-x64`; extrair e instalar o `.exe` NSIS (SHA-256 em `SHA256SUMS.txt`).
4. Verificar dados e versões anteriores: notas, tarefas, ideias, apresentações, Modo Foco 2.0, vínculos Alpha 9.1, lembretes e pontos de retomada.
5. Abrir **Agora**; a revisão **não** deve enviar nenhuma notificação nem mudar tarefas sem interação.
6. Clicar **Revisar agora** e verificar os destaques. Se necessário, usar um lembrete de teste vencido e uma tarefa antiga sem próximo passo.
7. Abrir um destaque: o registro original deve abrir no Arquivo sem perder alterações pendentes em outro item.
8. Ocultar sugestão somente hoje, fechar/reabrir RECALL e verificar que ela fica oculta naquele dia; clicar **Mostrar itens ocultados** para restaurar a lista.
9. Confirmar que os campos do banco e as notificações originais não foram modificados pela revisão.
10. Repetir as regressões anteriores: criar tarefa sem título, sair de ideia vazia, excluir nota de teste, conectar nota a tarefa, pausar/retomar foco e reabrir a aplicação.
11. Confirmar `PRAGMA user_version=7` e **nenhum novo upgrade de esquema v8** para quem já usa Alpha 9.1.

## Pendências

A revisão diária desta fase é **manual**. Não há alertas ativos, sugestões por IA paga, projetos hierárquicos, Android ou sincronização. A8.2 (lixeira), A8.3 (restauração pelo histórico) e A8.4 (busca) seguem pendentes.

**Não fazer merge na `main` nem considerar estável sem CI completa, instalador aprovado e validação manual em Windows.**
