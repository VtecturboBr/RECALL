# Auditoria de bugs — RECALL Alpha 6 (redesign)

**Data:** 12 de novembro de 2026 (revisão de código).  
**Revisão exata:** \`a37f4557b6afc013d0cfdcd75007a3bd600a222a\`  
**Branch auditada:** \`feature/v0.5-alpha6-frontend-redesign\`; PR #8, base Alpha 5.  
**Método:** leitura direta de Svelte/TypeScript, CSS, APIs Tauri, Rust, SQLite, testes automatizados e workflows. **Não houve execução interativa do aplicativo instalado no Windows**, screenshots nem simulação operacional de falhas.  
**Status da CI já existente:** [Windows #43338021001](https://github.com/VtecturboBr/RECALL/actions/runs/43338021001) com sucesso, svelte-check 0 erros / 0 avisos, 15 testes Rust e 3 testes Python aprovados; [instalador #43338021091](https://github.com/VtecturboBr/RECALL/actions/runs/43338021091) concluído com sucesso.

## Critérios e classificação

- **P1 / alta:** pode descartar uma edição não salva ou sobrescrever mudanças concorrentes; tratar antes de promover a versão.
- **P2 / média:** comportamento incorreto, inconsistência de estado, alerta omitido, privacidade ou degradação significativa em determinadas condições.
- **P3 / baixa:** acessibilidade em cenários específicos ou endurecimento preventivo.
- **Confirmado no código:** uma sequência determinística de leitura dos trechos demonstra o defeito ou a ausência de proteção.
- **Risco de corrida / validação pendente:** interleaving plausível encontrado na implementação; requer reprodução automatizada ou manual para confirmar frequência, contexto e impacto.

**Resumo:** 14 achados — **3 P1, 9 P2, 2 P3**. *A CI verde verifica compilação e testes cobertos, não a ausência destes defeitos.*

## Achados de alta prioridade

### A6-01 — P1 · Captura inline não salva rascunho antes do envio
**Evidência:** \`RecallStudio.svelte:45,174-192\`. \`newText\` existe somente no estado Svelte e é zerado após o envio, sem \`localStorage\`/SQLite para conteúdos ainda não enviados. Em contraste, \`QuickCapture.svelte:17-24,90\` persiste e restaura o rascunho.

**Reprodução prevista:** em **Agora**, digitar um texto longo na captura embutida; sair completamente do aplicativo sem clicar em Guardar; reabrir. O texto não é recuperado.  
**Impacto:** perda de pensamento não enviado — especialmente relevante na missão de memória externa.  
**Correção sugerida:** persistir texto, tipo e timestamp localmente a cada edição; restaurar ao iniciar; apagar rascunho somente após confirmação de escrita no banco; teste com reinício.

### A6-02 — P1 · Edições de tarefas e ideias não têm recuperação após interrupção abrupta
**Evidência:** \`RecallStudio.svelte:69-81,358-409,429-465\`. Ideias e tarefas são persistidas com debounce de 900 ms; antes de a gravação terminar, não há cópia persistente de \`ideaText\`, \`taskTitle\`, \`taskDetails\` ou \`taskDue\`. O callback de desmontagem (\`155-165\`) não é uma garantia de gravação quando o processo é encerrado à força ou perde energia. Para notas, já existe \`preserveNoteDraft\` em \`54-63,194-200\`.

**Reprodução prevista:** editar tarefa/ideia; encerrar processo imediatamente antes do debounce; reiniciar. A última edição pode faltar.  
**Impacto:** perda de alterações em itens organizados, não apenas ideias recém-capturadas.  
**Correção sugerida:** mecanismo comum de rascunho recuperável para os três tipos, sem confundir cache local com commit SQLite. Testar falha da gravação, encerramento abrupto e persistência após recuperar.

### A6-03 — P1 · Tarefas e ideias aceitam sobrescrita concorrente (last write wins)
**Evidência:** \`src-tauri/src/tasks.rs:99-103\` e \`src-tauri/src/inbox.rs:43-51\` executam \`UPDATE\` incondicional sem revisão/versão. O frontend envia texto completo. Notas, ao contrário, usam compare-and-swap em \`db.rs:193-203\`. O app não registra proteção explícita de instância única em \`lib.rs:279-411\`.

**Reprodução proposta:** se duas instâncias/janelas editoras da mesma base estiverem ativas, ler a mesma tarefa/ideia em ambas; salvar conteúdo diferente em sequência. A segunda gravação substitui a primeira sem sinalizar conflito.  
**Impacto:** perda silenciosa de conteúdo concorrente.  
**Correção sugerida:** revisão otimista também para \`tasks\` e \`inbox_items\`, ação de recuperar cópia, testes de duas atualizações com a mesma revisão; avaliar single-instance nativo.

## Achados funcionais e operacionais

### A6-04 — P2 · Editor principal não se sincroniza com mudanças do cartão flutuante
**Evidência:** \`RecallStudio.svelte:112-122,130-139\` atualiza arrays ao receber \`notes:changed\`, mas os estados do editor selecionado (\`noteTitle\`, \`noteBody\`, \`noteRevision\`) só são atualizados em \`openEntry\` (\`244-278\`). O cartão emite \`notes:changed\` após salvar.

**Reprodução prevista:** abrir uma nota no Arquivo, abrir seu cartão flutuante e alterar o texto no cartão. O Arquivo pode manter o texto antigo até trocar o item. Se escrever na versão antiga, o CAS corretamente aponta conflito, mas a tela poderia ter atualizado automaticamente se não estava suja.  
**Correção:** sincronizar o editor apenas quando **não** houver alterações locais; com alterações locais, apresentar aviso de mudança remota sem sobrescrevê-las.

### A6-05 — P2 · Lotes de lembretes podem ser registrados sem alerta nativo
**Evidência:** \`src-tauri/src/lib.rs:349-376\`. O agendador chama \`claim_due\` e muda os itens para \`attention\` **antes** de verificar o bloqueio de resumos (\`last_bulk_toast_at\`). Se outro lote antigo/maior chegar dentro de uma hora, o \`continue\` omite o toast e não grava nem a tentativa de despacho.

**Reprodução proposta:** acumular mais de 150 avisos antigos, abrir o RECALL e observar lotes consecutivos de 150. O primeiro pode gerar resumo, outros podem entrar na atenção sem aviso do Windows durante a janela de supressão.  
**Importante:** os lembretes permanecem **visíveis no aplicativo**, portanto não há evidência de perda do registro. O defeito é de entrega/observabilidade da notificação.  
**Correção:** marcar explicitamente \`suppressed\` no histórico, calcular resumo que inclua toda a fila ou agregar por janela; permitir que lotes novos importantes continuem sinalizados.

### A6-06 — P2 · Fixar nota não mantém sua prioridade no Arquivo
**Evidência:** \`RecallStudio.svelte:617-622\` chama \`notesApi.pin\`, mas \`src/lib/workspace.ts:8-35\` ordena tudo apenas por \`updatedAt\`, ignorando \`note.pinned\`. Alterar o pin também muda \`updated_at\` no backend (\`db.rs:214-218\`), fazendo a nota parecer fixada temporariamente.

**Reprodução prevista:** fixar nota A, depois editar nota B. B sobe acima de A; A não tem posição fixa.  
**Correção:** ranking estável dos fixados no topo, respeitando busca/filtros; adicionar teste de ordenação após outras alterações.

### A6-07 — P2 · Filtro Hoje/Depois inclui tarefas concluídas
**Evidência:** \`RecallStudio.svelte:86-89\` filtra o período, mas não \`completed_at\`. Os contadores laterais usam \`tasksByPeriod\` (\`src/lib/workspace.ts:71-74\`), que **exclui** concluídas.  
**Reprodução prevista:** concluir uma tarefa planejada para Hoje; abrir navegação Hoje. O contador pode indicar zero, mas a lista ainda mostra a concluída.  
**Correção:** escolher explicitamente se os filtros mostram só tarefas abertas; caso mostrem concluídas, refletir isso no contador e oferecer filtro separado.

### A6-08 — P2 · Conversão de ideia mantém o filtro antigo e oculta o item recém-criado
**Evidência:** \`RecallStudio.svelte:569-584\` converte ideia em nota/tarefa e abre o novo item, mas não limpa \`archiveFilter\`/busca. \`visibleEntries\` (\`86-89\`) pode continuar limitado à categoria Ideias.  
**Reprodução prevista:** filtrar Ideias, selecionar uma e clicar em Transformar em nota. O editor abre a nota, enquanto ela não aparece na lista filtrada.  
**Correção:** abrir com filtro apropriado ou limpar filtros e busca após conversão.

### A6-09 — P2 · Criar nota com busca ativa pode deixá-la invisível na lista
**Evidência:** \`RecallStudio.svelte:603-614\` limpa filtros de categoria e período ao criar, mas não \`search\`. \`searchWorkspace\` em \`src/lib/workspace.ts:51-60\` mantém o texto de busca, que pode não corresponder à nota em branco.  
**Reprodução prevista:** pesquisar por um termo sem correspondência, clicar em Nova nota. O inspector abre a nova nota, mas a lista esquerda continua vazia.  
**Correção:** limpar a busca ao criar ou acrescentar a nota recém-criada fora do filtro, com comportamento visível ao usuário.

### A6-10 — P2 · Requisições simultâneas de refresh podem exibir dados antigos
**Evidência:** \`RecallStudio.svelte:112-139\`. Vários eventos \`notes:changed\`, \`tasks:changed\` etc. iniciam \`refresh()\` sem controle de sequência, cancelamento nem descarte de respostas ultrapassadas.  
**Cenário de corrida:** refresh A começa com estado anterior, refresh B começa após uma mudança e termina primeiro; se A retornar depois, ele aplica o estado antigo por último.  
**Correção:** geração monotônica de requests ou fila coalescida, ignorando respostas obsoletas; teste com chamadas controladas que terminam fora de ordem. **Risco de corrida a confirmar**, não há reprodução de GUI nesta auditoria.

### A6-11 — P2 · Concluir tarefa e cancelar lembrete são operações separadas
**Evidência:** \`src-tauri/src/lib.rs:133-141\`. A tarefa é marcada como concluída em uma gravação SQLite e o lembrete é cancelado em outra; uma falha intermediária deixa os estados dessincronizados. \`reminders::list\` (\`reminders.rs:105-111\`) não filtra \`completed_at\`, embora \`claim_due\` exclua tarefa concluída.  
**Correção:** uma transação SQLite envolvendo tarefa e lembrete, com teste de rollback; proteger também a listagem de ocorrências concluídas. **Cenário de falha intermediária, não observado no Windows.**

### A6-12 — P2 · Prévia de conteúdo pessoal nas notificações Windows
**Evidência:** \`src-tauri/src/lib.rs:378-386\` usa até 150 caracteres do corpo da nota/detalhes da tarefa como texto do toast. Dependendo da configuração do Windows, esse texto pode aparecer na central de notificações ou na tela bloqueada.  
**Correção:** por padrão mostrar título genérico e usar preferências opt-in para prévias. **Risco de privacidade**, não exploração verificada nem confirmação de configuração específica do Windows.

## Robustez e manutenção

### A6-13 — P3 · Menu não pode ser reaberto em CSS de janela estreita
**Evidência:** \`src/recall.css:257\`. A media query até 680px fixa sidebar em largura 0 independentemente de \`re-collapsed\`; o botão de alternar permanece, mas não consegue exibir a barra.  
**Condição:** a janela Windows tem mínimo de 800px físicos, porém zoom do WebView, DPI ou embed de viewport menor pode atingir a faixa CSS.  
**Correção:** drawer responsivo com estado aberto/fechado e backdrop/teclado; teste de zoom 125–200%.

### A6-14 — P3 · Política Content Security Policy desativada
**Evidência:** \`src-tauri/tauri.conf.json:41\` declara \`"csp": null\`. Não foi demonstrada vulnerabilidade XSS no código atual (Svelte interpola conteúdo escapado), mas isso remove uma camada de defesa se algum conteúdo não confiável entrar em HTML no futuro. A auditoria da Alpha 4 já havia apontado essa dívida.  
**Correção:** ativar CSP apropriada às APIs Tauri, scripts/estilos e modo dev; verificar permissões do WebView e testar captura e cartão após habilitar.

## Testes e cobertura

A CI **não** executa testes end-to-end da GUI nem testes de crash/reinício para rascunhos de tarefa/ideia. Atualmente há testes TypeScript unitários de parsing, organização e busca, além de Rust/SQLite e Python para backup. A aprovação de build não demonstra persistência de edição, navegação após filtros nem privacidade das notificações.

**Matriz de regressão sugerida** (nenhuma linha foi executada no Windows por esta auditoria):

| Caso | Preparação / ação | Resultado esperado |
| --- | --- | --- |
| T01 | Digitar captura inline, sair sem enviar, abrir | Rascunho reaparece |
| T02 | Editar tarefa, encerrar processo antes de 900ms, abrir | Edição recuperável |
| T03 | Editar ideia, encerrar processo antes de 900ms, abrir | Edição recuperável |
| T04 | Abrir mesma tarefa/ideia em instâncias distintas e salvar versões concorrentes | Conflito detectado, nenhum texto sobrescrito |
| T05 | Editar cartão da nota enquanto a nota está aberta no Arquivo | Editor sincroniza quando limpo e preserva edição local quando sujo |
| T06 | Fixar nota A e depois editar nota B | A permanece entre fixados |
| T07 | Concluir tarefa Hoje; abrir filtro Hoje | Só tarefas ativas, contagem coerente |
| T08 | Filtrar Ideias; converter uma ideia em nota | Nota recém-criada visível e selecionada |
| T09 | Buscar termo inexistente; criar nota | Nova nota visível no Arquivo |
| T10 | Forçar refresh A/B com retorno fora de ordem | Resposta atrasada não reverte estado novo |
| T11 | Gerar >150 lembretes vencidos após pausa | Todos persistem e resumos são rastreáveis |
| T12 | Simular erro ao cancelar lembrete após concluir tarefa | Operação atômica ou estado consistente |
| T13 | Configurar Windows para mostrar prévias na tela bloqueada | Nenhum corpo de nota exposto sem opt-in |
| T14 | Zoom 150%; recolher e expandir menu | Navegação permanece acessível |

## Ordem recomendada para correções

**PR de integridade:** A6-01, A6-02, A6-03 e A6-11 — antes de novas funcionalidades; revisar dados/cópias e estender testes.  
**PR de frontend:** A6-04, A6-06, A6-07, A6-08, A6-09, A6-10 e A6-13 — manter a direção visual aprovada, corrigindo estados e filtros.  
**PR de notificações/segurança:** A6-05, A6-12 e A6-14 — não confundir notificação aceita pelo SO com alerta visto.

**Regra de segurança:** corrigir em novas branches derivadas da Alpha 6; testes e backups antes de promover. Não fazer merge na \`main\` sem autorização. A auditoria atual só registra diagnóstico; não altera o código.
