# RECALL

**Sua memória externa — notas, lembretes e foco para Windows.**

**Versão principal:** v0.4. **Em desenvolvimento na branch `feature/v0.5`:** v0.5 alpha 1. Interface minimalista preta e vermelha.

> **Estado atual:** o código-fonte ainda não foi validado como executável Windows. Lembretes dependem de o aplicativo estar em execução. A sincronização Android não foi implementada.

## Novidades da v0.5 alpha 1

- **Hoje:** crie, edite e conclua tarefas, destacando uma atividade por vez.
- **Entrada:** capture ideias sem precisar organizá-las na hora.
- **Converter:** transforme ideias da Entrada em notas ou tarefas; conversão segura no SQLite.
- **Captura rápida:** Ctrl + Shift + Espaço salva ideias na Entrada.
- **Compatibilidade:** novas tabelas SQLite sem remover anotações antigas.

O campo opcional de data das tarefas ainda não gera notificações. Os lembretes existentes da v0.4 continuam vinculados a notas.

**A prévia TESTAR_AGORA.html ainda é da v0.4.** Para ver as telas novas, execute o aplicativo Tauri no Windows.

## Recursos da v0.4

- Notas locais com SQLite e salvamento automático.
- Captura rápida: `Ctrl + Shift + Espaço` enquanto o aplicativo está ativo na bandeja.
- Cartões flutuantes, fixação, pesquisa, foco e cartões de apresentação.
- Lembretes locais com o processo em execução.
- Prévia independente em `TESTAR_AGORA.html`, com armazenamento separado no navegador.

## Instalador automático para Windows

A branch feature/v0.5 gera um **instalador Windows x64 (NSIS)** no GitHub Actions, sem exigir Rust ou Node.js no seu computador. Veja [como baixar e instalar](docs/INSTALADOR_WINDOWS.md). É uma **versão alpha** sem assinatura digital, ainda dependente de testes manuais.

## Executar no Windows

Instale [Node.js LTS](https://nodejs.org/), [Rust](https://rustup.rs/) e os [pré-requisitos do Tauri no Windows](https://v2.tauri.app/start/prerequisites/) (incluindo MSVC Build Tools e WebView2).

```powershell
npm install
npm run check
npm run desktop
```

Alternativamente, utilize `INICIAR_WINDOWS.bat`. Para gerar o instalador:

```powershell
npm run installer
```

**Privacidade:** não publique bases de dados pessoais, notas reais, arquivos `.env` ou backups. O `.gitignore` impede o versionamento acidental dos formatos comuns de banco SQLite e de arquivos de configuração sensíveis.

## Estrutura do projeto

- `src/`: interface Svelte e TypeScript.
- `src-tauri/`: aplicativo Rust/Tauri e banco SQLite.
- `TESTAR_AGORA.html`: prévia executada no navegador.
- `.github/workflows/ci.yml`: verificação automatizada para Windows, com execução verificada no GitHub.
- `docs/`: identidade visual, testes e histórico.

## Próxima versão

O desenvolvimento da v0.5 ocorre na branch `feature/v0.5` e no PR #1, sem alterar a versão principal antes de revisão. O plano está em `docs/ROADMAP_V0.5.md`.

## Validação

O código foi importado de uma versão experimental. O GitHub Actions aprovou Svelte, TypeScript e cargo check no Windows, mas notificações, bandeja, atalho global e instalador ainda precisam de testes interativos no Windows. Não considere a v0.4 uma versão de produção.

## Workflow da nova fase (IA e Codex)

- [AGENTS.md](AGENTS.md): regras para agentes e continuidade.
- [Workflow](docs/AI_WORKFLOW.md), [skills](docs/AI_SKILLS.md) e [checkpoint](docs/AI_CHECKPOINT.md): decisões, estado verificado e próximos passos.
- [Configuração portátil](config/ai-workflow.yaml): preferências confirmadas.

A configuração não instala skills e não confirma testes em Windows real; consulte o checkpoint antes de agir.

## Alpha 4.1 — integridade de dados (branch experimental)

A branch \`feature/v0.5-alpha4-integridade\` adiciona revisão de notas, detecção de conflitos entre janelas, rascunhos recuperáveis, snapshots SQLite seguros e roteiro de restauração offline. Veja [guia de proteção e testes](docs/INTEGRIDADE_ALPHA4_1.md). Não altera a \`main\`; validação Windows real ainda é necessária.
