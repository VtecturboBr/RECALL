# RECALL v0.5 — Plano de desenvolvimento

## Objetivo

Evoluir o RECALL de bloco de notas para uma **memória externa** simples: capturar, lembrar, focar e retomar.

## Etapas

1. **Base estável:** validar compilação Tauri Windows, ícones, permissões, bandeja e atalho global. Corrigir erros antes de alterar o banco.
2. **Banco:** migrações versionadas com backup pré-migração, preservando registros v0.4. Estruturar notas, caixa de entrada, tarefas, passos, lembretes e histórico.
3. **Fluxo principal:** telas Hoje, Entrada, Notas e Apresentar. Evitar métricas e excesso de elementos visuais.
4. **Tarefas e retomada:** próximo passo, conclusão e registro explícito de "onde parei".
5. **Lembretes 2.0:** adiar, recorrência, tratamento de avisos vencidos e recuperação após reinício; verificar duplicidade.
6. **Dados seguros:** exportação/importação, backups SQLite consistentes, lixeira e recuperação.
7. **Confiabilidade:** testes unitários, fluxos de navegação e validação manual em Windows.

## Princípios

- Offline-first e zero assinatura externa.
- Captura rápida sem decisões e foco em uma tarefa por vez.
- Compatibilidade com dados existentes; nunca destruir banco durante migração.
- IDs estáveis, `updated_at` e tombstones para sincronização Android futura.
- Android e nuvem **fora do escopo da v0.5**.

## Critérios de pronto

- CI de frontend e Rust passa em Windows.
- A versão Windows é executada manualmente em computador com WebView2.
- Notas sobrevivem a reinícios; migração preserva dados antigos.
- Captura, lembretes, janelas flutuantes e backups têm cenários testados.
- Não há dados pessoais ou credenciais no Git.
