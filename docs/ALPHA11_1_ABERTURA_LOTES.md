# RECALL Alpha 11.1 — Abertura por lotes e redução de overhead reativo

**Status:** implementação experimental em PR separado sobre Alpha 11.0; nenhuma mudança na `main`.

## Motivação

O backend anterior enviava todos os textos e metadados de um quadro em uma só chamada Tauri.
Em um quadro muito grande, isso criava um único payload IPC e trabalho de materialização de estado reativo proporcional ao tamanho completo da coleção.

## Mudanças implementadas

- Nova consulta Rust `whiteboard::list_page`: usa `(board_id,deleted_at,created_at,id)` e ordenação `(created_at,id)` sobre o índice v9 existente.
- Cursor composto por `created_at` e `id`, necessário quando muitos cartões têm timestamp idêntico.
- Limite validado no Rust de 1 até 200 cartões; frontend usa 120.
- Cada página retorna só os cartões do lote e um cursor opcional (null ao concluir).
- Validação frontend bloqueia cursor repetido, tamanho exagerado ou ordenação inesperada.
- O frontend acumula os lotes fora do estado visível, mostrando contador de progresso durante carregamento.
- A lista de cartões só substitui a anterior quando todas as páginas terminam com sucesso.
- A restauração de rascunhos, CAS, dados de referência e histórico ocorre após a leitura completa.
- `items`, `connections`, `references` e `assets` usam `$state.raw` em Svelte 5.
- Isso evita proxies reativos profundos e preserva os padrões imutáveis já existentes.
- A renderização por viewport e o cache de imagem da Alpha 11.0 continuam em vigor.

## Limitações e escolhas explícitas

- Todos os textos dos cartões ainda ficam em memória quando o carregamento termina. A otimização reduz o **pico de IPC** e a **sobrecarga de proxies reativos**, não implementa paginação espacial de conteúdo em memória.
- A abertura completa pode continuar demorada em quadros extremos; o novo contador mostra o progresso, mas só libera edição após sucesso de todas as páginas.
- Durante a leitura, outro WebView pode criar/editar/excluir cartões; a paginação por cursor impede duplicatas na ordenação observada, mas não é um snapshot SQLite transacional de múltiplas consultas.
- Edições concorrentes continuam protegidas por revisão CAS. Uma atualização remota feita após a leitura de um cartão pode precisar de recarga para aparecer.
- Não há atualização de schema SQLite: continua na versão 10; nenhum dado existente é removido.

## Validação

1. `npm ci`, `npm run check`, `npm run test:unit`, build do frontend.
2. Restauração de backup Python, `cargo check` e `cargo test` no Windows.
3. Testes Rust cobrem 47 cartões com o mesmo timestamp, exclusão lógica, isolamento entre quadros e lotes de 200.
4. Testes JS verificam 5.003 cartões, cursores repetidos, lotes vazios e limites de tamanho.
5. Confirmar no WebView que os dados não são substituídos por uma lista parcial se o IPC falhar.
6. Testar troca de quadros, reinício do app, texto, tabelas, imagens, referências e botões Desfazer/Refazer.
7. Confirmar que nenhuma edição é permitida antes do término da leitura, nem na presença de erro.
8. Validar acessibilidade do indicador de progresso e consumo de memória em máquina Windows real quando possível.

**Não instalar sobre a única cópia do banco.** O PR permanece draft; testes manuais estão pendentes por opção do usuário.
