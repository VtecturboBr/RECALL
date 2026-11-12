# Testes e verificações da v0.3

Executados na prévia HTML usando Chromium sem servidor externo:

- Carregamento sem erros de JavaScript e tela inicial vazia.
- Criação, edição e persistência no `localStorage`.
- Fixação, navegação entre categorias e pesquisa.
- Cartões flutuantes internos à prévia.
- Modo de foco com edição.
- Apresentação com múltiplos cartões e navegação.

Não testados neste ambiente:

- `npm run check` / compilação Svelte, indisponíveis porque `npm install` não pôde acessar o registro de pacotes.
- Compilação Rust e integração Windows Tauri.
- Notificações nativas e janelas independentes no sistema operacional.

## v0.4 — captura rápida
- [ ] Atalho Ctrl+Shift+Espaço abre o painel com janela principal minimizada na bandeja.
- [ ] Ctrl+Enter persiste uma nota e fecha o painel; Enter sozinho cria nova linha.
- [ ] Esc fecha sem perder o rascunho no painel (enquanto o app continuar executando).
- [ ] A nota aparece na janela principal sem necessidade de reiniciar.
- [ ] Falha de gravação exibe erro e preserva o texto.
- [ ] Quando atalho está ocupado, o aplicativo continua iniciando, e botão Capturar funciona.
- [ ] Nenhum dado pré-existente da v0.3 é modificado ou perdido.


## v0.5 alpha — testes manuais necessários

- [ ] Dados das notas da v0.4 preservados após inicialização.
- [ ] Criar, editar, concluir, reabrir e excluir tarefas na tela Hoje.
- [ ] Reabrir o aplicativo e verificar persistência das tarefas.
- [ ] Capturar uma ideia e confirmar que aparece na Entrada.
- [ ] Converter ideia em nota ou tarefa e verificar que sai da Entrada.
- [ ] Confirmar que os cartões flutuantes e lembretes antigos continuam funcionando.
- [ ] Testar instalação e bandeja no Windows real.

Obs.: o campo de data de tarefas ainda não dispara notificações. A prévia HTML permanece na v0.4.
