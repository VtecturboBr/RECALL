# RECALL Alpha 10.2 — Referências dinâmicas ao Arquivo

**Estado:** Draft PR separado, dependente da Alpha 10.1 (PR #25); nenhuma alteração na main.

## Motivação

Cartões do tipo Do Arquivo não duplicam o texto das notas ou tarefas.
O registro guarda apenas item_id, target_type e target_id. O título,
prévia e status vêm diretamente das tabelas autoritativas do SQLite.
Um item excluído aparece indisponível sem exibir texto privado antigo.

## Arquitetura

- Backend: src-tauri/src/board_references.rs; migração aditiva v10.
- Tabela whiteboard_references guarda o ID do cartão e da origem.
- A criação do cartão e da referência ocorre na mesma transação.
- O kind do cartão continua note: sem reconstruir CHECK ou tabela de imagens.
- Busca no Arquivo parametrizada com limite de 40 resultados.
- UI: src/components/Whiteboard.svelte com modal de busca e cartão de leitura.
- Atualização por eventos notes:changed e tasks:changed sem sobrescrever cartões editáveis.
- Abrir no Arquivo usa o editor existente do RECALL.
- Backup verificado pre-alpha10-2-references antes da migração v9 -> v10.
- Restaurador offline permite v10 e verifica a nova tabela.

## Casos de teste

1. Validar PR #25 e CI do próprio PR: Node, Svelte, Python, Rust, NSIS.
2. Criar nota e tarefa, vinculá-las, reiniciar e verificar IDs mantidos.
3. Atualizar títulos e descrições no Arquivo, conferir atualização dos cartões.
4. Concluir tarefa e verificar indicador concluída.
5. Excluir nota de teste e confirmar indisponibilidade sem prévia privada.
6. Conectar referências a outros cartões, mover/redimensionar, alternar quadros.
7. Simular erro na criação: a transação não pode deixar cartão órfão.
8. Validar migração v9 -> v10 com cópia do banco e snapshot verificado.

## Limitações desta Alpha

- Referências somente para notas e tarefas. Inclusão de ideias é futura.
- Busca limitada a 40 itens, sem paginação global.
- Atualizações externas ao processo RECALL podem exigir reabrir a tela.
- Conteúdo vinculado é editado no Arquivo, não diretamente no Quadro.
- Banco v10 incompatível com builds anteriores: nunca usar a única cópia pessoal.
- Teste manual Windows/WebView2 ainda é obrigatório.
