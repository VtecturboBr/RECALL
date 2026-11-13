# RECALL v0.5 Alpha 7 — Onde parei?

**Base:** `fix/alpha6-notificacoes-seguranca` (PR #13, sem merge na `main`).  
**Branch:** `feature/v0.5-alpha7-onde-parei`, experimental.  
**Missão:** registrar o mínimo necessário para retomar uma tarefa e uma apresentação sem depender da memória imediata.

## Experiência

### Tarefas em foco

Na tela **Agora**, a tarefa atual mantém seu destaque. Um botão secundário **Onde parei?** abre um editor compacto com dois campos:

- **O que eu estava fazendo?** — contexto breve do trabalho interrompido.
- **Qual é o próximo passo?** — ação concreta para continuar.

O painel é recolhível; se houver próximo passo salvo, ele aparece resumido no botão. O conteúdo é salvo automaticamente após 900 ms sem digitar e também ao sair dos campos ou clicar em **Salvar agora**. A edição é copiada imediatamente para armazenamento local antes do debounce para recuperar mudanças caso o aplicativo seja encerrado no meio do processo.

Nenhuma informação é enviada a serviços externos. O armazenamento persistente autoritativo é SQLite.

### Segurança e conflitos

- Tabela auxiliar aditiva `task_resumes` com `task_id`, contexto, passo, horário e revisão.
- Primeira gravação começa na revisão 1; atualizações só acontecem com revisão esperada. Duas janelas não podem substituir silenciosamente a mesma revisão.
- Uma leitura remota não substitui texto local sujo. Conflitos bloqueiam gravação automática e preservam o rascunho; a ação **Salvar cópia como ideia** escreve versão independente no Arquivo.
- Em banco existente com tabela `tasks` mas sem `task_resumes`, o RECALL exige snapshot SQLite `pre-resume` validado antes de iniciar a migração.
- `PRAGMA user_version=4`; sem exclusão de tabelas nem colunas legadas.

### Apresentações

A apresentação pode ser retomada do último cartão visitado. O progresso do cartão fica no armazenamento local do mesmo dispositivo e é limitado ao número de cartões disponível após edições. **Recomeçar** volta ao primeiro cartão. Não há nova tela, serviço externo ou alteração no corpo da nota.

## Como testar no Windows

1. Criar uma tarefa em Hoje e digitar contexto e próximo passo em **Onde parei?**. Conferir o salvamento automático.
2. Fechar o app rapidamente após digitar; reabrir e verificar recuperação do rascunho.
3. Trocar a tarefa em foco; confirmar checkpoints independentes entre tarefas.
4. Editar o mesmo checkpoint em duas instâncias; confirmar conflito e criação de cópia sem perder a primeira versão.
5. Abrir apresentação com três cartões; avançar ao segundo, fechar e reabrir; deve voltar ao segundo.
6. Excluir cartões da nota; a posição restaurada deve ser limitada ao último cartão válido.
7. Reiniciar o aplicativo com banco de teste da Alpha 6; conferir backup `pre-resume`, conteúdo anterior e ausência de perdas.
8. Testar campos com zoom de 150–200%, teclado e leitor de tela, caso disponível.
9. Confirmar que capture, sticky, notificações e backups existentes não regrediram.

## Testes automatizados adicionados

- Rust: CAS de checkpoint com gravações concorrentes; persistência após reabrir o banco; backup antes de migração.
- Node: interpretação segura do índice persistido de apresentação e limite quando diminui o número de cartões.
- Pipeline Windows: `npm run check`, `npm run test:unit`, `npm run build`, `cargo check`, `cargo test --lib` e 3 testes Python; instalador NSIS em artefato separado.

**Limitação:** CI não valida interação com um executável instalado no Windows real. Não promover a branch à `main` sem testes manuais e revisão.
