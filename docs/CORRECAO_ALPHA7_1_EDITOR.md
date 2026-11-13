# RECALL v0.5 Alpha 7.1 — Correção de navegação, exclusão e tarefas sem título

**Branch:** `fix/alpha7-editor-desbloqueio`, baseada na Alpha 7 experimental (`feature/v0.5-alpha7-onde-parei`).  
**Status:** hotfix em PR próprio, sem merge na `main` e **sem incluir a migração v5 da Alpha 8.1**.  
**Relato em 13/11/2026:** tela Arquivo fica presa em ideia cujo editor mostra um campo em branco, aviso "A ideia não pode ficar vazia", status "Alterações pendentes". Cliques em outros itens não funcionam. Exclusão e criação de tarefas sem título também apresentam problemas.

## Causas verificadas
1. `saveIdea` recusava texto vazio, mas `openEntry` chamava `flushDraft` e cancelava toda navegação quando o resultado era falso. Assim a pessoa não conseguia sair do item.
2. O cache `recall-idea-draft:<id>` restaurava inclusive um rascunho **inteiramente vazio**, escondendo o texto não vazio persistido no SQLite.
3. `deleteSelected` exigia salvar o item **antes** de excluí-lo. Qualquer edição inválida ou conflito impedia a exclusão.
4. `saveTask` não aceitava título vazio e `captureInline` desativava o botão de tarefa quando não havia texto.
5. As exclusões de notas e tarefas gravavam o tombstone e o cancelamento de lembrete em **transações separadas**, deixando risco de operação parcial.

## Alterações

- Rascunho em branco de ideia não substitui conteúdo persistido quando reabre o editor; cache em branco é ignorado.
- Se a pessoa tenta navegar com uma ideia editada totalmente vazia, o app **cancela só a alteração inválida**, mantém intacta a ideia anteriormente salva no SQLite e permite a navegação. Não aceita gravar a ideia vazia em SQLite.
- O editor informa claramente que o campo vazio não será salvo, sem prender a navegação.
- A exclusão exige confirmação, mas **não depende de autosave bem-sucedido**. Para evitar perder trabalho sem aviso, o diálogo explicita que alterações ainda não salvas serão descartadas. O cache do item só é removido quando o backend confirma que foi excluído; falhas preservam o rascunho.
- Notas e tarefas usam `deleted_at` (soft delete) como antes. Excluir + cancelar lembrete fazem commit juntos, com rollback completo se houver falha.
- Nova tarefa pode ser criada sem digitar nada; nesse caso recebe o título `Nova tarefa`. Tarefas já existentes cujo título seja apagado recebem o mesmo título provisório ao salvar.
- Botão `Nova tarefa` acrescentado ao rodapé do Arquivo, além da captura rápida de tarefas.
- Não modifica identificador do aplicativo, caminho dos bancos, configurações de lembretes ou arquivos da Alpha 8.

## Plano de teste

1. Em uma ideia com texto salvo, selecionar todo o texto, apagar e clicar em outra nota; a navegação deve funcionar e o texto anterior permanecer no banco.
2. Reabrir uma ideia que tenha cache local em branco da Alpha 7; deve exibir o texto realmente salvo.
3. Com uma ideia em branco no editor, clicar em **Descartar ideia**, confirmar e verificar que desaparece da lista.
4. Selecionar uma nota, alterar título e excluir; se aceitar a confirmação, ela deve ser soft-deletada, sem travar.
5. Criar tarefa pela captura com campo em branco; deve aparecer como `Nova tarefa`.
6. Criar tarefa pela ação `Nova tarefa` no Arquivo; deve abrir o título para edição.
7. Apagar o título de uma tarefa antiga e sair; seu título deve virar `Nova tarefa`, sem perder os detalhes.
8. Simular falha SQL ao cancelar lembrete durante exclusão da nota e tarefa; nada deve ser excluído parcialmente.
9. Testar conflito de nota/ideia de duas janelas, recuperação após reinício e criação de cópia sem regressão.
10. Confirmar que o instalador Alpha 7.1 preserva o banco v4 e o backup do usuário.

## Testes automatizados
- Node: normalização de título provisório, detecção de ideia vazia e política de restauração do cache.
- Rust: rollback atômico ao falhar cancelamento de lembrete durante exclusão de nota e tarefa.
- CI Windows: `npm run check`, `npm run test:unit`, 3 testes Python, `npm run build`, `cargo check`, `cargo test --lib`; build NSIS e checksum SHA-256 em artefato separado.

**Limitações:** a CI valida a compilação e a lógica do SQLite, mas a reprodução e o comportamento gráfico devem ser confirmados na instalação Windows real. Esta é uma correção de Alpha 7; **não fazer merge na `main`** sem autorização explícita.
