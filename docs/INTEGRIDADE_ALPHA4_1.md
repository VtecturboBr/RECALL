# RECALL Alpha 4.1 — Proteção dos dados

**Escopo:** corrigir R01, R02 e a falta de backup da R04 da auditoria, sem refazer a interface nem mudar notas/tarefas antigas. A restauracão é offline e exige ação explícita. Baseado na Alpha 4.

## O que mudou

1. **Salvamento protegido por revisão.** SQLite mantém uma pequena tabela auxiliar \`note_revisions\` e gatilhos para inserções e mudanças no texto. \`save_note\` compara a revisão lida pelo editor com a revisão atual. Se outras janelas alteraram a nota, o banco recusa a atualização obsoleta em uma única operação SQL, sem sobrescrever ninguém.
2. **Rascunho recuperável.** Edição na janela principal e nos cartões guarda cópia temporária em \`localStorage\` do próprio aplicativo. Em caso de erro de gravação ou reinício, a versão não salva é recuperada na próxima abertura do item. Em conflito, há botão para criar nova nota com uma única inserção atômica.
3. **Cartão flutuante.** Fechar pelo controle próprio exige sucesso do salvamento; no erro a janela permanece aberta. Windows Alt+F4/outro evento de fechamento oculta a janela, mantendo sua instância e o rascunho durante a sessão. A cópia persistente em armazenamento local reduz riscos quando o aplicativo encerra.
4. **Backups SQLite consistentes em WAL.** Um botão com ícone de escudo na barra principal executa \`VACUUM INTO\`. O arquivo é gravado primeiro como \`.partial\`, validado com \`PRAGMA quick_check\` e só então renomeado para \`.sqlite\`. Seu caminho completo é exibido no aplicativo. O usuário pode guardar cópias adicionais fora da pasta do aplicativo.
5. **Snapshot antes da migração.** Ao abrir um banco preexistente sem a tabela \`note_revisions\`, o RECALL cria e valida um backup \`recall-pre-integrity-*.sqlite\` antes de criar a tabela, os gatilhos e registrar \`PRAGMA user_version = 1\`. Se o backup falhar, a nova migração não prossegue.
6. **Restauração offline com rollback.** O script Python \`scripts/restore_backup.py\` verifica o backup, impede restaurar enquanto o processo do RECALL está aberto, recusa arquivos WAL/SHM residuais, cria uma cópia verificada do estado atual e troca o banco via renomeações dentro da mesma pasta, com tentativa de rollback se ocorrer erro.

### Backup no próprio aplicativo

Abra RECALL e clique no **ícone de escudo** no topo. O aplicativo mostrará \`Backup verificado: C:\\...\\backups\\recall-manual-...sqlite\`. Esse é um snapshot SQLite único e consistente, incluindo mudanças comprometidas no WAL. Para proteção contra perda de disco, copie esse arquivo para outro lugar que você controle.

**Não confunda** o backup com sincronização automática na nuvem. Ele é local; o programa não faz backup periódico automático de todas as alterações.

### Como restaurar com segurança

1. Localize o backup \`recall-*.sqlite\` e tenha acesso ao código do projeto (pasta \`scripts\`).
2. Feche completamente o RECALL pelo ícone na bandeja, opção **Sair do aplicativo**. Fechar só a janela principal não é suficiente.
3. Instale Python 3.10+ gratuito se não estiver disponível.
4. No terminal, a partir da raiz do repositório, execute **prévia sem alterações**:

   \`\`\`powershell
   python scripts/restore_backup.py "C:\\caminho\\backups\\recall-manual-ARQUIVO.sqlite"
   \`\`\`

5. Confira os caminhos e contagens de notas. Para confirmar a restauração:

   \`\`\`powershell
   python scripts/restore_backup.py "C:\\caminho\\backups\\recall-manual-ARQUIVO.sqlite" --confirm
   \`\`\`

Se o arquivo foi movido para outra pasta, acrescente \`--db "C:\\caminho\\do\\memoria.sqlite"\` para identificar o banco ativo. **Nunca apague os arquivos SQLite \`-wal\` ou \`-shm\` por conta própria.** Se existirem após fechar tudo, o restaurador não prossegue, preservando os dados.

Após confirmação, uma cópia da situação anterior permanecerá em \`backups/recall-pre-restore-...sqlite\`. Reinicie o RECALL e confira os dados.

**Limitações:** ainda não há restauração com um clique dentro do aplicativo. O Python é exigido somente para a restauração offline, não para usar o RECALL ou criar backups. O mecanismo depende de o programa estar fechado e não substitui backups externos independentes. A validação em Windows real segue pendente.

## Testes de código previstos

- \`cargo test --lib\`: banco legado com notas, snapshot pré-migração, CAS concorrente, gatilho para edição de código antigo, integridade de backup e dados comprometidos no WAL.
- \`python -m unittest discover -s scripts -p "test_*.py"\`: restauração com cópia de segurança, rejeição de backup corrompido e bloqueio com indicador WAL.
- \`npm run check && npm run test:unit && npm run build\`: interface Svelte/TypeScript, testes existentes de datas e busca.
- Instalação Windows x64 com NSIS em GitHub Actions.

## Testes manuais obrigatórios antes de promover

- Abrir uma nota em Arquivo e cartão flutuante, editar nas duas janelas e confirmar que uma edição antiga não sobrescreve silenciosamente a mais recente.
- Fazer o editor falhar (ex.: desconectar/gravar conteúdo inválido) e tentar fechar o cartão; ele deve manter o texto editável.
- Fechar e reabrir o cartão com rascunho não salvo; confirmar recuperação.
- Criar backup com o ícone de escudo e validar no computador o caminho do arquivo.
- Restaurar um backup de **banco de teste** com o RECALL fechado, conferindo contagens e cópia pré-restauração.
- Abrir uma base de Alpha 4 com dados reais **após criar uma cópia independente**, verificar notas, tarefas, ideias, lembretes, captura e apresentação.
- Verificar uso com bandeja, Ctrl+Shift+Espaço e cartões flutuantes.

O código é experimental. **Não execute a restauração sobre dados reais sem manter uma cópia externa da pasta original.**
