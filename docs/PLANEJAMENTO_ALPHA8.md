# RECALL v0.5 Alpha 8 — Recuperação e organização segura

**Status:** planejamento aprovado para revisão; **nenhuma implementação de produto neste PR**.  
**Data:** 13/11/2026.  
**Base de implementação proposta:** `feature/v0.5-alpha7-onde-parei` (PR #14), após sua validação; cadeia Alpha 6 (#10–#13) ainda em rascunho.  
**Identidade:** editorial minimalista, escuro/vermelho já existente, Windows-first e 100% offline.

## 1. Por que esta versão existe

O RECALL já permite capturar, pesquisar, editar, fixar, planejar, lembrar e registrar "Onde parei?". A Alpha 8 deve fazer a pessoa **confiar que pode recuperar o que apagou ou alterou por engano**, e localizar informações sem uma nova interface complexa.

Objetivos, em ordem:
1. **Lixeira com restauração** de notas, apresentações, tarefas e ideias descartadas.
2. **Histórico recuperável do conteúdo** de notas, tarefas, ideias e pontos de retomada.
3. **Arquivo/busca mais úteis**, mantendo o comportamento atual e garantindo que itens excluídos não apareçam no resultado comum.

**Não inclui:** contas, nuvem, Android, IA online, tags/pastas complexas, colaboração, exclusão permanente, sincronização remota nem redesign completo. Nenhuma dependência de serviço pago.

## 2. Diagnóstico confirmado no código da Alpha 7

- `src-tauri/src/db.rs`: notas possuem `deleted_at` (soft delete), `note_revisions`, CAS e snapshot SQLite verificado via `VACUUM INTO` + `quick_check`.
- `src-tauri/src/tasks.rs`: tarefas também usam `deleted_at`, `task_revisions`; `task_plans` e `task_resumes` são tabelas auxiliares. Checkpoints possuem revisão própria.
- `src-tauri/src/inbox.rs`: `processed_at` **não significa apenas exclusão**. É usado tanto ao descartar quanto ao converter uma ideia em nota/tarefa. Dados antigos não informam qual ação ocorreu.
- `src-tauri/src/lib.rs`: exclusão de nota/tarefa e cancelamento de lembrete são hoje duas operações separadas. Tornar atômicas antes de expor a lixeira.
- `src/lib/workspace.ts`: a busca atual filtra notas, tarefas e ideias carregadas em memória, ignorando acentos e maiúsculas; trabalha somente com itens ativos. Preservar essa semântica.
- Histórico de revisão atual é **apenas um contador**; não existe cópia de versões anteriores do texto.
- SQLite atual chega a `PRAGMA user_version=4`. A Alpha 8 propõe `user_version=5`.
- A CI e o instalador do último commit Alpha 7 terminaram com sucesso. Isso **não equivale a testes manuais Windows** e não autoriza merge na `main`.

## 3. Experiência desejada (sem novos dashboards)

### Agora
Sem mudanças na composição da página. O foco, a captura e os lembretes seguem iguais.

### Arquivo
Manter a pesquisa e os filtros existentes. Acrescentar **Lixeira** como opção secundária do Arquivo, com um contador discreto se couber. Não exibir cartões excluídos na pesquisa normal.

Na lista de itens ativos, excluir deve ser descrito como **Mover para lixeira**; oferecer retorno imediato **Desfazer** após operação confirmada, sem garantir desfazer quando o aplicativo for encerrado. A recuperação durável é pela Lixeira.

### Lixeira
- Listagem compacta por tipo, título/primeira linha, data de exclusão e pesquisa restrita à lixeira.
- Abrir um item em modo **somente leitura** com **Restaurar**. Não permitir editar um item excluído sem restaurá-lo primeiro.
- Restaurar preserva o **ID original, conteúdo, criação, pin e planejamento**.
- **Não rearmar lembretes automaticamente** depois da restauração; informar "Lembrete desativado — reagende se quiser". Evita notificações inesperadas, inclusive de horários já vencidos.
- Uma tarefa concluída antes de ser excluída volta concluída. `task_resumes` continua vinculada ao mesmo `task_id`.
- Para ideias antigas com `processed_at` mas sem informação de destino, **não presumir que foram descartadas**. São dados legados ambíguos; não devem aparecer automaticamente como lixo. Opcionalmente, exibir sob "Processadas antigas" como arquivo somente leitura com ação **Criar cópia como ideia**.
- Excluir definitivamente e "Esvaziar lixeira": **fora do escopo da Alpha 8**. Não apagar fisicamente registros.
- O diálogo de restauração informa sucesso/erro real; em conflito, manter o registro e permitir recarregar a versão atual.

### Histórico de versões
No inspector de um item ativo: botão secundário **Histórico**. Exibir hora, tipo de edição, resumo da versão e uma prévia somente leitura; destacar qual é atual.

- **Restaurar esta versão** não apaga histórico nem volta a contagem de revisão: grava o texto antigo como **uma nova revisão**.
- Se houver um rascunho local ainda não salvo, bloquear restauração até salvar, preservar como cópia ou cancelar a ação explicitamente.
- Se uma janela diferente alterou o item, rejeitar restauração com a mesma regra CAS já usada nos editores.
- Versões cobertas: título/corpo de nota e apresentação; título/detalhes de tarefa; texto de ideia; `last_context`/`next_step` do ponto de retomada. Planejamento, pin, lembretes e metadados não são restaurados pelo histórico **nesta fase**; não insinuar o contrário na interface.
- Não prometer versões anteriores à instalação Alpha 8. Criar apenas uma **versão inicial de referência** do conteúdo existente durante a migração.
- Não armazenar cada caractere digitado; registrar **gravações confirmadas**. Evitar duplicar conteúdo em comandos sem mudança efetiva.

### Busca e organização
- Manter normalização por acentos/caixa (`pt-BR`) e correspondência de trechos já existente, inclusive dentro de palavras.
- Acrescentar filtros leves: `Todos / Notas / Tarefas / Ideias`; `recentes`, `fixados` (notas), `concluídos` (tarefas) e ordenação `atualizados`/`criados`, sem múltiplos painéis.
- Exibir pequeno trecho onde houve ocorrência, sem interpretar Markdown/HTML do usuário e sem destaque injetando HTML.
- Busca no histórico somente dentro do item aberto, e busca da lixeira somente quando a aba Lixeira está ativa.
- Para escala inicial, otimizar e testar a busca atual em memória. **FTS5 é opcional**, condicionado a medições e teste de suporte no SQLite empacotado, nunca obrigatório nem mudança silenciosa da semântica (FTS5 por tokens não equivale a substring). Se não houver suporte, o sistema deve funcionar da mesma maneira.
- Fechar painéis e limpar filtros incompatíveis após exclusão/restauração, sem descartar rascunhos.

## 4. Arquitetura e decisões de banco

### 4.1 Migração v4 → v5

Criar `src-tauri/src/recovery.rs` (orquestração de lixeira + eventos) e `src-tauri/src/history.rs` (snapshots + recuperação). Os módulos existentes permanecem proprietários de suas tabelas. `lib.rs` fornece IPC e emite eventos apenas depois do commit.

Na abertura de banco **existente** com versão inferior a 5:
1. Verificar esquema presente e integridade/capacidade de backup.
2. Criar backup `recall-pre-alpha8-*.sqlite` via infraestrutura atual; se falhar, **abortar migração**.
3. Migrar tabelas, índices, triggers, baseline do histórico e `PRAGMA user_version=5` numa **única transação**; rollback em qualquer erro.
4. Reabrir/validar esquema e `PRAGMA quick_check` em banco de teste (protocolo de instalação); manter snapshot intacto.
5. A segunda inicialização não deve repetir snapshot ou migração; banco v5 não deve ser interpretado/reescrito por código antigo sem considerar compatibilidade.

Nenhum `DROP TABLE`, reescrita destrutiva de registros ou `DELETE FROM` de dados do usuário.

### 4.2 Lixeira

Notas/tarefas: reutilizar `deleted_at`, filtrando ativas como hoje; criar `list_trash` por consulta `UNION ALL` com paginação, discriminador `note`/`task`/`idea`, data e revisão. IDs são nomes de namespace, não texto de query.

Ideias: adicionar uma **tabela auxiliar** `inbox_dispositions`, com `item_id` (PK), `state` (ex.: `dismissed`, `converted_note`, `converted_task`, `restored`), `target_id` opcional e `changed_at`; registrar a ação e a mudança de `processed_at` na **mesma transação**. Não criar classificação presumida para `processed_at` antigos sem linha auxiliar. Ações posteriores devem verificar `processed_at IS NULL` e, onde aplicável, a revisão esperada.

Operações:
- `move_to_trash(type,id,expectedRevision)`: para nota/tarefa, definir `deleted_at` e cancelar lembrete no **mesmo COMMIT**; para ideia, registrar `dismissed` com `processed_at`. Evitar dupla deleção silenciosa.
- `list_trash(limit,offset,type,query)`: somente itens de fato descartados; limites server-side, parâmetros vinculados e ordenação estável por `deleted_at DESC, type, id`.
- `restore_trash_item(type,id,expectedRevision)`: validar estado e CAS, desfazer tombstone, preservar IDs e metadados e emitir evento de atualização após COMMIT. Para ideia, voltar `processed_at=NULL` e marcar a disposição como `restored`.
- `list_legacy_processed_ideas` / `copy_legacy_processed_idea` como extensão controlada da Alpha 8 caso os testes mostrem necessidade; nunca religar uma conversão antiga sem decisão explícita.

Nota: neste projeto `delete_note` e `delete_task` já existem; manter compatibilidade IPC, mas fazer os comandos chamarem a mesma operação transacional central, evitando dois caminhos divergentes. Preservar os eventos `notes:changed`, `tasks:changed`, `inbox:changed`, `reminders:changed` e adicionar `trash:changed` quando necessário.

### 4.3 Histórico do conteúdo

Tabela aditiva proposta `content_history` com:
- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `item_type TEXT NOT NULL` CHECK em `note|task|idea|resume`
- `item_id TEXT NOT NULL`
- `source_revision INTEGER NOT NULL` (revisão do conteúdo capturado, não da operação futura)
- `snapshot_json TEXT NOT NULL` com campos permitidos do tipo, incluindo schema version
- `reason TEXT NOT NULL` (`baseline|before_edit|before_restore` ou semântica equivalente)
- `captured_at INTEGER NOT NULL`
- `UNIQUE(item_type,item_id,source_revision)`, índice `(item_type,item_id,captured_at DESC,id DESC)`.

Criar **baseline apenas uma vez** para conteúdo encontrado na migração. Para gravações futuras, usar triggers **BEFORE UPDATE** condicionados à alteração real dos campos de texto, salvando `OLD`; assim até uma versão antiga que execute SQL diretamente fica coberta. Integrar com os triggers de revisão existentes sem depender da ordem entre triggers `AFTER UPDATE`. Para `task_resumes`, `revision` já é campo da própria linha. Evitar histórico repetido em no-op; ter cuidado com caracteres UTF-8/JSON, citações e limites de tamanho.

`list_item_history(type,id,limit,cursor)` retorna apenas resumo/metadados paginados; `get_history_snapshot(historyId)` lê o conteúdo completo sob demanda. Nunca carregar todas as versões no startup.

`restore_content_version(type,id,historyId,expectedRevision)`:
1. Verifica que snapshot pertence ao mesmo item e que item está ativo;
2. Valida payload, limites atuais e tipo;
3. Usa o mecanismo **CAS** do módulo proprietário e transação única;
4. Persiste o conteúdo antigo como **nova revisão**, sem restaurar lembretes/pin/deleted_at/status por acidente;
5. Garante que o conteúdo atual continue recuperável no histórico;
6. Publica evento ao frontend somente após `COMMIT`.

Sem política automática de purga nesta Alpha. Histórico e backup ocupam disco; medir e mostrar tamanho nas configurações/backup e discutir retenção em versão posterior, **nunca descartar versões silenciosamente**. Histórico e backups guardam conteúdo pessoal sem criptografia adicional; não prometer privacidade contra acesso ao disco por outros usuários.

### 4.4 Segurança e consistência

- Validar `type` por enum/allowlist, IDs e revisões nos comandos Rust; toda query com `bind`, não concatenar texto do usuário em SQL.
- Operações de excluir/restaurar/histórico são transacionais; consultas sem efeito colateral.
- Não restaurar lembrete expirado nem mudar confirmação de tarefa. Cancelar no trash; explicitamente permitir agendamento novo depois de restaurar.
- Rascunhos não confirmados continuam no cache local existente; **histórico SQLite não é backup de texto que nunca foi salvo**.
- Tratar conflito de `expectedRevision` com mensagem de recarregamento e cópia, não sobrescrever.
- Logs, notificações Windows, telemetria e notificações de erro não devem expor texto de notas/ideias.
- Testar migrações com um SQLite copiado, jamais com arquivo de dados de produção.

## 5. Contratos e módulos

Backend:
- `db.rs`: migração v5 e função de backup existente.
- `tasks.rs`, `inbox.rs`, `reminders.rs`: preservar invariantes de conversão, checkpoint e lembretes; adicionar variantes transacionais onde necessário.
- `recovery.rs`: listagem paginada e restore/delete CAS.
- `history.rs`: persistência de versões e CAS de restauração.
- `lib.rs`: comandos Tauri, validação e eventos.

Frontend:
- `src/lib/types.ts`: `TrashEntry`, `ContentHistoryEntry`, `HistorySnapshot` e estados.
- `src/lib/api.ts`: clientes IPC tipados.
- `src/components/RecallStudio.svelte`: navegação Lixeira e aba Histórico no inspector; evitar concentrar todas as lógicas neste componente de >1200 linhas; criar `TrashView.svelte`, `HistoryPanel.svelte`.
- `src/lib/workspace.ts`: filtros e ordenação como funções puras com testes.
- `src/recall.css`: estilo existente, foco visível e zoom 125/150/200%, sem redesign completo.

**Critério de compatibilidade:** nada de renomear IDs existentes, alterar diretórios de dados, quebrar Quick Capture/Sticky ou gerar chamadas IPC inacessíveis a janelas existentes.

## 6. Ordem sugerida de implementação (PRs separados, empilhados)

1. **A8.1 — Segurança e migração** (`feature/v0.5-alpha8-migracao`): snapshot, v5, disposições de ideias, histórico baseline e triggers, testes de upgrade/rollback; sem UI.
2. **A8.2 — Lixeira** (`feature/v0.5-alpha8-lixeira`): comandos Rust CAS, transações de lembretes, listar/restaurar, estados da UI, testes de várias janelas.
3. **A8.3 — Histórico** (`feature/v0.5-alpha8-historico`): visualizar snapshots, restaurar como nova revisão e resolver conflitos, integração ao inspector e pontos de retomada.
4. **A8.4 — Busca/estabilização** (`feature/v0.5-alpha8-busca-estabilizacao`): filtros, snippets, performance, acessibilidade, regressões, documentação, instalador Alpha 8 separado.

Base de A8.1: Alpha 7 validada; A8.2 usa A8.1 como base; A8.3 usa A8.2; A8.4 usa A8.3. **Draft PR em cada etapa**; nunca dar merge na `main` automaticamente. Preferir implementação e revisão incremental com testes em cada commit.

## 7. Matriz de testes mínima

**Rust + SQLite:**
1. Migrar cópias de v0.4/v0.5 Alpha 6/Alpha 7: IDs, conteúdo, tarefas, lembretes, checkpoints e contagens iguais.
2. Falha deliberada no backup: migração impedida e banco antigo íntegro; falha no meio da migração faz rollback.
3. Inicializar duas vezes: banco versionado, sem baseline duplicado nem snapshots de migração repetidos.
4. Excluir nota/tarefa + cancelar lembrete: sucesso e rollback simulando erro SQL, incluindo histórico preservado.
5. Restaurar com revisão antiga em duas janelas: exatamente uma transação vence; outra recebe conflito.
6. Descartar ideia vs converter ideia em nota/tarefa: somente descarte novo vai à lixeira; legado ambíguo não é perdido.
7. Restaurar tarefa com `task_plan`, `task_resume` e conclusão anterior; não rearmar lembrete cancelado.
8. Triggers capturam uma versão anterior após escrita por SQL legado, inclusive no texto de retomada; salvar o mesmo texto não duplica snapshot.
9. Restaurar versão cria nova revisão e preserva a anterior; conteúdo concorrente nunca sobrescrito.
10. Busca normal não revela tombstones; busca na lixeira não retorna ativos nem ideias convertidas.
11. Testar Unicode, acentos, emojis, multiline, texto grande, strings com aspas e entrada SQL adversarial.
12. Snapshot de pré-migração passa `PRAGMA quick_check`; comparar hash/contagens antes e após restauração em banco descartável.

**Node/Svelte:** filtros de busca, normalização, ordenação, recuperação do inspetor após restore, paginação, snapshots listados sem conteúdo pesado, estados de loading/erro e teclado.

**Windows real, obrigatório:** instalar sobre **cópia** do banco Alpha 7; conferir lixeira e restauração após fechar/reabrir; editar o mesmo item em duas janelas; Crash/kill durante autosave e restore; Sticky, captura global, lembretes nativos, backup, apresentação, zoom 125–200%, teclado/aria; validar 1k e 10k itens sintéticos para busca e histórico sem travamentos.

**Pipeline:** `npm run check`, `npm run test:unit`, `npm run build`, `cargo check`, `cargo test --lib`, `python -m unittest discover -s scripts -p "test_*.py"`, Windows NSIS + SHA-256. Adicionar testes cobrindo casos acima; nenhum teste falho pode ser suprimido para "passar CI".

## 8. Critérios objetivos de aceite

- [ ] Nenhuma exclusão de nota/tarefa/ideia nova elimina conteúdo; lixeira recupera ID e corpo original.
- [ ] Ideia convertida não aparece como excluída; ideias antigas ambíguas nunca são reclassificadas por suposição.
- [ ] Histórico de conteúdo funciona nas quatro modalidades e restaura como revisão **nova**, sem ressuscitar lembretes nem destruir a versão atual.
- [ ] Duas janelas não sobrescrevem notas, tarefas, ideias, checkpoints, restores nem edições restauradas.
- [ ] Backup pré-v5 verificado, migração repetível, sem operações destrutivas, rollback real testado.
- [ ] Pesquisa preserva o resultado esperado sem acentos/caixa e exclui tombstones do Arquivo comum.
- [ ] Interface continua simples, acessível, com performance aceitável nos cenários testados e offline.
- [ ] CI Windows e instalador NSIS da revisão final aprovados.
- [ ] Testes manuais reais Windows concluídos, especialmente com cópia do banco e desligamento forçado.
- [ ] `main` intocada até aprovação explícita; documentação e `docs/AI_CHECKPOINT.md` atualizados.

## 9. Comando para IA implementadora

Leia primeiro `docs/PLANEJAMENTO_ALPHA8.md`, `docs/ALPHA_7_ONDE_PAREI.md`, `docs/ESTABILIZACAO_ALPHA6.md` e os módulos Rust atuais. Faça uma auditoria de compatibilidade, verifique a base da Alpha 7 e os PRs empilhados. Implemente **somente A8.1**, começando pela migração reversível e seus testes; apresente diff e riscos, abra branch + draft PR próprio, sem merge. Após CI passar, avance incrementalmente para A8.2, A8.3 e A8.4 com revisão específica de perda de dados a cada etapa. Não assuma validação manual, não acrescente serviços externos, não faça operações destrutivas nem altere dados reais.
