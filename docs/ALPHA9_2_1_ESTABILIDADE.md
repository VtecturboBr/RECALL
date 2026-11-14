# RECALL Alpha 9.2.1 — estabilidade e integridade

**Branch**: `fix/alpha9-2-1-estabilidade-integridade`  
**Base**: `feature/v0.5-alpha9-2-revisao-diaria` (Alpha 9.2 / PR #20)  
**Versão**: `0.5.0-alpha.9.2.1`  
**Status**: prévia experimental, sem merge na `main` e sem validação manual do Windows.

## Correções incluídas

- **F01** Captura Rápida: o conteúdo digitado durante um salvamento é preservado. Só limpamos texto e cache se ambos continuarem iguais ao snapshot submetido.
- **F02** Modo Foco: pulso nativo Rust de 12 segundos continua ativo fora da tela Agora. O limite de 20 s por pulso previne horas falsas após sleep/crash.
- **F03** Controle global flutuante de foco em todas as telas: abrir tarefa, pausar ou finalizar uma sessão, inclusive para tarefas mudadas para Depois/Algum dia.
- **F04** Resumo em edição da sessão ativa é gravado **na mesma transação** do encerramento de uma tarefa (conclusão ou exclusão). O cache de resumo só é apagado depois de confirmar sucesso.
- **F05** Restauração offline: cópia anterior sempre verificada; alteração de banco executada com **uma substituição atômica**, eliminando intervalo sem `memoria.sqlite`. Se versões antigas deixaram arquivo `.recall-original-*.sqlite` e não há banco principal, o aplicativo falha fechado em vez de criar um arquivo vazio.
- **F06/F13** Relacionamentos agora são resolvidos por uma **única consulta SQL** que remove destinos processados/excluídos antes do `LIMIT 50` (sem consultas N+1).
- **F07** A Revisão Diária recebe chaves de sugestões ocultadas e as filtra no backend antes de limitar a 12, revelando itens restantes em vez de exibir falsamente lista vazia.
- **F08** Backups manuais são possíveis mesmo em caso de conflito de edição. A interface avisa de forma explícita que rascunhos locais não confirmados **não fazem parte do backup SQLite**.
- **F10/F17** Tarefas sem nome são normalizadas para `Nova tarefa` na captura rápida e também na opção de salvar cópia de uma tarefa com conflito.
- **F11** Ao converter ideia em nota ou tarefa, os relacionamentos são duplicados para a entidade resultante, na mesma transação, mantendo o histórico da ideia.
- **F13** Revisão Diária lê um sinal booleano sobre detalhes da tarefa em vez de transferir 100 mil caracteres por linha.
- **F15** Resolução de npm/Cargo registrada em lockfiles; workflows usam instalação e testes bloqueados nas versões registradas.
- **F16** Restauração valida schema mínimo, `user_version` aceitável e `foreign_key_check`, rejeitando bases de outro produto ou versões novas.

## Testes acrescentados

- 70 relacionamentos: os 60 mais recentes foram excluídos; 10 antigos válidos continuam visíveis.
- 20 destaques na revisão: ocultar os primeiros 12 revela outros oito.
- Conversão de ideias associadas em notas e tarefas: contexto duplicado sem apagar vínculo histórico.
- Conclusão e exclusão de tarefa durante sessão ativa: resumo de progresso persistido no SQLite.
- Pausa após mover tarefa para Algum dia.
- Pulso após simulação de suspensão longa: incremento limitado a 20 segundos.
- Restauração: SQLite estruturalmente inválido, versão futura e falha simulada na substituição deixam a base original intacta.

## Riscos ainda pendentes (não prometer estabilidade)

- **F09:** o cache de rascunhos de múltiplos editores equivalentes ainda usa chaves por entidade. A aplicação principal atual é única, mas uma futura arquitetura multi-janela exige namespace por instância e recuperação de órfãos.
- **F12:** histórico, backups e snapshots legados ainda podem crescer; não foi implementada exclusão automática, que poderia destruir dados de recuperação. Requer política explícita e opção do usuário.
- **F13:** busca global e Arquivo ainda carregam documentos completos; paginação exige alterar contratos frontend/backend e testes de busca sem regressão.
- **F14:** testes de componentes e E2E WebView2 continuam pendentes; CI Rust/Node/Python não substitui teste visual no Windows.
- **F15 (parcial):** `package-lock.json` e `src-tauri/Cargo.lock` foram criados no runner Windows; CI usa `npm ci` e `cargo --locked`. **Resta** fixar as GitHub Actions por SHA revisada e fazer verificação de supply chain. O build ainda não é bit-a-bit reproduzível.
- **F18:** código legado de telas e CSS ainda permanece para evitar excluir dependências visuais sem captura/smoke test. README antigo necessita atualização completa.

## Roteiro único de validação Windows

1. Fechar completamente o RECALL pela bandeja. Fazer **cópia integral** da pasta `%APPDATA%\app.memoriaexterna.desktop` (incluindo WAL/SHM, se presentes). Preferir usuário Windows isolado com uma cópia dos dados.
2. Baixar o artefato `RECALL-v0.5-alpha9-2-1-estabilidade-Windows-x64` da branch somente **depois** de Success no GitHub Actions. Extrair e conferir SHA256SUMS.
3. Captura Rápida: salvar ideia e digitar mais texto durante uma escrita lenta; o texto novo deve continuar no campo e após reabrir.
4. Criar tarefa vazia na Captura Rápida e confirmar título `Nova tarefa`.
5. Iniciar sessão de foco; ir ao Arquivo e trabalhar por 3 minutos; ao retornar, confirmar tempo registrado. Simular suspensão e verificar que não foram contadas horas falsas.
6. Mover tarefa com sessão de foco para Algum dia; a barra global deve oferecer Pausar/Finalizar em qualquer tela.
7. Digitar resumo, marcar tarefa como concluída sem finalizar o foco previamente; verificar registro de resumo no histórico.
8. Criar 70 vínculos de teste, excluir 60 destinos recentes: os 10 válidos devem aparecer. Converter ideia associada e conferir vínculos na nova entidade.
9. Criar 20 pendências; ocultar 12 na Revisão Diária e verificar que as próximas são mostradas.
10. Editar uma nota em duas janelas até gerar conflito; criar backup e conferir aviso sobre rascunho não salvo.
11. Testar restauração **em uma cópia** e nunca sobre o único banco real. Validar arquivo inválido e versão nova, e confirmar que banco atual permanece intacto.
12. Confirmar notas, ideias, tarefas, lembretes, apresentação, links, foco, histórico e backups pré-existentes. Nenhuma migração SQLite extra além da v7.

**Nunca fazer merge automático na `main`.**
