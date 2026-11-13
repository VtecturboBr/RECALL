# RECALL 0.5.0-alpha.8.1 — Teste integrado no Windows

**Esta versão reúne a migração da Alpha 8.1 e as correções do editor da Alpha 7.1.** Não é preciso instalar nem testar outra Alpha 7 antes dela.

## Incluído no mesmo instalador

- Migração local SQLite para `PRAGMA user_version=5`.
- Backup SQLite pré-migração `pre-alpha8` criado e verificado antes da gravação do novo esquema.
- Histórico de conteúdo `content_history` para notas/apresentações, tarefas, ideias e pontos de retomada. **Ainda não há interface de restauração de versões**.
- Separação transacional das novas ideias descartadas das convertidas em `inbox_dispositions`.
- Correção da ideia presa no Arquivo por texto temporariamente vazio; o texto válido já salvo continua intacto e o usuário pode trocar de item.
- Correção da restauração de cache vazio da ideia, que antes encobria o texto salvo.
- Exclusão lógica de notas e tarefas, atômica com cancelamento dos lembretes, sem bloquear por rascunho inválido. A **lixeira visual só virá na Alpha 8.2**.
- Nova tarefa sem título recebe o nome provisório `Nova tarefa`.

## Pré-requisito obrigatório — proteger dados pessoais

**Não começar sem uma cópia integral dos dados.**

1. Fechar o RECALL; também encerrar a instância da bandeja do Windows (ícone perto do relógio).
2. Pressionar **Win + R** e abrir `%APPDATA%\app.memoriaexterna.desktop`. Esse é o diretório obtido no Tauri por `app.path().app_data_dir()` com o identificador atual.
3. Copiar a pasta inteira, incluindo `memoria.sqlite`, possíveis `-wal` / `-shm` e `backups`, para um local separado, com data no nome. **Não cortar, mover nem substituir o original**.
4. Para um teste isolado, preferir outro usuário do Windows ou uma VM com **uma cópia** desses dados no AppData da conta de teste. O instalador usa o mesmo identificador de aplicativo do RECALL já instalado; se executado na conta habitual, o primeiro início migrará o banco real para v5.
5. Não abrir uma versão anterior do RECALL sobre um banco já migrado para v5. O rollback exige restaurar a cópia íntegra do banco, sempre com o programa fechado.

Se o AppData original não existir, esta é uma primeira instalação; ainda assim vale manter uma cópia de segurança após capturar conteúdo importante.

## Instalar

Abrir o workflow **Gerar instalador RECALL Windows** da branch `feature/v0.5-alpha8-migracao` no GitHub Actions. Após execução marcada **Success**, baixar o artefato:

`RECALL-v0.5-alpha8-1-integrado-Windows-x64`

Extrair o ZIP, verificar que há um `*-setup.exe` e um `SHA256SUMS.txt`, executar o instalador NSIS. Não confundir com ZIPs antigos de Alpha 7/7.1.

## Roteiro de testes (um único ciclo)

1. **Migrar e preservar:** abrir a Alpha 8.1 com **cópia do banco Alpha 7**. Verificar que notas, tarefas, ideias, apresentações e "Onde parei?" continuam visíveis e com conteúdo.
2. **Recuperar a ideia que travava:** abrir a ideia original da captura de tela; confirmar que o texto não aparece em branco se houver registro salvo no SQLite. Apagar todo o texto *temporariamente* e clicar em outra nota. Deve navegar, sem salvar o vazio nem destruir o texto original.
3. **Excluir item:** selecionar uma nota de teste, adicionar edição inválida/temporária quando aplicável e clicar em Excluir. Confirmar. Ela desaparece da lista, sem bloquear o usuário; **a interface de lixeira ainda não existe**, portanto use um item de teste descartável.
4. **Tarefa sem título:** selecionar Tarefa na captura com campo vazio e confirmar. Deve criar `Nova tarefa`. Também testar o botão `Nova tarefa` no Arquivo e depois apagar o título de tarefa de teste; não deve ficar preso na edição.
5. **Persistência:** fechar completamente e abrir novamente; conferir o texto de outras notas, a ideia original, a tarefa recém-criada, planos e pontos de retomada.
6. **Lembretes:** agendar lembrete de teste, excluir a tarefa e confirmar que não chega um lembrete de um item excluído.
7. **Apresentações:** retomar cartão anterior e confirmar progresso.
8. **SQLite:** verificar no banco de teste `PRAGMA user_version=5` e a existência de `content_history` e `inbox_dispositions`. Backup automático `recall-pre-alpha8-*.sqlite` deve constar em `backups`; ao reabrir não deve gerar outro `pre-alpha8`.

**Critérios de aprovação manual:** todos os itens acima sem perda de dados, sem interface travada e sem erro de SQL. Ainda exigimos CI aprovada e instalador NSIS com checksum.

## Resultado esperado do esforço

**Um único instalador para testar Alpha 8.1 e os consertos da Alpha 7.1**. Não haverá novo teste separado da Alpha 7 antes de seguir para Alpha 8.2.

## Pendências fora do escopo

- Lixeira visual e restaurar item excluído: Alpha 8.2.
- Navegar e restaurar versões pelo histórico: Alpha 8.3.
- Pesquisa aprimorada e estabilização: Alpha 8.4.
- Nenhum merge automático na `main`.
