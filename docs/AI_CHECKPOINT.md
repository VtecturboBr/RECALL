# Checkpoint de continuidade — RECALL

**Atualizado:** 2026-11-12
**Natureza:** estado observado no GitHub + decisões confirmadas na conversa. Revalidar a cada nova sessão.

## Produto e código

- **Repositório:** https://github.com/VtecturboBr/RECALL
- **Branch principal:** main — base v0.4; evitar supor que contém código da v0.5.
- **Branch de desenvolvimento mais recente verificada:** feature/v0.5-alpha4-captura-tempo
- **HEAD observado:** 8635152b785d2375eb5f44712320fde1d0d03f16
- **PR da Alpha 4:** https://github.com/VtecturboBr/RECALL/pull/4
- **Stack:** Tauri 2 + Rust + SQLite; Svelte 5 + TypeScript + Vite; Windows 64 bits.
- **Interface v0.5:** Agora e Arquivo, com criação de ideias e tarefas, pesquisa, foco manual e edição.
- **Alpha 4:** interpretação offline de datas simples em português e períodos Hoje/Depois/Algum dia.
- **Compatibilidade:** nova tabela aditiva task_plans; preservar notes, tasks, inbox_items e identificador da aplicação.
- **Prazos de tarefas Alpha 4 NÃO geram notificações.** Lembretes existentes de notas têm funcionamento dependente do aplicativo aberto.
- **Instalador Alpha 4:** NSIS x64 pelo GitHub Actions; arquivo publicado e checksum conferido na conversa.
- **Testes GitHub Actions observados:** validação e geração do instalador do commit acima com conclusão success. Não equivalem a testes de uso real.

## Validação e riscos pendentes

O usuário confirmou que abriu a Alpha 4 no Windows, mas não forneceu resultados de testes detalhados dos fluxos. Portanto **não afirmar** que persistência, atalhos, notificações, bandeja, instalação sobre versões antigas ou UX foram validados manualmente.

O README da branch main ainda descreve a v0.4/v0.5 inicial; consultar docs/ALPHA_4.md na branch de desenvolvimento como fonte específica da Alpha 4.

## Próxima direção do produto já discutida

Plano proposto em três entregas:
1. Captura inteligente e organização temporal — implementada na Alpha 4, falta validar uso real.
2. Lembretes confiáveis: tarefas e notas, adiamento, recorrência, avisos vencidos, deduplicação.
3. Onde parei?: recuperar contexto e próximo passo na tarefa ativa.

O usuário iniciou uma nova fase focada em qualidade, refinamento e continuidade e escolheu o workflow definido em docs/AI_WORKFLOW.md. **Nenhuma tarefa adicional de código foi autorizada durante a configuração.** Antes do próximo desenvolvimento, recuperar o pedido concreto.

## Decisões firmes para futuras sessões

- Autonomia alta; máximo acabamento; três direções em grandes redesigns; até três ciclos de refinamento.
- Ferramentas locais gratuitas permitidas; alternativas pagas podem ser sugeridas sem compra.
- Entrega: resultado pronto e resumo curto; continuidade versionada neste GitHub.
- Todos os grupos de skills habilitados para seleção contextual, não instalados automaticamente.
- Não adicionar funções redundantes, nem substituir arquitetura sem plano de migração.
- Documentar testes executados e pendências com evidência.

## Atualização do checkpoint

Após cada entrega concreta: registrar branch, commit/PR, funcionalidades modificadas, evidências dos testes, decisões confirmadas, riscos e próxima ação. Não registrar propostas como recursos já implementados.

## Implementação de integridade (branch separada, ainda em revisão)

- Branch: \`feature/v0.5-alpha4-integridade\`; base: \`feature/v0.5-alpha4-captura-tempo\`.
- Escopo: revisões e detecção de conflito em \`notes\`, recuperação local de rascunhos, proteção de notas flutuantes, backup WAL consistente, validação, restauração offline assistida, testes SQLite/Rust e Python.
- Documento: \`docs/INTEGRIDADE_ALPHA4_1.md\`.
- Migração: nova tabela auxiliar \`note_revisions\` com backup pré-migração e \`PRAGMA user_version=1\`; não remove colunas.
- **Atenção:** o PR é trabalho em andamento e não deve ser integrado sem CI e testes Windows. Não tratar a existência do código como validação do comportamento.
- Lembretes 2.0 e \`Onde parei?\` ficam fora deste PR.

## Lembretes 2.0 — nova branch (Alpha 5)

- Base verificada antes do trabalho: `feature/v0.5-alpha4-integridade`, commit `0e424456a5e42bffd15e207b0bdd9c0c1249084e`; PR #6 aberto, rascunho, não integrado à main; CI e NSIS anteriores passaram.
- Nova branch: `feature/v0.5-alpha5-lembretes2`, **não fazer merge na main**.
- Modelo: tabela aditiva `reminder_rules` com estados scheduled/snoozed/attention/done/cancelled e tabela `reminder_events`, migração das notas legadas, backup verificado antes do schema novo.
- Recursos: agendamento opt-in de notas/tarefas, repetição diária/semanal/mensal, adiamento, histórico, fila de vencidos persistente, poll Rust e resumo de lotes antigos.
- Documento: `docs/LEMBRETES_2_ALPHA5.md`.
- Necessário: CI completa da branch, build NSIS, testes reais Windows com permissão nativa de notificações e migração de banco teste. Não declarar estável sem esse ciclo.
- Próxima fase posterior, fora do escopo: modo Onde parei?.
