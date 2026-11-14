# RECALL Alpha 10.1 — Múltiplos quadros e conexões

Estado: PR draft, separado da Alpha 10.0 (PR #24), sem merge na main.

## Funcionalidade

- Criar quantos quadros forem necessários, com nomes de 1 a 80 caracteres; renomear sem perder conteúdo.
- Trocar de quadro apenas depois de escoar as gravações pendentes. Se houver erro, impedir a troca e preservar o rascunho.
- Conectar dois cartões ativos no mesmo quadro usando os botões no cabeçalho; desenhar seta SVG de centro a centro.
- Impedir auto-conexões, conexões entre quadros ou para cartão removido; criação idempotente.
- Listar e remover as conexões do cartão selecionado.
- Quando um cartão é removido logicamente, suas conexões não aparecem; os dados originais permanecem no SQLite.

## Modelo e migração

SQLite user_version 8 -> 9, transação única:
- `whiteboards`: ID, nome, datas.
- `whiteboard_items.board_id`: coluna aditiva `TEXT NOT NULL DEFAULT 'default'`, preservando IDs, revisões, conteúdo e assets.
- `whiteboard_connections`: origens/destinos, ID, índice e UNIQUE por direção.
- O quadro inicial é `default` — Meu quadro. A versão anterior do RECALL só tinha um quadro, que se torna o original.
- Cria backup SQLite verificado `pre-alpha10-1-boards` antes da migração 8->9, caso existam dados.
- Restauração offline aceita no máximo a versão 9 e verifica as novas tabelas.

## Testes de regressão

1. Abrir Alpha 10.0 com cartões e imagens e atualizar para Alpha 10.1 em um **perfil Windows de teste**. Confirmar migração, assets e backup.
2. Criar dois quadros, adicionar várias notas em cada, trocar e confirmar isolamento/reinício.
3. Criar e remover setas; arrastar cartões e confirmar que as pontas acompanham o movimento.
4. Impedir duplicatas, auto-conexão e conexões entre quadros.
5. Forçar falha de salvamento e trocar de quadro; a troca deve ser bloqueada, com rascunho preservado.
6. Validar CI (Node/Svelte/Rust/Python), instalador NSIS, hash SHA256 e testes manuais WebView2.

## Limitações

- Conexões são apenas setas retas, sem roteamento ortogonal, labels ou agrupamento.
- Quadros não podem ser excluídos nesta versão; medida conservadora para evitar perda acidental.
- Links para notas/tarefas existentes, minimapa, exportação, undo/redo e recuperação visual de conflitos continuam pendentes.
- Banco SQLite v9 não pode ser aberto diretamente por versões antigas; faça backup e não instale sobre a única cópia pessoal.
