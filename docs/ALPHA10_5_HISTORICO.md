# RECALL Alpha 10.5 — Histórico seguro do Quadro

**Status:** PR de desenvolvimento separado da Alpha 10.4, sem merge na `main`.

## Escopo

- Desfazer e refazer alterações de conteúdo, posição e tamanho de cartões do Quadro.
- Reconhece alterações confirmadas, após gravação SQLite, com ações antes/depois.
- Histórico limitado a 80 operações por quadro **na sessão atual**; sem nova migração.
- Ctrl+Z = Desfazer; Ctrl+Y ou Ctrl+Shift+Z = Refazer quando o fundo do quadro estiver em foco.
- Não intercepta atalhos do editor de texto: o navegador mantém o desfazer nativo em textareas/inputs.

## Segurança

- A fonte de verdade é SQLite. A UI mantém cópias das últimas revisões confirmadas.
- Antes de qualquer operação no histórico, esgota os rascunhos pendentes via `finishPending`.
- Se o flush falhar, desfazer/refazer não prossegue e o rascunho local permanece preservado.
- A operação envia `expectedRevision` da última confirmação local e usa CAS no backend Rust.
- Se outra janela tiver alterado o cartão, CAS bloqueia; não repete com revisão nova automaticamente.
- Em caso de conflito, o histórico local é invalidado; reabrir o quadro atualizará as revisões.
- Só move o cursor de histórico após resposta bem-sucedida do SQLite.
- As ações não criam/excluem cartões, não recriam imagens e não alteram referências dinâmicas.
- Novas edições confirmadas limpam a pilha de refazer; exclusão de cartão limpa seu histórico.
- Ao trocar de quadro, o histórico é reiniciado para não misturar ações de quadros distintos.
- O histórico não é persistido no banco: evita novas tabelas e implicações na retenção de dados.

## Arquivos

- `src/lib/boardHistory.ts`: funções puras de snapshot, limites, empilhamento e segurança.
- `tests/boardHistory.test.mjs`: testes de edit/move/resize, ordem de ações, redo e referências obsoletas.
- `src/components/Whiteboard.svelte`: interface/atalhos e integração com a fila de autosave.
- `src-tauri/src/whiteboard.rs`: teste de regressão de CAS com duas janelas, sem alterar lógica SQL.

## CI e aceitação

1. `npm ci`, Svelte/TypeScript check, Node tests, build web, restaurador Python.
2. `cargo check`, `cargo test`, geração NSIS, hash SHA256 do instalador.
3. Digitar em nota, aguardar salvar, Desfazer e Refazer; repetir com movimentação e redimensionamento.
4. Alterar diferentes cartões, usar Undo/Redo na ordem inversa e após novas edições.
5. Editar um cartão em outra janela e garantir conflito CAS sem sobrescrever a versão remota.
6. Forçar falha SQL durante autosave e confirmar rascunho preservado, sem aplicar undo.
7. Reiniciar ou trocar de quadro, confirmar histórico vazio e dados SQLite intactos.
8. Verificar cartões vinculados ao Arquivo, imagens e tabelas sem alterar o conteúdo original.
9. Teste visual manual Windows/WebView2 adiado conforme disponibilidade do usuário.

## Fora de escopo

- Não desfaz criação, exclusão, conexão ou múltiplas ações em uma transação composta.
- Não oferece histórico persistido entre sessões; isso exigiria projeto de log transacional.
- Não pode garantir que uma alteração jamais será perdida fora das condições de falha cobertas por backups e rascunhos.
