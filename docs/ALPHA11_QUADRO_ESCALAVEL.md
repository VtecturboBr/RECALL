# RECALL Alpha 11.0 — Escalabilidade visual do Quadro

**Estado:** recurso experimental em branch separada e PR draft. Nenhum merge na `main`.

## Objetivo

Reduzir o custo de interface do Quadro Infinito com centenas/milhares de cartões e imagens,
sem trocar o modelo de dados, sem migração SQLite e sem arriscar arquivos existentes.

## O que foi implementado

- Filtro espacial de cartões com sobredesenho de 280 pixels convertidos em coordenadas do quadro.
- Cartões ficam carregados na coleção em memória e no SQLite: apenas os componentes HTML fora da área são desmontados.
- Mantém também o cartão selecionado mesmo quando não está na região visível.
- Setas limitadas às que cruzam a viewport com algoritmo geométrico Liang–Barsky.
- Eliminada a busca repetida `items.find` por extremidade de aresta, substituída por índice Map.
- Minimap com Canvas de alta resolução 2×, renderizado por animação de quadro via requestAnimationFrame.
- Clique no minimapa reposiciona a câmera; ativação por teclado enquadra o quadro.
- Carregamento lazy de data URLs de imagem somente para cartões visíveis + margem.
- Cache visual limitada a 20 imagens simultâneas e no máximo três requisições de imagem em voo.
- Ao deslocar o quadro, URLs de imagem de cartões fora da tela são removidas só da memória da UI, não do SQLite.
- Ao trocar de quadro, respostas atrasadas de imagens são ignoradas por identificador de requisição.
- Testes Node em `tests/boardPerformance.test.mjs` usam 5.000 cartões artificiais e situações-limite.

## Pontos preservados

- SQLite schema 10, exclusão lógica, backups, revisão CAS e tratamento de conflitos.
- Histórico Desfazer/Refazer limitado à sessão (Alpha 10.5).
- Referências dinâmicas a notas/tarefas e conexões persistidas entre cartões.
- Recuperação de rascunhos de outras janelas (Alpha 10.4).
- Nenhuma dependência adicional npm ou chamada de rede externa.

## Critérios automáticos

1. `npm ci`, `npm run check`, `npm run test:unit`, `npm run build`.
2. Teste de restaurador SQLite offline, `cargo check`, `cargo test` e instalador NSIS Windows com SHA256.
3. Provar que elementos completamente fora da viewport não entram na lista renderizada.
4. Não cortar cartões visíveis com zoom, coordenadas negativas ou margem de segurança.
5. Não deixar de desenhar seta que cruza a tela quando ambas as pontas estão fora dela.
6. Limitar carregamentos de imagens simultâneos e limitar cache, sem excluir o asset original.
7. Continuar preservando textos, IDs, drafts, referências, revisões e estados de conclusão.

## Limitações e riscos

- SQLite continua consultando os metadados de todos os cartões ao abrir o Quadro. Paginação e índice espacial no banco são futuros.
- A filtragem de visibilidade é uma varredura O(n) em memória e Canvas desenha todas as miniaturas; não promete 60 FPS em qualquer máquina.
- Se mais de 20 imagens estiverem simultaneamente visíveis, parte aguardará até a câmera se mover e liberar o cache.
- O Canvas não fornece foco individual por marcador; navegue pelo botão de busca para selecionar cartões por texto.
- Data URL grande ainda ocupa memória enquanto visível; não há geração automática de miniaturas por resolução.
- Testes manuais de desempenho, WebView2, atalhos e acessibilidade continuam pendentes até haver disponibilidade do usuário.
- Não instalar build experimental em cima da única cópia real dos dados e não realizar merge na main.
