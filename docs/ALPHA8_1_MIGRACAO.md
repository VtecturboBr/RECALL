# RECALL Alpha 8.1 — Segurança e migração do SQLite

**Branch:** `feature/v0.5-alpha8-migracao`  
**Base de PR:** `plan/v0.5-alpha8-recuperacao-organizacao` (#15), sobre Alpha 7 (#14).  
**Escopo:** apenas preparação de dados/histórico e classificação de descarte/conversão; **sem lixeira, restore, frontend novo ou merge**.

## Mudanças efetivas

- Banco abre agora em **`PRAGMA user_version=5`**. Bases com versão superior a 5 são rejeitadas, evitando downgrades destrutivos.
- Quando já existe conteúdo e `user_version < 5`, o RECALL cria um snapshot **`recall-pre-alpha8-*.sqlite`** via `VACUUM INTO`. O arquivo é verificado com `PRAGMA quick_check` antes de ser considerado backup. Se falhar, o startup interrompe a migração.
- Migração v5 usa **uma transação SQLite**: cria `content_history`, `inbox_dispositions`, índices, snapshots iniciais, triggers e escreve o número de versão **antes do COMMIT**.
- `content_history`: snapshot JSON por item/revisão, com `item_type` (`note`, `task`, `idea`, `resume`), `item_id`, `source_revision`, `reason` (`baseline` ou `before_edit`) e `captured_at`. Histórico de notas, apresentações (como nota), tarefas, ideias e `Onde parei?`.
- Baselines contemplam itens ativos e itens já processados/excluídos; representam **o estado disponível no instante da migração**, não versões antigas não registradas.
- Triggers `BEFORE UPDATE` preservam o estado textual anterior ao próximo salvamento (inclusive via SQL legado) e não duplicam histórico em edições sem mudança textual. Um trigger adicional aumenta a revisão de checkpoints caso uma edição externa não o faça.
- `inbox_dispositions` passa a registrar **novos** eventos de `dismissed`, `converted_note` ou `converted_task` com `target_id` quando convertido, atomicamente com o fluxo existente.
- **Nunca inferir** que um `inbox_items.processed_at` antigo foi descarte: conversões anteriores possuem o mesmo marcador. Esses registros não recebem disposição inventada.
- Reabertura de banco v5 **verifica** a presença das novas tabelas/triggers e falha de forma explícita caso estejam ausentes; não inventa um histórico perdido silenciosamente.

## Arquivos

- `src-tauri/src/history.rs`: criação de esquema, baselines, triggers, verificação e testes de integridade.
- `src-tauri/src/db.rs`: backup pré-v5, versão de esquema, migração e verificação.
- `src-tauri/src/inbox.rs`: fluxo transacional de classificação das ideias.
- `src-tauri/src/lib.rs`: registro do novo módulo.

## Preservação e limites

- Sem mudança nos IDs, colunas atuais de conteúdo, caminho de banco ou contratos IPC.
- Sem `DELETE` de registros de usuário e sem `DROP` em migrações de produção.
- Nenhuma restauração de conteúdo disponível nesta etapa; isso é trabalho de A8.2/A8.3.
- Repetir `connect` com banco v5 não cria novos baselines nem backups de migração.
- Snapshots duplicam o texto em disco; dados não são enviados à nuvem, mas **não existe criptografia extra** para histórico/backup.
- Históricos posteriores à migração são gerados quando o conteúdo é efetivamente alterado. Rascunhos nunca salvos continuam cobertos apenas pelo mecanismo local de recuperação, não por esse histórico.
- A Alpha 8.1 não substitui restauração offline de backup nem validação manual Windows com **cópia** do banco Alpha 7.

## Testes adicionados

1. Upgrade v4 → v5 preservando nota/apresentação, tarefa, ideia e retomada; backup consistente.
2. Reinicialização idempotente sem duplicar histórico nem backups.
3. Trigger captura texto Unicode anterior a alterações via SQL; edição de texto idêntico não duplica snapshot.
4. Edição externa de ponto de retomada invalida a revisão CAS obsoleta.
5. Processamento de ideia antiga continua ambíguo e não é erroneamente classificado.
6. Descarte e conversão de ideias registram corretamente seu tipo e ID de destino.
7. Falha deliberada no registro da disposição reverte por completo o descarte/conversão.
8. Falha durante a criação do esquema v5 faz rollback das tabelas/versão.
9. Falha de criação de backup impede qualquer alteração de esquema v5.

CI esperada: `npm run check`, `npm run test:unit`, `npm run build`,
`cargo check --manifest-path src-tauri/Cargo.toml`,
`cargo test --manifest-path src-tauri/Cargo.toml --lib` e testes Python de restauração.

## Testes obrigatórios ainda não executáveis por CI

Windows real com banco antigo **copiado**, instalação e reinício, verificação de backup em disco,
parada forçada no meio do upgrade, nota aberta no Sticky, novo `Onde parei?`,
notificações e dados já capturados. Não executar sobre o único banco real do usuário.

**Não fazer merge na `main`** até aprovação explícita e testes manuais.
