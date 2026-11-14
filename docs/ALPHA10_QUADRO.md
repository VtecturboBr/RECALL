# RECALL Alpha 10 — Quadro Infinito (primeira implementação)

**Estado:** Alpha experimental, branch `feature/v0.5-alpha10-quadro-infinito`, sem merge na main.

## Objetivo

Adicionar uma terceira seção **Quadro**, separada das telas Agora e Arquivo.
A experiência utiliza um mundo de coordenadas livre, com aproximação e movimentação
do fundo, cartões independentes, edição direta, imagens e armazenamento durável.
O RECALL segue local e offline.

## Funcionalidades desta entrega

- Um quadro único inicialmente, com superfície conceitualmente infinita,
  pan por arrasto do fundo, zoom pela roda do mouse e botões.
- Nota: textarea livre com salvamento automático e rascunho de recuperação.
- Link: edição de URL e abertura HTTP/HTTPS com rel=noopener noreferrer.
  O conteúdo não é carregado em iframe nem executado como HTML.
- Tabela: células editáveis, expansão até 15 linhas e 8 colunas, colagem TSV.
- Imagem: seletor PNG/JPEG/WebP/GIF, Ctrl+V e tentativa de soltar arquivo;
  limite 4,5 MB por arquivo; sem SVG nem remote fetch.
- Arrastar e redimensionar cartões; exclusão lógica; armazenamento SQLite,
  persistência de posição, dimensões e conteúdo por revisão CAS.

## Arquitetura

- Frontend: `src/components/Whiteboard.svelte` (Svelte 5) em um módulo isolado;
  nenhuma dependência npm nova; estilos escopados e alinhados ao RECALL.
- Backend: `src-tauri/src/whiteboard.rs` com comandos Tauri por cartão;
  entrada validada e geometria finita.
- SQLite v8: `whiteboard_items` e `whiteboard_assets` em migração transacional
  aditiva. Backup verificado `pre-alpha10-board` precede a migração
  quando há conteúdo pré-existente.
- As imagens são armazenadas como Data URLs apenas na tabela de assets,
  e não são transmitidas em `list_board_items`. Asset carregado
  separadamente por ID, sem serviços remotos.
- `revision` por cartão previne sobrescrita silenciosa. Rascunhos são guardados
  no localStorage até confirmação do SQLite. Em CAS conflitante, preservamos
  o rascunho local e interrompemos a gravação.

## Validação obrigatória

1. Rodar CI Windows: npm ci, Svelte/TS check, testes Node, Python restore,
   Rust cargo check/test e compilação do frontend.
2. Compilar instalador NSIS e publicar SHA256SUMS.
3. Instalar primeiro em perfil de teste do Windows com cópia integral de dados.
4. Testar migração 7 -> 8, abertura posterior, sobrevivência de notas,
   tarefas, ideias, lembretes, sessões de foco e backups existentes.
5. Colar texto, link, tabela TSV e imagem; reiniciar e confirmar persistência.
6. Mover/redimensionar cada tipo de cartão e confirmar o estado após reiniciar.
7. Testar imagem inválida, arquivo >4,5 MB e URL javascript: (não abrir).
8. Simular erro SQLite e edição simultânea; nenhum rascunho pode ser apagado.
9. Testar acessibilidade, zoom 80%-200%, tela mínima 800x560 e WebView2.

## Próximas etapas

- Referências dinâmicas às notas, tarefas e ideias do Arquivo, sem duplicação.
- Conexões visuais e setas; blocos agrupáveis e frames.
- Múltiplos quadros, pesquisa, minimapa, atalhos e undo/redo.
- Otimização por viewport/virtualização para milhares de objetos.
- Acessibilidade de arrasto por teclado e recuperação visual de conflitos.
- Exportação/importação e versões de quadro.

**Segurança:** não fazer merge na main, não instalar sobre o único banco real.
Bancos SQLite v8 não são compatíveis com versões anteriores do RECALL.
