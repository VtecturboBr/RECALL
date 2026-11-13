# RECALL — sua memória externa

Aplicativo **offline para Windows** com notas, tarefas, apresentações, ideias, lembretes, sessões de foco e pontos de retomada. Construído com **Svelte 5, TypeScript, Tauri 2, Rust e SQLite**.

> **Estado atual:** versão de testes **0.5.0-alpha.9.2.1** na branch `fix/alpha9-2-1-integridade-estabilidade` (PR #21). A `main` ainda não recebeu essa Alpha. **Não use um banco real sem cópia de segurança**; teste em um usuário Windows separado, se possível. Não é uma versão estável.

## Funcionalidades nesta Alpha

- **Agora:** tarefas de hoje, Captura, Modo Foco 2.0 e Revisão Diária sob demanda.
- **Arquivo:** notas, apresentações, ideias, tarefas, busca e organização por período.
- **Onde parei?:** contexto e próximo passo por tarefa, com controle de revisões e rascunhos.
- **Informações conectadas:** links bidirecionais entre notas, tarefas e ideias.
- **Lembretes 2.0:** recorrência, adiamento, recuperação de vencidos e histórico de eventos.
- **Notas flutuantes e Captura Rápida:** acesso no Windows, inclusive pela bandeja/atalho quando disponível.
- **Proteção de dados:** SQLite WAL, transações, exclusão lógica, backups verificados e detecção de conflitos entre editores.

As notificações dependem do RECALL em execução. Ao desligar o PC ou encerrar o processo, nenhuma notificação nativa é enviada durante esse período; os lembretes vencidos voltam para a central de atenção ao abrir o app.

## Alpha 9.2.1 — estabilidade

Corrige problemas identificados na auditoria da Alpha 9.2, principalmente:
- Texto digitado durante a Captura Rápida não é apagado por um salvamento anterior; tarefas sem título recebem `Nova tarefa`.
- Modo Foco possui heartbeat nativo independente da aba e controle de sessão ativa mesmo para tarefas fora de Hoje.
- Concluir ou excluir uma tarefa com sessão em andamento preserva o resumo de foco dentro da mesma transação.
- A Revisão Diária carrega mais candidatos ao ocultar sugestões.
- Links consultam itens válidos **antes** do limite e preservam relações quando uma ideia é convertida.
- Backups podem ser gerados em conflito editorial, deixando explícito que rascunhos não salvos não fazem parte do backup SQLite.
- O restaurador valida estrutura e versão, detecta interrupção e evita criação silenciosa de um banco vazio.
- Rascunhos de janelas concorrentes recebem proteção contra exclusão acidental.

Veja [a auditoria original](docs/ALPHA9_2_REVISAO_DIARIA.md) e [a documentação da correção](docs/ALPHA9_2_1_ESTABILIDADE.md).

## Baixar e testar no Windows

1. Feche completamente o RECALL pelo menu da bandeja.
2. Faça uma **cópia integral** de `%APPDATA%\app.memoriaexterna.desktop`, incluindo `memoria.sqlite`, backups e eventuais arquivos `-wal`/`-shm`. Nunca apague WAL/SHM manualmente.
3. No [GitHub Actions](https://github.com/VtecturboBr/RECALL/actions), abra a execução **Gerar instalador RECALL Windows** da branch `fix/alpha9-2-1-integridade-estabilidade`.
4. Quando o workflow indicar **Success**, baixe o ZIP `RECALL-v0.5-alpha9-2-1-estabilidade-Windows-x64`; confirme o `SHA256SUMS.txt` e instale o `.exe` NSIS.
5. Teste todas as funcionalidades em um **banco de teste**, não no único original.

A versão Alpha 9.2.1 inclui as Alphas anteriores da sequência. Você não precisa reinstalar a 7/8/9.0/9.1 para testar. Como o banco está no schema SQLite v7, **não tente abri-lo em builds antigas**.

## Desenvolver

Pré-requisitos: Node.js 22, Rust estável, MSVC Build Tools e WebView2 no Windows ([guia do Tauri 2](https://v2.tauri.app/start/prerequisites/)).

```powershell
npm install
npm run check
npm run test:unit
npm run desktop
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

- Interface: `src/components/RecallStudio.svelte`, `src/components/FocusSession.svelte`, `src/components/DailyReview.svelte`.
- Dados/IPC: `src-tauri/src/db.rs`, `tasks.rs`, `focus.rs`, `links.rs`, `reminders.rs`, `review.rs`, `lib.rs`.
- Testes: `tests/`, `scripts/test_restore_backup.py` e testes Rust nos módulos.
- Instalador: `.github/workflows/windows-installer.yml`.
- Planejamento e histórico: `docs/`, `AGENTS.md`, `config/ai-workflow.yaml`.

Componentes antigos `Main.svelte`, `Simplified.svelte`, `Today.svelte` e `Inbox.svelte` permanecem como legado não montado. Não remover folhas de estilo sem checagem visual do instalador.

## Limitações e segurança

Ainda **não há** sincronização Android, conta na nuvem, IA paga, lixeira visual ou restauração de histórico pelo aplicativo. O backup manual protege **somente alterações já persistidas no SQLite**; cópias locais de rascunhos exigem recuperação separada. Não publique bancos, backups nem dados pessoais no GitHub.

**Fluxo GitHub:** as funcionalidades são implementadas em branches e PRs empilhados. Não realizar merge na `main` sem aprovação explícita.
