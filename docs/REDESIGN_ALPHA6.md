# RECALL v0.5 Alpha 6 — Redesign total do frontend

**Branch:** \`feature/v0.5-alpha6-frontend-redesign\`  
**Base:** \`feature/v0.5-alpha5-lembretes2\`, PR #7, sem merge.  
**Escopo:** substituição do frontend visual, preservando Tauri/Rust/SQLite e os comandos de dados. Alpha experimental até validação real.

## Objetivo

Redesenhar completamente a experiência sem transformar RECALL em um dashboard. A tarefa principal é **capturar sem atrito, focar no que importa, localizar qualquer informação e retomar**. O aplicativo continua offline-first, Windows-first, com dados locais e identidade escura com o vermelho \`#E90530\`.

## Três direções comparadas

| Direção | Características | Decisão |
| --- | --- | --- |
| **A. Editorial minimalista** | Navegação leve, tipografia legível, superfícies discretas, grande ênfase no foco atual, quase nenhuma decoração | **Escolhida** — reduz carga cognitiva e preserva identidade já definida |
| B. Central operacional | Mais métricas, várias informações simultâneas, telas densas e painéis de controle | Rejeitada: densidade desnecessária para memória externa |
| C. Workspace de cartões | Grades visuais, cartões coloridos e arranjos livres | Rejeitada como tela principal; preservamos cartões flutuantes opcionais |

Esta decisão usa as preferências de workflow (IA escolhe detalhes e pergunta só o essencial), não implica que o usuário tenha aprovado uma renderização visual no Windows.

## Estrutura construída

1. **Shell global:** novo menu lateral compacto, navegação Agora/Arquivo, atalhos para tarefas Hoje/Depois/Algum dia, barra superior consistente, backup e captura.
2. **Agora:** introdução reduzida, tarefa em foco com conclusão direta, próximas tarefas, captura inline, lembretes pendentes/próximos e histórico.
3. **Arquivo:** coleção pesquisável com filtros por tipo e por período real, preview e inspector com edição contextual por nota, ideia, tarefa ou apresentação.
4. **Editores:** tipografia e campos revisados, texto como área principal, seções de planejamento e lembretes explícitas, exclusão no menu secundário.
5. **Janelas menores:** captura global e cartão flutuante redesenhados com a mesma linguagem visual; rascunhos, estado de salvamento e conflito preservados.
6. **Apresentação:** modo tela cheia redesenhado sem modificar a separação de slides por \`---\`.
7. **Acessibilidade:** labels, foco visível, contraste mais cuidadoso, redução de movimento e layout adaptável ao tamanho disponível.

### Arquitetura de frontend

- \`src/components/RecallStudio.svelte\`: nova UI principal — mantém os mesmos comandos/handlers e lógica de persistência da Alpha 5, sem migrar registros.
- \`src/components/QuickCapture.svelte\`: mesma captura offline com novos controles visuais.
- \`src/components/Sticky.svelte\`: mesmos salvamentos CAS, cache de rascunho e ação de cópia em conflitos.
- \`src/recall.css\`: tokens e interface principal.
- \`src/capture-redesign.css\`: janela de captura.
- \`src/sticky-redesign.css\`: cartão flutuante.
- \`src/main.ts\`: importa **apenas** as novas folhas de estilo; estilos anteriores continuam no repositório como referência, mas não são carregados.
- \`src/App.svelte\`: nova interface ativa sem alterar a seleção por parâmetros \`?capture=1\` e \`?sticky=ID\`.

## Compatibilidade: não alterar os dados

Sem alterações no banco, no identificador Tauri, nas funções Rust ou no IPC. Preservar:
- Notas e apresentações, autosave, revisão e cópia de conflito.
- Tarefas, Hoje/Depois/Algum dia, datas planejadas e conclusão.
- Ideias da inbox, edição e conversão em notas e tarefas.
- Captura rápida por Ctrl+Shift+Espaço e Ctrl+Enter, rascunho local.
- Lembretes Alpha 5: agendamento, recorrência, adiamento, resolução, histórico.
- Backup manual, cartões flutuantes e modo apresentação.
- Pesquisar com Ctrl+K, abrir captura com Ctrl+N.

## Critérios de qualidade e testes

**Automatizados:** \`npm run check\`, \`npm run test:unit\`, \`npm run build\`, \`cargo check\`, \`cargo test --lib\`, \`python -m unittest discover -s scripts -p "test_*.py"\`, Windows NSIS.

**Manuais indispensáveis antes de estabilidade:**
1. Usar Agora/Arquivo em 800×560, 1160×770 e janela expandida, sem controles cortados.
2. Ctrl+K abre pesquisa com foco; Ctrl+N/Ctrl+Shift+Espaço abrem captura.
3. Criar, editar, converter, planejar, completar e excluir em banco de teste.
4. Editar nota no inspector e no cartão simultaneamente; detectar conflito e salvar cópia sem perda.
5. Criar backup, confirmar arquivo; não testar restauração diretamente em dados reais.
6. Agendar lembrete de nota/tarefa, adiar, resolver, repetir e revisar histórico.
7. Apresentação com teclado, zoom e redução de movimento.
8. Confirmar que fonte, ícones e CSS funcionam totalmente sem rede.
9. Verificar compatibilidade dos registros da Alpha 4.1/5 (com cópia independente).

**Limitações:** screenshots reais do executável Windows e navegação com leitor de tela não são verificados automaticamente por CI. Só considerar essa revisão visual concluída após usar o aplicativo em Windows real.

## Fluxo de revisão

- Ciclo 1: implementar shell, Agora, Arquivo e estilos novos.
- Ciclo 2: completar captura, cartão, apresentações e controles preservados.
- Ciclo 3: corrigir erros descobertos pela CI e registrar pendências de validação Windows.

Manter PR em rascunho. Não fazer merge na main sem autorização explícita.
