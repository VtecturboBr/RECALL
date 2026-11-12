# RECALL

**Sua memória externa — notas, lembretes e foco para Windows.**

Versão importada: **0.4 (captura rápida)**. Projeto em desenvolvimento, com interface minimalista preta e vermelha.

> **Estado atual:** o código-fonte ainda não foi validado como executável Windows. Lembretes dependem de o aplicativo estar em execução. A sincronização Android não foi implementada.

## Recursos da v0.4

- Notas locais com SQLite e salvamento automático.
- Captura rápida: `Ctrl + Shift + Espaço` enquanto o aplicativo está ativo na bandeja.
- Cartões flutuantes, fixação, pesquisa, foco e cartões de apresentação.
- Lembretes locais com o processo em execução.
- Prévia independente em `TESTAR_AGORA.html`, com armazenamento separado no navegador.

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
- `.github/workflows/ci.yml`: verificação automatizada para Windows, ainda dependente da primeira execução.
- `docs/`: identidade visual, testes e histórico.

## Próxima versão

O desenvolvimento da v0.5 seguirá pela branch `feature/v0.5` e por Pull Requests. O planejamento estará nessa branch, sem alterar a versão principal antes de revisão.

## Validação

O código foi importado de uma versão experimental. A compilação, notificações, bandeja, atalho global e instalador ainda precisam de testes reais no Windows. Não considere a v0.4 uma versão de produção.
