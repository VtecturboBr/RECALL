# RECALL

**Sua memória externa para Windows:** notas, tarefas, ideias, lembretes locais, apresentações, sessões de foco e revisão diária.

> **Versão experimental mais recente desta branch:** `0.5.0-alpha.9.2.1` — correções de estabilidade derivadas da Alpha 9.2. **Não é versão estável nem foi validada visualmente em Windows real.** Não há sincronização Android nem contas remotas.

## O que o aplicativo oferece

- **Agora:** tarefas do dia, próxima ação, captura inline, Modo Foco 2.0 e Revisão Diária manual.
- **Arquivo:** notas, apresentações, tarefas e ideias com busca, associação de informações e edição automática.
- **Onde parei?**: checkpoint e próximo passo por tarefa.
- **Captura Rápida:** janela global `Ctrl+Shift+Espaço` enquanto o RECALL está em execução, inclusive na bandeja; cria ideias e tarefas.
- **Lembretes 2.0:** notificações genéricas (sem título privado na tela bloqueada), recorrência, adiamento e recuperação de lembretes vencidos.
- **Armazenamento local:** SQLite com WAL, revisões concorrentes para dados textuais, exclusão lógica, históricos internos e snapshots verificados.

## Alpha 9.2.1 — estabilidade

Esta atualização corrige erros encontrados na auditoria da Alpha 9.2:

- Captura Rápida não apaga texto novo digitado durante salvamento e aceita tarefas sem título (`Nova tarefa`).
- Heartbeat de foco no Rust independente da tela, com controle global para sessões em qualquer categoria.
- Resumos de foco em edição preservados ao concluir/excluir uma tarefa.
- Conversão de ideias preserva associações; consulta de relações filtra itens excluídos antes de limitar resultados.
- Revisão Diária recupera próximos destaques depois de ocultar os primeiros 12.
- Backups são possíveis mesmo em conflito de rascunho (alerta explicita o que fica fora do SQLite).
- Restauração offline usa uma substituição do banco depois de snapshot verificado e valida versões/esquemas.

**Plano de testes e limitações:** [Alpha 9.2.1 — Estabilidade](docs/ALPHA9_2_1_ESTABILIDADE.md).  
**Auditoria de origem:** RECALL Alpha 9.2 (relatório disponibilizado na conversa de auditoria).

## Instalação no Windows x64

1. Feche o RECALL completamente pelo ícone da bandeja do sistema.
2. Faça uma cópia integral da pasta `%APPDATA%\app.memoriaexterna.desktop`. Para testar migrações, prefira uma **conta Windows de teste** e uma **cópia** dos dados, não o banco principal.
3. Abra [GitHub Actions do RECALL](https://github.com/VtecturboBr/RECALL/actions/workflows/windows-installer.yml).
4. Selecione o run da branch `fix/alpha9-2-1-estabilidade-integridade` com **Success**, em **Artifacts** baixe `RECALL-v0.5-alpha9-2-1-estabilidade-Windows-x64`.
5. Extraia o ZIP, compare o SHA-256 do `.exe` com `SHA256SUMS.txt` e instale. Consulte [guia do instalador](docs/INSTALADOR_WINDOWS.md).

O instalador é prévia **não assinada digitalmente**. Nenhum merge automático é feito na `main`.

## Executar o código localmente

Requer Node.js LTS, Rust, MSVC Build Tools, WebView2 e os pré-requisitos do [Tauri 2 para Windows](https://v2.tauri.app/start/prerequisites/).

```powershell
npm install
npm run check
npm run test:unit
python -m unittest discover -s scripts -p "test_*.py"
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run desktop
```

Para gerar o instalador: `npm run installer -- --bundles nsis`.

## Organização do código

| Pasta | Responsabilidade |
|---|---|
| `src/components/RecallStudio.svelte` | Tela principal Agora/Arquivo |
| `src/components/QuickCapture.svelte` | Janela de captura |
| `src/components/Sticky.svelte` | Notas flutuantes |
| `src/components/FocusSession.svelte`, `ActiveFocusBar.svelte` | Foco, resumo e controle global |
| `src/components/LinkedItems.svelte`, `DailyReview.svelte` | Conexões e revisão |
| `src/lib/` | Contratos Tauri, validação e helpers |
| `src-tauri/src/` | Comandos Tauri, SQLite, backups, migrações e lembretes |
| `scripts/restore_backup.py` | Restauração offline, exige app fechado |
| `tests/` e `scripts/test_restore_backup.py` | Regressões Node/Python |
| `docs/` | Planos, histórico, decisões e roteiros de teste |

Os arquivos `src/components/Main.svelte`, `Simplified.svelte`, `Today.svelte`, `Inbox.svelte` pertencem a implementações antigas e **não são montados por `src/App.svelte`**. Permanecem no repositório para referência até uma limpeza posterior com teste visual.

## Dados e segurança

Não envie bases SQLite, snapshots ou conteúdos pessoais para commits. O `.gitignore` contém padrões para os formatos locais. A aplicação é offline, mas **lembretes dependem do processo RECALL estar em execução** (o processo pode ficar na bandeja). Durante desligamento do computador, não há notificações nativas; o RECALL recupera pendências ao abrir.

Use [AGENTS.md](AGENTS.md), [AI Workflow](docs/AI_WORKFLOW.md) e [checkpoint](docs/AI_CHECKPOINT.md) para contexto do projeto. Os branches e PRs experimentais são separados; **nenhuma etapa deve ser integrada à `main` sem autorização explícita**.
