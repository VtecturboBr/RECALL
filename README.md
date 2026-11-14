# RECALL

**Sua memória externa para Windows:** notas, tarefas, ideias, lembretes locais, apresentações, foco, revisão diária e o novo Quadro Infinito.

> **Versão experimental desta branch:** `0.5.0-alpha.10.3` — navegação do Quadro com minimapa, pesquisa e enquadramento. CI e teste visual no Windows continuam obrigatórios. **Não é uma versão estável.** Sem sincronização Android.

## O que o aplicativo oferece

- **Agora:** tarefas do dia, próxima ação, captura inline, Modo Foco 2.0 e Revisão Diária manual.
- **Arquivo:** notas, apresentações, tarefas e ideias com busca, associação de informações e edição automática.
- **Quadro (Alpha 10):** tela visual com pan/zoom, notas, imagens PNG/JPEG/WebP/GIF, links HTTP(S) e tabelas editáveis; cartões persistidos no SQLite com CAS.
- **Onde parei?**: checkpoint e próximo passo por tarefa.
- **Captura Rápida:** janela global `Ctrl+Shift+Espaço` enquanto o RECALL está em execução, inclusive na bandeja; cria ideias e tarefas.
- **Lembretes 2.0:** notificações genéricas (sem título privado na tela bloqueada), recorrência, adiamento e recuperação de lembretes vencidos.
- **Armazenamento local:** SQLite com WAL, revisões concorrentes para dados textuais, exclusão lógica, históricos internos e snapshots verificados.

## Alpha 10.3 — Navegação e descoberta do Quadro

- **Encontrar:** pesquisa local por texto dos cartões e pelas informações atuais das notas/tarefas vinculadas, incluindo acentos.
- **Minimapa:** visão reduzida dos cartões; clique em um cartão para centralizá-lo, e veja qual região está visível.
- **Enquadrar:** ajuste automático do zoom para a área ocupada pelos cartões.
- **Retomar a posição:** a câmera e o zoom ficam preservados por quadro no armazenamento local, sem alterar o SQLite.
- **Atalhos (com o quadro em foco):** F enquadra tudo, Ctrl+F abre a busca e Esc fecha a busca ou limpa a seleção.
- **Sem migração:** mantém SQLite v10 da Alpha 10.2, sem dependências novas nem mudanças nos dados.

Consulte [roteiro de validação](docs/ALPHA10_3_NAVEGACAO.md).

## Alpha 10.2 — Cartões vinculados ao Arquivo

No Quadro, **Do Arquivo** permite buscar notas e tarefas existentes e inseri-las como cartões de referência. O conteúdo é consultado diretamente no SQLite sempre que a tela abre ou o RECALL emite uma alteração de notas/tarefas — **sem duplicação do texto**. As referências podem ser conectadas por setas e movidas como qualquer cartão, mas a edição permanece na tela **Arquivo**, acessível pelo botão Abrir no Arquivo. Itens excluídos aparecem como indisponíveis, sem mostrar conteúdo privado antigo.

Migração aditiva SQLite **v10**: tabela `whiteboard_references` com ID do cartão e ID da nota/tarefa. Antes de atualizar uma instalação v9, o aplicativo gera snapshot verificado `pre-alpha10-2-references`. Consulte [validação da Alpha 10.2](docs/ALPHA10_2_REFERENCIAS_DINAMICAS.md).

## Alpha 10.1 — Vários quadros e conexões

O seletor no topo do Quadro permite criar e renomear espaços independentes. Cada cartão fica associado ao seu quadro; o conteúdo existente da Alpha 10.0 migra para **Meu quadro** sem ser descartado. No cabeçalho de cada cartão, selecione **Conectar** e depois **Conectar** em outro cartão para criar uma seta persistente. Ao selecionar um cartão, suas conexões podem ser removidas na lista lateral.

A migração aditiva para SQLite **v9** exige backup verificado `pre-alpha10-1-boards` ao atualizar instalações v8; não faça downgrade direto para Alpha 10.0. Veja [documentação de validação](docs/ALPHA10_1_QUADROS_CONEXOES.md).

## Alpha 10 — Quadro Infinito

Crie notas, cole texto/links/tabelas (TSV) e imagens com Ctrl+V ou selecione uma imagem. Arraste os cartões, ajuste seu tamanho e faça zoom com a roda do mouse ou os controles. Alterações são persistidas por cartão no SQLite com proteção de revisão; imagens ficam no banco e entram nos backups.

Esta é a **primeira entrega experimental**, com um quadro único. Ainda não há setas/conexões, vários quadros, agrupamentos, histórico de desfazer, sincronização ou referências dinâmicas a notas/tarefas do Arquivo. A imagem individual aceita até 4,5 MB; há limites de 15 linhas e 8 colunas por tabela na interface. Rascunhos de texto são preservados localmente quando o salvamento falha, mas a recuperação visual de versões conflitantes ainda precisa de uma interface própria.

**Plano detalhado e testes:** [Alpha 10 — Quadro](docs/ALPHA10_QUADRO.md).

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
4. Selecione o run da branch `feature/v0.5-alpha10-3-navegacao-quadro` com **Success**, em **Artifacts** baixe `RECALL-v0.5-alpha10-3-navegacao-Windows-x64`.
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
| `src/components/Whiteboard.svelte` | Quadro Infinito, pan/zoom e edição visual |
| `src-tauri/src/whiteboard.rs` | Contratos SQLite do Quadro, CAS e imagens |
| `src-tauri/src/board_references.rs` | Índices de referências, atualização dinâmica a partir do Arquivo e migração v10 |
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
