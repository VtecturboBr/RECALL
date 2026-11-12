# AGENTS.md — RECALL

Estas orientações representam o workflow escolhido para este repositório. Respeite sempre as instruções da plataforma e as permissões disponíveis.

## Ao iniciar uma tarefa
1. Leia docs/AI_WORKFLOW.md e docs/AI_CHECKPOINT.md.
2. Confirme a branch, o estado do código e os testes realmente executados. Para funcionalidades da v0.5 Alpha 4, procure a branch feature/v0.5-alpha4-captura-tempo, e não deduza que a main já tenha essas mudanças.
3. Leia docs/AI_SKILLS.md. Selecione apenas as skills pertinentes à tarefa; um link no catálogo não significa skill instalada ou carregada.
4. Consulte arquivos reais antes de modificar a implementação. Use os comandos /status, /planejar, /debug, /refinar e /checkpoint como atalhos de conversa, conforme docs/AI_WORKFLOW.md.

## Decisões de colaboração
- Idioma: português brasileiro, direto, com resultados prontos e resumo curto.
- Autonomia: decida detalhes reversíveis; pergunte somente por decisões essenciais, riscos materiais, permissões ou mudanças de direção.
- Qualidade: máxima; até 3 ciclos de avaliação e refinamento por entrega.
- Design: compare três direções distintas antes de um redesign significativo; respeite a direção escolhida.
- Ferramentas: gratuitas e locais permitidas. Pode apresentar opções pagas, mas nunca contratar, gastar, criar contas ou ativar serviços sem autorização.
- Aplicar seleção automática de grupos de skills conforme objetivo; não tentar usar React em Svelte nem obrigar todo o catálogo em cada tarefa.

## Produto RECALL
- Missão: memória externa que permite capturar, lembrar, focar e retomar, com pouca carga cognitiva.
- Plataforma: Windows primeiro; Tauri 2, Rust, Svelte 5, TypeScript, SQLite; offline-first.
- Interface atual de desenvolvimento: Agora e Arquivo; identidade escura com vermelho como acento, sem excesso de menus.
- Preserve notas, tarefas, lembretes, atalhos e compatibilidade com dados antigos. Migrações requerem plano reversível e backup.
- Não interprete prazo de tarefa na Alpha 4 como notificação ativa: lembretes recorrentes são desenvolvimento futuro.

## Concluir e registrar
1. Defina critérios verificáveis; implemente a menor mudança coerente; revise/compare até três ciclos.
2. Execute, conforme aplicável, testes Svelte/TypeScript, testes unitários Node, Rust/cargo e compilação Windows no GitHub Actions.
3. Não confunda CI aprovado com instalação, acessibilidade ou comportamento validado manualmente num Windows real.
4. Não faça merge, publique releases estáveis, apague dados ou altere contas sem autorização apropriada.
5. Atualize docs/AI_CHECKPOINT.md quando mudar estado, decisões, testes ou próximo passo. Indique caminhos, PRs e limitações reais.
6. Exiba resultado e resumo curto, sem reproduzir uma cronologia extensa de chamadas de ferramenta.

Nota: uma conversa diferente precisa receber ou acessar estes arquivos. Eles não criam memória automática entre chats nem garantem execução de skills não instaladas.
