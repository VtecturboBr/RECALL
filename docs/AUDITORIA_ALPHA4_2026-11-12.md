# RECALL Alpha 4 — Auditoria técnica, UX e qualidade

**Data:** 12/11/2026  
**Objeto:** código da branch `feature/v0.5-alpha4-captura-tempo` no commit `8635152b785d2375eb5f44712320fde1d0d03f16`  
**Branch de auditoria:** `audit/alpha4-workflow-qualidade`  
**Natureza:** revisão estática aprofundada de código e documentação, confrontada com resultados já publicados no GitHub Actions. **Não equivale a teste funcional ou visual da aplicação instalada no Windows.**

## Parecer executivo

**Decisão:** a Alpha 4 está apta a continuar como alpha de testes, mas **não deve ser promovida a versão estável nem iniciar uma migração destrutiva de dados** sem corrigir os riscos P1 e executar regressão manual. O código tem boas bases de persistência local, mas as garantias de gravação e os lembretes ainda não são suficientemente robustos para um aplicativo de "memória externa".

**Inventário:** 4 achados P1 (prioridade alta), 11 P2 (relevantes), 4 P3 (polimento). Há riscos demonstrados pelo fluxo do código e problemas de design/integridade que **ainda exigem reprodução em execução real**. A classificação de gravidade não significa que o defeito já tenha sido observado pelo usuário.

### Estado realmente verificado

| Verificação | Resultado | Limite |
| --- | --- | --- |
| Código Alpha 4 consultado diretamente no repositório | Concluído | Revisão estática, não instrumentação runtime |
| Workflow e AGENTS sincronizados na branch de desenvolvimento | Concluído | Commit separado de documentação, sem código modificado |
| GitHub Actions de validação Alpha 4 | Success, run [43292313090](https://github.com/VtecturboBr/RECALL/actions/runs/43292313090) | Unitários, TypeScript/Svelte, build e Rust; não E2E |
| GitHub Actions de instalador Alpha 4 | Success, run [43292313317](https://github.com/VtecturboBr/RECALL/actions/runs/43292313317) | Build NSIS e checksum, não valida interação humana |
| Usuário abriu a Alpha 4 | Informado na conversa | Não forneceu resultados de regressão/persistência |
| Captura real de telas/fluxos, acessibilidade com leitor de tela | **Não executado** | Nesta sessão não há controle da GUI do Windows |
| Testes novos executados como parte da auditoria | **Nenhum** | Não afirmar que cenários abaixo já passaram |

### Como foi feita a avaliação

Três passagens: (1) arquitetura, persistência, migrações e lembretes; (2) fluxos de interface, foco, navegação e acessibilidade via código; (3) parser, testes, CI, segurança e desempenho. O catálogo de skills em `docs/AI_SKILLS.md` foi respeitado. O módulo próprio de simplificação e a exigência de verificação por evidência foram aplicados como método. A skill Product Design Audit foi localizada/lida, mas **não foi executada como auditoria visual**, pois seu procedimento exige screenshots do fluxo em uso; este ambiente não controla a janela Windows do RECALL. As skills externas do catálogo permanecem pendentes de instalação/carregamento verificável.

## Arquitetura observada

```text
Svelte 5 / TypeScript
  App.svelte
    ├── Simplified.svelte  (Agora, Arquivo, editores, apresentação)
    ├── QuickCapture.svelte (janela global)
    └── Sticky.svelte       (cartão flutuante)
           │ Tauri invoke + eventos
           ▼
Rust / Tauri 2 (src-tauri/src/lib.rs)
    ├── db.rs      -> notes + lembretes
    ├── tasks.rs   -> tasks + task_plans
    └── inbox.rs   -> inbox_items (conversões transacionais)
           │
           ▼
SQLite em app_data_dir()/memoria.sqlite, WAL, pool de 1 conexão
    + polling nativo de lembretes a cada 15 s
```

Pontos positivos sustentados pelo código: SQL parametrizado para entradas do usuário; conversão de ideia para nota/tarefa feita em transação (`inbox.rs:70-102`); exclusão lógica para notas e tarefas; SQLite em WAL e `busy_timeout`; `task_plans` aditiva, preservando as colunas anteriores; validação básica de tamanho/formatos no IPC; suporte à redução de movimento nas folhas CSS; interpretação de datas offline e testes do parser. Manter esses comportamentos na correção.

## Achados priorizados

Legenda: **Confirmado pelo código** = existe caminho verificável sem depender de observação visual; **Risco plausível** = código não possui proteção, mas frequência/consequência em uso exigem teste; **UX estática** = interface inferida de componentes/CSS, sem inspeção de screenshot.

### Prioridade P1 — Confiabilidade/dados

#### R01 — Cartão flutuante fecha mesmo após falha ao salvar
**Severidade:** P1 · **Confiança:** confirmada pelo código.  
**Evidência:** [`src/components/Sticky.svelte:27-43`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Sticky.svelte#L27-L43) e [`src-tauri/src/lib.rs:55-60`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/src/lib.rs#L55-L60).

`persist()` captura a exceção e apenas define `dirty=true`, mas **não comunica falha ao chamador**; `close()` chama `persist()` e fecha a janela independentemente do resultado. O cartão não define `maxlength` para título, enquanto o comando Rust rejeita título com mais de 250 caracteres. Logo, existe um caso reproduzível de texto não persistido com fechamento normal.

**Reprodução sugerida:** criar nota; abrir cartão; inserir título de 251 caracteres; clicar fechar; reabrir. Esperado: impedir fechamento/mostrar erro e preservar rascunho; atualmente o fluxo fecha após a falha. Também há salvamento assíncrono sem espera no desmontar da janela (`onMount` cleanup).

**Correção recomendada:** `persist(): Promise<boolean>`, serializar gravações e impedir fechar enquanto alterações não persistirem; oferecer opção explícita de descartar, e manter rascunho recuperável após erro. Testar erro de validação e falha SQLite.

#### R02 — Edições simultâneas podem sobrescrever conteúdo sem aviso
**Severidade:** P1 · **Confiança:** risco sustentado pelo código; reprodução simultânea pendente.  
**Evidência:** [`src-tauri/src/db.rs:91-95`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/src/db.rs#L91-L95), [`src/components/Simplified.svelte:151-187`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L151-L187) e [`src/components/Sticky.svelte:14-40`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Sticky.svelte#L14-L40).

`save_note` faz `UPDATE notes SET title=?, body=?` sem versão ou verificação do valor original. O editor principal e o cartão flutuante mantêm cópias independentes de ambos os campos. Gravação tardia de uma janela pode restaurar título/corpo antigo de outra.

**Correção:** controle otimista por `revision` ou `updated_at` confiável com atualização condicional; retornar conflito e permitir revisão/mesclagem, sem sobrescrita silenciosa. Cobrir duas janelas editando campos iguais e diferentes.

#### R03 — Lembrete considerado notificado sem confirmação de que o usuário o recebeu
**Severidade:** P1 · **Confiança:** limitação estrutural confirmada; efeito no Windows precisa ser observado.  
**Evidência:** [`src-tauri/src/lib.rs:242-265`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/src/lib.rs#L242-L265), [`src-tauri/src/db.rs:118-127`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/src/db.rs#L118-L127).

Quando `notification().show()` retorna `Ok`, o app marca `notified_at`, sem confirmação de visualização/leitura pelo usuário. Se a permissão do sistema bloquear o aviso, `show()` aceitar a chamada não é prova de entrega perceptível. Uma falha entre exibir e marcar pode resultar em repetição; múltiplos processos não têm garantia explícita de exclusão mútua. A lista `due()` processa até 30 pendências por ciclo de 15 segundos, com potencial de avalanche após tempo offline.

**Correção:** estados persistentes `pending / dispatched / acknowledged / snoozed`, deduplicação/idempotência e política de avisos vencidos; tratar permissão de notificação e incluir central interna com histórico/adiamento. Não prometer "entregue" quando só houve despacho ao SO.

#### R04 — Sem backup consistente, restauração nem migrações versionadas
**Severidade:** P1 · **Confiança:** ausência observada no escopo do código; risco de integridade.  
**Evidência:** [`src-tauri/src/db.rs:26-50`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/src/db.rs#L26-L50) e [`src-tauri/src/tasks.rs:22-43`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/src/tasks.rs#L22-L43).

Esquema cresce por `CREATE TABLE IF NOT EXISTS`; não existe tabela/rotina de versão de migração, rotina de backup SQLite ou recuperação no código atual. A estratégia aditiva da Alpha 4 é positiva, mas não substitui backup verificável antes de versões futuras.

**Correção:** migrações explícitas versionadas, backup consistente usando recursos SQLite adequados ao modo WAL, checksum/versão, e restauração testada. Nunca copiar somente `memoria.sqlite` enquanto o WAL ativo pode conter operações ainda não consolidadas.

### Prioridade P2 — Corretude, UX, segurança e capacidade de evolução

#### R05 — Atualizações concorrentes recarregam todos os dados e podem chegar fora de ordem
**Confiança:** risco plausível. **Evidência:** [`src/components/Simplified.svelte:73-95`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L73-L95).

Cada evento `notes/tasks/inbox:changed` aciona `refresh()` completo, e várias ações chamam `refresh()` outra vez. Não há deduplicação, cancelamento ou token de geração: resposta antiga pode substituir uma mais recente. Uma atualização bem-sucedida também zera `error`, mesmo se a mensagem for de outra operação.

**Correção:** coalescer eventos, impedir aplicação de resposta obsoleta, preservar estado de erro por operação, aplicar atualização incremental quando viável.

#### R06 — Sem testes de persistência, conflito, notificações e UI integrada
**Confiança:** confirmada pela matriz de testes presente. **Evidência:** `tests/smartCapture.test.mjs`, `tests/workspace.test.mjs`, `src-tauri/src/inbox.rs:104-116`, `src-tauri/src/tasks.rs:123-137` e `.github/workflows/ci.yml:35-46`.

Testes atuais exercitam parser, classificação, busca e validadores simples, mas não cobrem transações SQLite, atualização concorrente, fechamento com falha, migração desde a v0.4, lembretes, atalho global e regressão visual/E2E.

**Correção:** testes SQLite com banco temporário; cenários de conflito e rollback; smoke test Windows pós-instalação; automação visual quando houver navegador/sessão controlável.

#### R07 — Builds não reprodutíveis por ausência de arquivos lock
**Confiança:** confirmada pelo inventário do repositório. **Evidência:** `package.json:16-27`, `src-tauri/Cargo.toml`, `.github/workflows/ci.yml:35-36`.

O repositório não contém `package-lock.json` nem `src-tauri/Cargo.lock` no estado auditado. Uso de `npm install` e ranges de versões pode resolver dependências transitivas diferentes nas próximas builds.

**Correção:** versionar os lockfiles adequados ao aplicativo, usar `npm ci` no GitHub Actions e verificar dependências antes de atualizá-las.

#### R08 — CSP desativada no aplicativo Tauri
**Confiança:** configuração observada; **não** representa exploit confirmado. **Evidência:** [`src-tauri/tauri.conf.json:39-42`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/tauri.conf.json#L39-L42) e [`src-tauri/capabilities/default.json:1-15`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/capabilities/default.json#L1-L15).

`csp: null` elimina camada útil de defesa do WebView. A interface atual usa escapamento padrão Svelte, o que reduz a exposição, mas não justifica manter o endurecimento ausente indefinidamente.

**Correção:** adotar política CSP restritiva compatível com Tauri/estilos locais, revisar permissões IPC/janelas e testar especialmente captura, cartões e apresentação.

#### R09 — “Semana que vem” é reconhecido, mas não tem data de retorno
**Confiança:** confirmado por código e teste. **Evidência:** [`src/lib/smartCapture.ts:31-47`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/lib/smartCapture.ts#L31-L47), [`tests/smartCapture.test.mjs:27-33`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/tests/smartCapture.test.mjs#L27-L33) e [`src/lib/workspace.ts:64-74`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/lib/workspace.ts#L64-L74).

O parser salva `bucket='later', plannedFor=null`. Sem data, a tarefa **nunca chega automaticamente em Hoje**, apesar da expressão temporal indicar a semana seguinte. O usuário pode interpretar isso como agendamento automático.

**Correção:** definir dia da semana pelo comportamento documentado, pedir confirmação contextual ou classificar explicitamente como "Depois, sem data" e não insinuar programação.

#### R10 — Horários já passados podem ser criados para hoje sem aviso
**Confiança:** comportamento inferido diretamente da lógica do parser. **Evidência:** [`src/lib/smartCapture.ts:77-104`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/lib/smartCapture.ts#L77-L104).

Exemplo com relógio local às 17h: `Revisar trabalho às 09h` resulta em `dueAt` de hoje às 09h. Não há validação nem aviso de prazo passado. Isso é diferente de tarefa atrasada deliberadamente criada.

**Correção:** prévia com alerta "horário já passou", sem alterar o texto silenciosamente. Testar noite, virada do dia e horário de verão.

#### R11 — Os atalhos “Depois”/“Algum dia” não filtram por período
**Confiança:** confirmado pelo código. **Evidência:** [`src/components/Simplified.svelte:504-508`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L504-L508) e [`src/lib/workspace.ts:50-60`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/lib/workspace.ts#L50-L60).

Ambos apenas mudam `archiveFilter='task'`; no Arquivo aparecem tarefas de Hoje, Depois e Algum dia juntas. O botão comunica uma ação que não realiza.

**Correção:** adicionar filtro secundário temporal ou abrir Arquivo com `period=later/someday`, preservando pesquisa e seleção.

#### R12 — Carregamento integral e busca em todos os textos a cada atualização
**Confiança:** gargalo estrutural; **sem benchmark executado**. **Evidência:** [`src/components/Simplified.svelte:56-85`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L56-L85), [`src/lib/workspace.ts:8-35`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/lib/workspace.ts#L8-L35) e [`src/lib/workspace.ts:50-60`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/lib/workspace.ts#L50-L60).

O front carrega todas as notas/tarefas/ideias, recria entradas, ordena e normaliza todo o texto de notas durante busca. Em bancos pequenos isso é aceitável; com milhares de registros ou corpos de até 100 mil caracteres, tende a escalar mal.

**Correção:** medição primeiro; índices/search SQLite FTS quando justificado, paginação/virtualização e cache de texto normalizado; debounce ou worker conforme perfil medido. **Não** adicionar infraestrutura prematuramente.

#### R13 — Apresentação declara diálogo modal sem gerenciar foco de teclado
**Confiança:** risco de acessibilidade pelo código, validação com leitor de tela pendente. **Evidência:** [`src/components/Simplified.svelte:97-110`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L97-L110) e [`src/components/Simplified.svelte:639-651`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L639-L651).

Há `role='dialog'` e `aria-modal='true'`, mas sem foco inicial, contenção de Tab e restauração do foco após fechar. Setas são escutadas globalmente. Pode causar navegação confusa para teclado/leitores de tela.

**Correção:** gestão de foco, Escape, Tab, leitura de contador e restauração, testados com teclado e ao menos um leitor de tela.

#### R14 — Lembretes antigos somem da tela Agora após disparo
**Confiança:** confirmado pela regra da interface. **Evidência:** [`src/components/Simplified.svelte:69-70`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L69-L70) e [`src/components/Simplified.svelte:511-518`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L511-L518).

A UI filtra `notified_at === null` e portanto remove o item após despacho. Não existe área de histórico, adiamento ou confirmação; o usuário pode perder o aviso e não localizar o que venceu.

**Correção:** separar pendentes, vencidos e confirmados; permitir abrir/reagendar sem gerar nova tela complexa.

#### R15 — Exclusão não oferece desfazer ou restauração no app
**Confiança:** comportamento observado no código. **Evidência:** [`src/components/Simplified.svelte:419-433`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L419-L433), [`src-tauri/src/db.rs:112-116`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/src/db.rs#L112-L116) e [`src-tauri/src/inbox.rs:53-59`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src-tauri/src/inbox.rs#L53-L59).

Notas/tarefas têm exclusão lógica e ideias são marcadas processadas, mas a interface não permite ver ou recuperar o item. Confirmação via `confirm()` não substitui desfazer, especialmente em um aplicativo de memória externa.

**Correção:** desfazer imediato, depois lixeira com prazo e restauração; cobrir também descarte de ideias.

### Prioridade P3 — Polimento e manutenção

#### R16 — Identidade da versão e documentação fora de sincronia
**Confiança:** confirmado. **Evidência:** [`src/components/Simplified.svelte:438-445`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L438-L445) vs. `src-tauri/tauri.conf.json:4` e `README.md`.

O rodapé ainda mostra **α3** quando o executável é Alpha 4. O README herdado da linha principal ainda apresenta v0.4/Alpha 1 no topo; o guia `docs/ALPHA_4.md` está mais atual.

**Correção:** versão vinda de metadados e README reorganizado por branch/estado; não alterar histórico retroativamente.

#### R17 — Captura em Agora e captura global têm limites/experiências diferentes
**Confiança:** confirmado. **Evidência:** [`src/components/Simplified.svelte:520-535`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/Simplified.svelte#L520-L535) e [`src/components/QuickCapture.svelte:80-93`](https://github.com/VtecturboBr/RECALL/blob/8635152b785d2375eb5f44712320fde1d0d03f16/src/components/QuickCapture.svelte#L80-L93).

A entrada inline limita a 250 caracteres inclusive para ideias; a janela global aceita até 100 mil caracteres e múltiplas linhas. Diferença não é explicada, embora a captura seja conceitualmente uma ação única.

**Correção:** explicar limite curto/long-form ou consolidar o componente de captura com experiências coerentes.

#### R18 — Componentes/estilos legados continuam no projeto e aumentam custo de manutenção
**Confiança:** confirmado pelo inventário. **Evidência:** `src/App.svelte:1-16`, `src/main.ts:1-9` e diretórios `src/components/` e `src/`.

O app renderiza `Simplified.svelte`, `QuickCapture.svelte` e `Sticky.svelte`, mas mantém `Main.svelte`, `Today.svelte`, `Inbox.svelte`, `v05.css` e uma prévia HTML antiga. `styles.css` preserva estilos legados e há camadas adicionais `simplified.css`, `alpha3.css` e `alpha4.css`.

**Correção:** identificar dependências reais e testes antes de arquivar/remover código morto. Não apagar `styles.css` integralmente: contém estilos ainda necessários para captura e cartão flutuante.

#### R19 — Contraste insuficiente para texto pequeno na contagem de resultados
**Confiança:** cálculo estático da combinação declarada, sem inspeção visual final. **Evidência:** `src/alpha3.css:38` + `src/simplified.css` (`.rx-items`).

`.rx-results` usa texto `#777782` sobre fundo `#111114`: contraste calculado de **4,26:1**, abaixo de **4,5:1** requerido pelo nível AA para texto pequeno. A fonte ali é 10,5 px.

**Correção:** elevar contraste e manter hierarquia visual discreta; validar os demais pares depois com renderização real e teste de zoom.

## Observações de arquitetura e desempenho

- **Acerto:** banco local, IDs estáveis, transações de conversão, validação no IPC e planejamento aditivo reduzem risco de mudanças destrutivas.
- **Trade-off:** `SqlitePool` com uma conexão simplifica coordenação inicial; pode gerar filas com várias janelas, mas só benchmark real justificará aumentar o pool.
- **Risco de corrida de UX:** `refresh()` precisa coalescência; cartões e editor principal precisam de protocolo de revisão.
- **Modelagem:** notas, tarefas e ideias estão unificadas apenas na projeção `buildWorkspace`, não em um único tipo persistente. É uma escolha segura por ora; evitar migração "unificada" antes de backups e testes.

## UX — avaliação por etapas, limitada à estrutura de código

| Etapa do fluxo | Avaliação estática | Observação |
| --- | --- | --- |
| 1. Abrir Agora | Funcionalidade coerente | Foco destaca uma tarefa; data da tarefa é apenas informativa |
| 2. Registrar ideia/tarefa | Bom fluxo básico, ajuste necessário | Duas superfícies de captura com capacidades diferentes (R17) |
| 3. Agendar para Depois | Requer correção semântica | "Semana que vem" não possui dia de retorno (R09) |
| 4. Consultar Depois/Algum dia | Defeito de navegação inferido diretamente | Botões abrem todas as tarefas (R11) |
| 5. Editar no Arquivo | Forte potencial, mas risco de consistência | Autosave e sincronização entre janelas (R02, R05) |
| 6. Abrir/fechar nota flutuante | Risco grave de perda | Fechamento após erro de gravação (R01) |
| 7. Receber lembrete | Não confiável como "recebimento confirmado" | Falta confirmação, histórico e adiamento (R03, R14) |
| 8. Apresentar cartões | Precisa teste de acessibilidade | Foco em modal sem gestão (R13) |
| 9. Excluir / restaurar | Incompleto | Não há desfazer/restauração visual (R15) |

**Sem screenshots, não foi possível confirmar** composição real, tamanhos visuais, legibilidade em zoom, estados de foco em leitor de tela, animações, responsividade da WebView, ergonomia da janela, comportamento do tray nem percepção do usuário.

## Critérios para autorizar próximas fases

1. **P1 resolvidos e testados:** nenhuma edição some ao falhar a gravação ou ao alternar janela; backups e restauração verificados; lembretes modelados com estado confiável.
2. **Contrato das datas claro:** prazo x retorno à tela Hoje x aviso nativo são conceitos distintos e explícitos.
3. **CI reprodutível:** lockfiles versionados, testes DB e smoke Windows, preservando dados antigos.
4. **UX validada no Windows:** testes reais de captura global, cartões, foco, notificações e navegação com teclado.
5. **Confiabilidade antes do redesign:** comparar três direções visuais após estabilizar fluxos críticos, sem reintroduzir telas redundantes.

## Entregáveis relacionados

- [Plano de correções](ALPHA4_PLANO_CORRECOES.md): ordem, critérios de aceitação e três ciclos de refinamento.
- [Matriz de testes Windows](ALPHA4_TESTES_MANUAIS.md): passos executáveis; todos inicialmente **não executados**.
- [Checkpoint](../AI_CHECKPOINT.md): registrar decisões confirmadas e próximo passo.

**Esta auditoria é somente diagnóstico.** Nenhum problema descrito foi alterado/corrigido no código nesta branch e **não há autorização para merge na main**.
