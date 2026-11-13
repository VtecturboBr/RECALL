# RECALL Alpha 9.1 — Contexto conectado

**Versão:** `0.5.0-alpha.9.1` (prévia Windows x64).  
**Branch:** `feature/v0.5-alpha9-1-contexto-conectado`.  
**Base:** Alpha 9.0 `feature/v0.5-alpha9-focus-2` (PR #18).  
**Objetivo:** notas, tarefas, ideias e apresentações (armazenadas como notas) podem fazer referência umas às outras sem criar um sistema complexo de projetos.

## Entrega

- **Informações conectadas:** painel secundário e recolhível em termos de densidade visual, localizado abaixo do editor no **Arquivo**.
- Escolher outro item ativo e clicar em **Conectar** cria vínculo bidirecional, sem duplicatas.
- Clique na informação relacionada para abrir o conteúdo original no Arquivo. A navegação respeita as proteções de rascunho do editor; se a gravação estiver bloqueada por conflito, o aplicativo não troca silenciosamente de item.
- Desconectar remove **somente a relação**, nunca a nota, tarefa ou ideia em si.
- Relações com itens excluídos logicamente ou ideias convertidas/processadas ficam guardadas no SQLite, mas ocultas enquanto o item não estiver ativo. Isso preserva o contexto para a futura Lixeira (Alpha 8.2).
- Nada de nuvem, tag automática, redesenho visual ou contas.
- Limite de **50 conexões exibidas por item** (evita payload excessivo na interface).
- Mudanças se propagam entre janelas por evento `links:changed` e consultas atualizadas.

## Banco SQLite v7

`item_links`: `id`, `left_type`, `left_id`, `right_type`, `right_id`, `created_at`.

As duas extremidades possuem `type` em `note|task|idea` e ID já existente. O par é canonizado por `(tipo,id)` e recebe restrição `UNIQUE` para impedir duplicatas em orientações opostas. Uma conexão com o próprio item é rejeitada. Como itens pertencem a **tabelas diferentes**, os relacionamentos fazem validação explícita de existência na mesma transação, em vez de FKs polimórficas irreais.

**Migração v6 → v7:** qualquer banco existente com versão anterior a v7 exige um snapshot verificado `pre-alpha9-links` antes de mexer no esquema. Criação da tabela, dos índices e `PRAGMA user_version=7` ocorrem na **mesma transação SQLite**. Falhas fazem rollback; o aplicativo recusa um esquema marcado v7 mas incompleto.

A migração de v5→v6 (Modo Foco 2.0) e a de v4→v5 (histórico e correções da Alpha 8.1) continuam disponíveis para quem atualiza diretamente de uma versão mais antiga. Não há exclusão automática de dados do usuário.

## Arquivos

- `src-tauri/src/links.rs`: banco, validações, migração, busca, criação e remoção de vínculos.
- `src-tauri/src/db.rs`: backup obrigatório e abertura da v7.
- `src-tauri/src/lib.rs`: comandos IPC `list_item_links`, `add_item_link`, `remove_item_link`.
- `src/lib/types.ts`, `src/lib/api.ts`: contratos com o frontend.
- `src/lib/related.ts`: candidatos ativos e exclusão de itens já vinculados.
- `src/components/LinkedItems.svelte`: cartões de contexto, lista e associação.
- `src/components/RecallStudio.svelte`, `src/recall.css`: integração discreta ao Arquivo.

## Testes automatizados

- Adicionar o mesmo vínculo em ordem invertida não cria duplicata.
- Conectar um item a si mesmo é rejeitado.
- Remover vínculo não remove conteúdo.
- Excluir nota ou processar ideia oculta a relação, **sem apagar a associação do banco**.
- Rejeitar tipo inválido ou destino inexistente.
- Atualizar cópia v6 para v7 preserva tarefas e gera backup `pre-alpha9-links` validado.
- Erro SQL simulado durante migração não muda a versão nem perde o conteúdo.
- Teste Node para lista de candidatos sem vínculos duplicados.
- Regressões anteriores: foco, lembretes, backup, migração v4→v5→v6, correções do editor, notas e tarefas.

## Roteiro de teste único Windows

1. Fechar **completamente** o RECALL (inclusive na bandeja ao lado do relógio).
2. Copiar a pasta **`%APPDATA%\app.memoriaexterna.desktop`** inteira, incluindo `-wal`/`-shm` se existirem, para outro local. Preferir testar em usuário Windows separado com **cópia** do banco.
3. Em GitHub Actions, abrir o workflow **Gerar instalador RECALL Windows** da branch `feature/v0.5-alpha9-1-contexto-conectado`, aguardar **Success**, baixar o artefato `RECALL-v0.5-alpha9-1-contexto-conectado-Windows-x64`.
4. Extrair ZIP, comparar SHA-256 do executável com `SHA256SUMS.txt` e executar o setup.
5. Confirmar que as informações da Alpha 9.0, sessões de foco, notas, ideias, lembretes, tarefas e backups continuam intactos. Um backup pré-v7 deve ter sido criado.
6. No Arquivo, abrir uma nota, conectar uma tarefa, conferir que aparece também **ao abrir a tarefa**. Abrir pelo cartão associado e voltar.
7. Conectar uma ideia, tentar duplicar a mesma conexão e confirmar que só há uma. Desconectar e confirmar que os dois itens permanecem.
8. Em item de teste: excluir a nota vinculada e verificar que a ligação some da lista de ativos sem apagar o conteúdo da tarefa.
9. Testar com duas janelas: vínculo criado em uma deve aparecer na outra após atualização.
10. Repetir os bugs anteriores: sair de ideia com editor temporariamente vazio, excluir nota, criar tarefa sem nome, pausar/retomar sessão, apresentação e Quick Capture.
11. Reiniciar o RECALL; verificar persistência e ausência de novo backup de migração v7.

## Fora desta fase

- Projetos ou pastas aninhadas, estrutura hierárquica, notas embutidas automaticamente e grafo visual de relacionamentos.
- Sugestões de relações por IA e indexação remota.
- Lixeira visual Alpha 8.2 e histórico restaurável Alpha 8.3 ainda pendentes.
- Revisão diária Alpha 9.2 e captura global Alpha 9.3 ainda pendentes.

**Não fazer merge na `main`; a validação Windows real ainda é obrigatória antes de considerar a fase estável.**
