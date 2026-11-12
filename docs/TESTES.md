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


## Alpha 3 — regressão de experiência
- [ ] Capturar ideia e tarefa diretamente em Agora, sem duplicação.
- [ ] Escolher tarefa em foco, reiniciar o app e confirmar preferência local.
- [ ] Editar ideia sem transformá-la em nota.
- [ ] Editar tarefa e verificar salvamento automático após fechar e abrir novamente.
- [ ] Texto não é perdido ao alternar item caso um salvamento falhe.
- [ ] Busca por "reuniao" encontra "Reunião"; filtros exibem apenas o tipo escolhido.
- [ ] Captura rápida, lembretes de notas e cartões flutuantes continuam funcionando.
- [ ] Notas antigas preservadas após instalação sobre a Alpha 2.

## Alpha 4 — planejamento temporal
- [ ] Testar "Revisar amanhã às 15h" e conferir a prévia.
- [ ] Datas inválidas e anos passados não devem ser reconhecidos.
- [ ] Criar tarefas para Depois e Algum dia sem ocupar o foco de Agora.
- [ ] Tarefas antigas permanecem em Hoje e notas não são alteradas.
- [ ] Definir data de retorno de Depois e verificar entrada em Hoje.
- [ ] Reiniciar e confirmar persistência do planejamento.
- [ ] Captura global em modo Tarefa preserva o rascunho em caso de erro.
- [ ] Prazos da tarefa não geram notificações nesta versão.

## Alpha 5 — Lembretes 2.0

- [ ] Migração a partir da Alpha 4.1 com backup pré-reminders, sem apagar dados.
- [ ] Aviso para nota e tarefa, com app na bandeja.
- [ ] Recuperação de aviso vencido com computador/app totalmente desligado.
- [ ] Adiamento, repetição, resolução e histórico persistem após reiniciar.
- [ ] Sem alertas duplicados após reiniciar sem ação do usuário.
- [ ] Lote de múltiplos avisos vencidos gera resumo e cada item fica visível.
- [ ] Tarefa concluída/deletada não notifica.
- [ ] Sem mensagens falsas de recebimento confirmado pelo Windows.
