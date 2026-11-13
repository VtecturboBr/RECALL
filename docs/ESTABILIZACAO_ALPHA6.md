# RECALL Alpha 6 — estabilização de integridade, interface e segurança

**Base:** `feature/v0.5-alpha6-frontend-redesign` (`a37f4557`).  
**Data da análise:** 12–13/11/2026.  
**Status:** correções experimentais em PRs empilhados; não publicadas na `main`.

## PRs e sequência de integração (sem merge automático)

1. [#10 — Captura inline](https://github.com/VtecturboBr/RECALL/pull/10), base Alpha 6.
2. [#11 — Integridade](https://github.com/VtecturboBr/RECALL/pull/11), base #10.
3. [#12 — Interface](https://github.com/VtecturboBr/RECALL/pull/12), base #11.
4. [#13 — Notificações e segurança](https://github.com/VtecturboBr/RECALL/pull/13), base #12.

Todos em **rascunho** até revisão e testes reais no Windows. Mesmo que a CI passe, isso não comprova comportamento visual, alertas nativos nem persistência após desligar a máquina.

## O que foi corrigido

| Auditoria | Mudança | Como verificar |
| --- | --- | --- |
| A6-01 | Captura embutida recuperável: texto e tipo em armazenamento local | Digitar, fechar sem enviar, reabrir e guardar |
| A6-02 | Rascunhos por identificador para tarefa/ideia, preservados antes do autosave e restaurados ao selecionar | Forçar encerramento antes dos 900 ms |
| A6-03 | Revisões CAS para tarefas e ideias, inclusive com trigger para gravações de versões antigas; ação de criar cópia | Duas janelas salvam revisão igual, apenas uma grava |
| A6-04 | Editor de nota sincronizado se limpo, mantém texto se houver edição local | Alterar nota no cartão flutuante com editor aberto |
| A6-05 | Resumo considera todo backlog; eventos `suppressed` persistentes para lotes agrupados | Mais de 150 lembretes vencidos |
| A6-06 | Notas fixadas aparecem primeiro, inclusive após outras alterações | Fixar nota A e editar B |
| A6-07 | Filtros por período omitem tarefas concluídas | Concluir tarefa Hoje |
| A6-08 | Conversão de ideia limpa filtros/busca incompatíveis | Filtrar Ideias e converter em Nota |
| A6-09 | Nova nota limpa busca que a ocultaria | Buscar palavra inexistente e criar nota |
| A6-10 | Snapshots antigos de refresh são descartados | Completar requisições fora de ordem |
| A6-11 | Concluir tarefa + cancelar lembrete na mesma transação | Falha SQL ao cancelar deve reverter tarefa |
| A6-12 | Toasts Windows mostram apenas conteúdo genérico | Inspecionar central/tela bloqueada |
| A6-13 | Drawer com backdrop e Esc em viewport até 680px CSS | Testar zoom 150–200% |
| A6-14 | CSP ativa em produção e desenvolvimento, permitindo somente origem local e IPC essencial | Testar WebView2, janelas, atalho e HMR |

## Banco e preservação

- Adicionadas tabelas auxiliares `task_revisions` e `inbox_revisions` com gatilhos de inserção/edição. Nenhuma coluna de conteúdo legada é removida.
- `PRAGMA user_version` passa a **3**. A abertura de banco com tabelas antigas e sem revisões cria primeiro um snapshot SQLite consistente e verificado `recall-pre-content-revisions-*.sqlite`. Se backup falhar, a migração é bloqueada.
- Ao detectar conflito de revisão, nunca sobrescrever silenciosamente. O editor mantém o rascunho local; o usuário pode gravar versão independente.
- Armazenamento local é mecanismo de recuperação de conteúdo *não confirmado*, não substituto do SQLite nem do backup externo.
- Conclusão de tarefa e cancelamento de lembrete fazem `COMMIT` conjunto. O histórico de lembretes permanece.
- A notificação nativa informa apenas que há um lembrete. `dispatched_at` registra aceitação pela API, não que a pessoa viu o alerta.

## Validações requeridas antes do merge

**Automatizadas:** `npm run check`, `npm run test:unit`, `npm run build`, `cargo check`, `cargo test --lib`, testes Python de restauração e build NSIS via GitHub Actions.

**Manuais no Windows, usando banco de teste e backup externo:**
1. Digitar captura inline e encerrar sem salvar; verificar recuperação.
2. Editar tarefa e ideia, encerrar processo antes dos 900 ms; verificar recuperação ao selecionar.
3. Editar mesmo item em duas instâncias; verificar conflito e ação de criar cópia (sem sobrescrita).
4. Editar a mesma nota no Arquivo e no cartão flutuante, com editor limpo e sujo.
5. Fixar nota A, alterar nota B, verificar ordenação e pesquisa.
6. Concluir Hoje, converter ideia sob filtro, criar nota com busca ativa.
7. Forçar respostas de refresh fora de ordem (mock/instrumentação).
8. Simular 151+ lembretes vencidos, conferir resumo/histórico/atenção.
9. Concluir tarefa com lembrete e simular falha transacional.
10. Verificar ausência de informações pessoais no toast, inclusive tela bloqueada.
11. Testar zoom 125%, 150% e 200%, teclado e Escape no menu.
12. Verificar CSP e IPC no WebView2 em janelas principal, captura e cartão; testar HMR em desenvolvimento.
13. Testar instalação sobre banco antigo copiado, `PRAGMA quick_check` e restauração offline.
14. Reiniciar e reabrir RECALL para confirmar lembretes, rascunhos e gravações concluídas.

**Importante:** esta documentação não declara nenhum dos testes manuais como realizado.
