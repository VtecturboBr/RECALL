# Plano de correções após a auditoria Alpha 4

**Base técnica auditada:** `8635152b785d2375eb5f44712320fde1d0d03f16`.  
**Regras:** manter a Alpha 4 utilizável; conservar SQLite, identificador Tauri e conteúdo legado; sem nova seção obrigatória; até três ciclos de refinamento por entrega; cada PR deve declarar testes executados.

## Ordem proposta (sem implementação nesta branch)

### Ciclo 1 — Não perder dados (bloqueia próximas versões)
1. **R01 e R02:** serializar salvamento das notas e introduzir detecção de conflito entre janelas, especialmente notas flutuantes; impedir fechamento após falha de gravação.
2. **R04:** criar backup SQLite consistente, restauração e migração versionada antes de mudar schema de lembretes.
3. **R06:** adicionar testes SQLite (criar, editar, converter, excluir, migrar, restaurar, falha de gravação) e roteiros de teste com notas antigas.
4. Revisar comunicação de erro e estado de salvamento do editor.

**Critérios de aceitação:**
- Com falha proposital no banco/validação, fechar cartão não descarta o rascunho silenciosamente.
- Duas janelas editando a mesma nota não sobrescrevem uma à outra sem alerta recuperável.
- Backup é íntegro e restauração devolve todos os registros em banco temporário.
- Atualização a partir de banco v0.4/Alpha 3/Alpha 4 conserva o número de notas, tarefas e ideias.
- GitHub CI passa; cenários Windows executados ou claramente pendentes.

### Ciclo 2 — Lembretes confiáveis + semântica temporal
1. **R03 e R14:** modelar aviso pendente, despachado, vencido, adiado, recorrente e reconhecido; proteger de duplicatas e avisos em lote após offline.
2. **R09 e R10:** resolver "semana que vem", horários já passados e prévia inequívoca; distinguir "fazer", "prazo" e "notificar".
3. **R05 e R12:** reduzir refreshs concorrentes; medir tempo de busca e abertura antes de otimizar.
4. **R07:** lockfiles e `npm ci` no pipeline.

**Critérios de aceitação:**
- Sem aviso perdido por fechamento/reinício simulado; nenhuma duplicata em cenário controlado.
- Adiar e recorrência sobrevivem reinício; transição entre períodos funciona sem mudança implícita indevida.
- Datas inválidas e horários passados apresentam comportamento compreensível.
- Semântica de "semana que vem" aparece na prévia e persiste corretamente.
- Testes automáticos de calendário (timezone, virada do dia, mudança de mês) e testes nativos de notificação documentados.

### Ciclo 3 — Produto polido e acessível
1. **R11, R13, R15:** filtros temporais verdadeiros, foco de teclado em apresentação, desfazer/lixeira.
2. **R16-R19:** corrigir versão e documentação, revisar limites da captura e contraste, arquivar legado apenas após testes.
3. **R08:** endurecer CSP e revisar permissões Tauri com testes de regressão.
4. Realizar comparação real de três direções **antes de um redesign**:
   - **A — Refinamento conservador:** manter Agora/Arquivo, tipografia e feedback melhores.
   - **B — Contextual:** Agora centrado em foco e ação rápida, Arquivo com filtros semânticos compactos.
   - **C — Temporal discreto:** organização por Hoje/Depois/Algum dia dentro das mesmas áreas, sem calendário como tela principal.
   Recomenda-se escolher após ver fluxos reais, não antecipar redesign pelo relatório.

**Critérios de aceitação:**
- Cada controle "Depois" e "Algum dia" abre somente o conjunto correspondente.
- Apresentação navegável com Tab, Escape, setas e leitor de tela, com foco restaurado.
- Contraste de texto regular >=4,5:1 nos estilos aplicáveis; zoom e foco testados.
- Captura e cartões preservados após endurecer CSP.
- Conjunto de screenshots reais e verificação Windows por usuário/ambiente controlável.

## Como dividir Pull Requests

- **PR A: integridade e backup** — somente mudanças indispensáveis do ciclo 1.
- **PR B: lembretes 2.0** — migração versionada e testes para recorrência/adiamento.
- **PR C: UX e acessibilidade** — direção visual escolhida, regressão e limpeza segura.

Não empilhar PRs concorrentes sobre `main` enquanto a Alpha 4 ainda estiver em linha de desenvolvimento. Usar branches derivadas da linha de produto ativa e separar diagnóstico de implementação.

## Definição de pronto e evidência

Para cada PR: objetivo, reproduções antes, mudanças limitadas, impacto no banco, testes Rust/Node/Svelte, CI, testes Windows confirmados vs não executados, capturas quando houver validação visual, riscos restantes, checkpoint atualizado. **Não tratar build de instalador como teste de funcionamento no Windows**.
