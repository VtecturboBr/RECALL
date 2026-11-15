# RECALL Alpha 11.2 — Integridade, concorrência e auditoria

**Branch:** `fix/v0.5-alpha11-2-integridade-carga`. PR draft baseado na Alpha 11.1. Sem merge na `main`.

## Achados da auditoria e correções

### 1. Leitura inconsistente entre WebViews (corrigido parcialmente, por detecção)

A Alpha 11.1 lia várias páginas separadas do SQLite. Um segundo WebView poderia alterar, incluir ou excluir cartões entre elas. CAS protege gravações, mas uma lista misturada poderia aparecer na interface.

- Criada API read-only `board_read_stamp`, baseada em contagem de cartões ativos, soma das revisões, rowids e tamanho em bytes do conteúdo.
- Antes da primeira página e depois da última, o frontend consulta essa assinatura no SQLite.
- Se a assinatura mudar ou a contagem final não corresponder à quantidade de cartões recebida, a abertura falha de maneira segura.
- Nenhum dado parcial é publicado; o usuário pode tentar novamente sem perder o estado anterior nem gravar resultados incompletos.
- Testes Rust verificam inserção, update de geometria/conteúdo, exclusão, quadros isolados e mutações entre páginas.
- Testes JavaScript verificam contadores inválidos, lote incompleto e mudanças que não alteram a quantidade total.

**Limite:** assinatura é um detector conservador e *best-effort*, não uma transação de leitura SQLite com snapshot único. Alterações simultâneas que resultem na mesma assinatura podem não ser observadas; o CAS segue sendo a barreira autoritativa para escritas. Não há garantia de snapshot transacional entre chamadas Tauri.

### 2. UI de falha/recuperação e operação incompleta

- Falhas interrompem a abertura e mantêm controles de edição bloqueados, com opção de recarregar.
- Também bloqueia criação de quadro enquanto dados estão sendo carregados.
- Mecanismo de rascunhos da Alpha 10.4 e histórico CAS da Alpha 10.5 mantidos, sem alteração destrutiva.

### 3. Desempenho mensurável

- `npm run bench:board` simula 50 mil cartões e 10 mil conexões, fazendo seleção por viewport e mesclagem de lotes de 120.
- O benchmark informa criação, seleção, mesclagem e variação do heap Node, **sem impor limite temporal em runner variável**.
- Esses números servem como linha de base sintética, não medem o WebView2, tempo de I/O SQLite real, RAM total ou FPS.
- A implementação ainda carrega o texto de todos os cartões em memória após a leitura completa.

## Verificações automatizadas

- `npm ci`, `npm run check`, `npm run test:unit`, `npm run bench:board`, `npm run build`.
- Testes Python de backup e restauração SQLite.
- `cargo check --locked` e `cargo test --locked --lib` em Windows.
- Instalador NSIS com verificação de arquivo e SHA-256.

## Pendências documentadas

- Teste manual com Windows/WebView2, foco do teclado, recuperação visual, desempenho real e acessibilidade.
- Teste de interrupção do processo, falha de energia e quota de localStorage em ambiente Windows real.
- Uma estratégia de snapshot transacional real ou paginação espacial via SQLite exigiria mais trabalho e testes em etapas posteriores.
- Sem nova migração: schema SQLite segue na v10. Nenhuma nova assinatura ou biblioteca paga foi introduzida.
- Qualquer instalação experimental deve ser feita sobre uma cópia/backup confirmado, nunca sobre a única cópia dos dados.

**Critério de conclusão:** duas execuções CI Windows e instalador com status `success`. Até lá, PR segue draft.