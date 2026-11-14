# RECALL Alpha 10.3 — Navegação do Quadro

**Estado:** código experimental em PR separado. A Alpha 10.2 é a base; `main` permanece intacta.

## Funcionalidades

- Busca dentro de um quadro por notas livres, conteúdo de links e tabelas, nomes de imagens e notas/tarefas vinculadas.
- Normalização de acentos em português, resultado limitado a 40 cartões e rolagem no painel.
- Clique no resultado centraliza e seleciona o cartão, mantendo escala legível.
- Minimap local de todos os cartões, com indicadores de posição e retângulo da região visível.
- Enquadrar: fit-to-content para cartões em coordenadas positivas ou negativas.
- Memorizar zoom e centro do quadro individualmente usando localStorage (sem alterar o SQLite).
- Keyboard: F enquadra o quadro, Ctrl+F abre busca, Escape limpa busca/seleção (quando quadro está em foco).
- O fluxo existente de referências e conexões é preservado, sem nova dependência npm.

## Arquitetura

- `src/lib/boardNavigation.ts`: funções puras de geometria, filtro, foco e estado de viewport.
- `tests/boardNavigation.test.mjs`: teste Node para limites, zoom inválido, foco, persistência e pesquisa.
- `src/components/Whiteboard.svelte`: encaixe dos controles no módulo existente.
- Não há migração: o SQLite continua na versão 10. Dados de notas, tarefas, imagens e cartões são intocados.
- A viewport é salva opcionalmente por quadro em `recall-board-viewport-v1:<board_id>`.
- Ao mudar de quadro, gravações de cartões pendentes continuam bloqueando a troca em caso de falha.

## Validação automatizada e revisão pendente

1. npm ci; npm run check; npm run test:unit; restauração SQLite offline; frontend build.
2. cargo check e cargo test; build do instalador Windows x64 e hash SHA256.
3. Verificar busca de conteúdos, notas/tarefas vinculadas e exclusões.
4. Testar enquadramento com cartões em coordenadas negativas e muito distantes.
5. Testar clique no minimapa, redimensionamento da janela, voltar ao mesmo quadro e reabrir o app.
6. Verificar atalhos sem interceptar digitação em textarea/input e sem zoom do navegador.
7. Verificar múltiplos quadros, edições pendentes, conflitos de rascunhos e referências antigas.
8. Manual Windows/WebView2 continua pendente e **não é necessário ao usuário neste momento**.

## Limitações

- Busca local e minimapa processam os cartões já carregados; virtualização de milhares de cartões será uma fase posterior.
- Visualização persistida no localStorage deste dispositivo, não sincronizada entre equipamentos.
- Não inclui undo/redo nem exportação nesta versão para evitar alterações não testadas no conteúdo.
- Não realizar merge na main nem instalar sobre a única cópia real do banco.
